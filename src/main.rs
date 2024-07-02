#![no_std]
#![no_main]

use core::cell::RefCell;

use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{Event, Gpio25, Gpio27, Input, Io, Level, Output, Pull},
    peripherals::{Peripherals, TIMG0},
    prelude::*,
    rmt::Channel,
    rng::Rng,
    system::SystemControl,
    time::current_time,
    timer::timg::{Timer, TimerGroup, TimerInterrupts, TimerX},
    Blocking,
};

use critical_section::Mutex;
use embedded_io::*;
use esp_backtrace as _;
use esp_wifi::{
    current_millis, initialize,
    wifi::{
        utils::create_network_interface, AccessPointConfiguration, Configuration, WifiApDevice,
    },
    wifi_interface::WifiStack,
    EspWifiInitFor,
};
use fugit::{Instant, MicrosDurationU64};
use patterns::shooting_star::ShootingStar;
use smoltcp::iface::SocketStorage;
use transmit::send_data;
use util::color::Rgb;

mod patterns;
mod transmit;
mod util;

type TimerG0N<const N: u8> = Timer<TimerX<TIMG0, N>, Blocking>;

#[derive(Debug, Clone)]
struct TapInfo {
    last_time: Option<Instant<u64, 1, 1000000>>,
    interval: MicrosDurationU64,
    tap_series_count: u8,
    tap_series_start: Option<Instant<u64, 1, 1000000>>,
}

struct SharedItems<'a> {
    tap_info: Option<TapInfo>,
    button: Option<Input<'a, Gpio25>>,
    shoot_timer: Option<TimerG0N<0>>,
    render_timer: Option<TimerG0N<1>>,
    rmt_channel: Option<Channel<Blocking, 0>>,
    led: Option<Output<'a, Gpio27>>,
    rgbs: Option<ShootingStar>,
    rng: Option<Rng>,
    render_started: bool,
}
const N_LEDS: usize = 149;
const MAX_INTENSITY: u8 = 30;
const RENDER_INTERVAL: u64 = 10;

static SHARED: Mutex<RefCell<SharedItems>> = Mutex::new(RefCell::new(SharedItems {
    tap_info: None,
    button: None,
    shoot_timer: None,
    render_timer: None,
    rmt_channel: None,
    led: None,
    rgbs: None,
    rng: None,
    render_started: false,
}));

static LAST_SHOT: Mutex<RefCell<Option<Instant<u64, 1, 1000000>>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);

    let clocks = ClockControl::max(system.clock_control).freeze();

    esp_println::logger::init_logger_from_env();

    let mut io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    io.set_interrupt_handler(button_press_handler);

    let led = Output::new(io.pins.gpio27, Level::Low);
    let mut button = Input::new(io.pins.gpio25, Pull::Up);
    let delay = Delay::new(&clocks);

    let channel = transmit::init_rmt(peripherals.RMT, io.pins.gpio26, &clocks);

    let rng = Rng::new(peripherals.RNG);

    let timg1 = TimerGroup::new(peripherals.TIMG1, &clocks, None);
    let wifi_timer = timg1.timer0;

    // delay.delay(2.secs());
    let init = initialize(
        EspWifiInitFor::Wifi,
        wifi_timer,
        rng,
        peripherals.RADIO_CLK,
        &clocks,
    )
    .unwrap();

    let wifi = peripherals.WIFI;
    let mut socket_set_entries: [SocketStorage; 3] = Default::default();
    let (iface, device, mut controller, sockets) =
        create_network_interface(&init, wifi, WifiApDevice, &mut socket_set_entries).unwrap();

    let mut wifi_stack = WifiStack::new(iface, device, sockets, current_millis);

    let client_config = Configuration::AccessPoint(AccessPointConfiguration {
        ssid: "esp-wifi".try_into().unwrap(),
        ..Default::default()
    });
    let res = controller.set_configuration(&client_config);
    log::info!("wifi_set_configuration returned {:?}", res);

    controller.start().unwrap();
    log::info!("is wifi started: {:?}", controller.is_started());

    log::info!("{:?}", controller.get_capabilities());

    wifi_stack
        .set_iface_configuration(&esp_wifi::wifi::ipv4::Configuration::Client(
            esp_wifi::wifi::ipv4::ClientConfiguration::Fixed(
                esp_wifi::wifi::ipv4::ClientSettings {
                    ip: esp_wifi::wifi::ipv4::Ipv4Addr::from(parse_ip("192.168.2.1")),
                    subnet: esp_wifi::wifi::ipv4::Subnet {
                        gateway: esp_wifi::wifi::ipv4::Ipv4Addr::from(parse_ip("192.168.2.1")),
                        mask: esp_wifi::wifi::ipv4::Mask(24),
                    },
                    dns: None,
                    secondary_dns: None,
                },
            ),
        ))
        .unwrap();

    log::info!("Start busy loop on main. Connect to the AP `esp-wifi` and point your browser to http://192.168.2.1:8080/");
    log::info!(
        "Use a static IP in the range 192.168.2.2 .. 192.168.2.255, use gateway 192.168.2.1"
    );

    let mut rx_buffer = [0u8; 1536];
    let mut tx_buffer = [0u8; 1536];
    let mut socket = wifi_stack.get_socket(&mut rx_buffer, &mut tx_buffer);

    socket.listen(8080).unwrap();
    log::info!("after socket listen");

    let handlers = TimerInterrupts {
        timer0_t0: Some(shoot_timer_handler),
        timer0_t1: Some(render_timer_handler),
        ..Default::default()
    };

    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks, Some(handlers));

    let rgbs = ShootingStar::new(400);

    let render_timer = timg0.timer1;
    render_timer.load_value(RENDER_INTERVAL.millis()).unwrap();
    render_timer.listen();

    let shoot_timer = timg0.timer0;
    shoot_timer.listen();
    log::info!("before cs");

    critical_section::with(|cs| {
        button.listen(Event::FallingEdge);
        let mut shared = SHARED.borrow_ref_mut(cs);
        shared.button.replace(button);
        shared.shoot_timer.replace(shoot_timer);
        shared.render_timer.replace(render_timer);
        shared.led.replace(led);
        shared.rgbs.replace(rgbs);
        shared.rng.replace(rng);
        shared.rmt_channel.replace(channel);
    });

    log::info!("before loop");

    loop {
        socket.work();

        if !socket.is_open() {
            socket.listen(8080).unwrap();
        }

        if socket.is_connected() {
            log::info!("Connected");

            let mut time_out = false;
            let wait_end = current_millis() + 20 * 1000;
            let mut buffer = [0u8; 1024];
            let mut pos = 0;
            loop {
                if let Ok(len) = socket.read(&mut buffer[pos..]) {
                    let to_print =
                        unsafe { core::str::from_utf8_unchecked(&buffer[..(pos + len)]) };

                    pos += len;
                    if to_print.contains("\r\n\r\n") {
                        log::info!("{}", to_print);
                        break;
                    }
                } else {
                    break;
                }

                if current_millis() > wait_end {
                    log::info!("Timeout");
                    time_out = true;
                    break;
                }
            }

            log::info!("buffer length: {}", pos);
            handle_http_request(unsafe { core::str::from_utf8_unchecked(&buffer[..pos]) });

            if !time_out {
                socket
                    .write_all(
                        b"HTTP/1.0 200 OK\r\n\r\n\
                    <html>\
                        <body>\
                            <h1>Hello Rust! Hello esp-wifi!</h1>\
                        </body>\
                    </html>\r\n\
                    ",
                    )
                    .unwrap();

                socket.flush().unwrap();
            }

            socket.close();

            log::info!("Done\n");
        }

        let wait_end = current_millis() + 5 * 1000;
        while current_millis() < wait_end {
            socket.work();
        }
    }
}

fn handle_http_request(request: &str) {
    // cut off first 5 characters (because we assume the req to start with "GET /")
    // same for the last for characters ("\r\n\r\n")
    // let size = request.len();
    let end_of_first_line = request.find('\r').unwrap();
    let truncated = &request[5..end_of_first_line];

    // further truncate await the " HTTP/1.1" suffix
    let truncated = &truncated[..truncated.len() - 9];
    log::info!("truncated: {:?}", truncated);

    match truncated {
        "half" => change_speed(0.5),
        "double" => change_speed(2.0),
        _ => (),
    }
}

fn change_speed(factor: f32) {
    critical_section::with(|cs| {
        let mut shared = SHARED.borrow_ref_mut(cs);
        let tap_info = shared.tap_info.as_mut().unwrap();
        tap_info.interval = MicrosDurationU64::from_ticks(
            (tap_info.interval.ticks() as f32 * 1f32 / factor) as u64,
        );
    });
}

#[handler]
fn render_timer_handler() {
    // log::info!("rendering...");
    // interrupt::disable(esp_hal::Cpu::ProCpu, Interrupt::TG1_T0_LEVEL);
    critical_section::with(|cs| {
        // wait clears the interrupt
        let mut shared = SHARED.borrow_ref_mut(cs);
        let timer = shared.render_timer.as_mut().unwrap();
        timer.clear_interrupt();
        timer.load_value(RENDER_INTERVAL.millis()).unwrap();
        timer.start();

        let channel = shared.rmt_channel.take();
        let rgbs = shared.rgbs.as_mut().unwrap();
        // log::info!("Sending data...");
        let channel = send_data(rgbs.next(), channel.unwrap());
        shared.rmt_channel.replace(channel);

        // log::info!("Sending data finished");
    });
    // interrupt::enable(Interrupt::TG1_T0_LEVEL, interrupt::Priority::Priority2);
    // log::info!("rendering over");
}

#[handler]
fn shoot_timer_handler() {
    critical_section::with(|cs| {
        let mut shared = SHARED.borrow_ref_mut(cs);
        let interval = shared.tap_info.as_mut().unwrap().interval;
        let timer = shared.shoot_timer.as_mut().unwrap();

        timer.clear_interrupt();
        timer.load_value(interval).unwrap();
        timer.start();
    });

    let last_shot = critical_section::with(|cs| {
        LAST_SHOT
            .borrow_ref_mut(cs)
            .unwrap_or(Instant::<u64, 1, 1000000>::from_ticks(0))
    });
    let current_time = current_time();
    log::info!("Shoot triggered! {:?}", current_time - last_shot);
    critical_section::with(|cs| LAST_SHOT.borrow_ref_mut(cs).replace(current_time));

    shoot_star();
}

fn shoot_star() {
    critical_section::with(|cs| {
        let mut shared = SHARED.borrow_ref_mut(cs);
        let rng = shared.rng.as_mut().unwrap();
        let color = Rgb::random(rng, MAX_INTENSITY);
        let speed = 2; //rng.random() % 4 + 1;
        let tail_length = 15; // rng.random() % 18 + 3;

        let rgbs = shared.rgbs.as_mut().unwrap();
        rgbs.shoot(color, speed as usize, tail_length as usize);

        let led = shared.led.as_mut().unwrap();
        led.toggle();
    })
}

#[handler]
fn button_press_handler() {
    let mut should_shoot = false;

    // enter critical section
    critical_section::with(|cs| {
        let mut shared = SHARED.borrow_ref_mut(cs);

        let tap_info = &mut shared.tap_info;
        if tap_info.is_none() {
            tap_info.replace(TapInfo {
                last_time: None,
                interval: 0.micros(),
                tap_series_count: 0,
                tap_series_start: None,
            });
        }

        // now, tap_info is definitely Some
        let tap_info = tap_info.as_mut().unwrap();

        let old_time = tap_info.last_time;

        // measure time
        let current_time = current_time();

        // set last time in info
        tap_info.last_time = Some(current_time);
        if tap_info.tap_series_start.is_none() {
            tap_info.tap_series_start = Some(current_time);
        }

        // calc speed and set it
        if let Some(old_t) = old_time {
            let duration = MicrosDurationU64::from_ticks(current_time.ticks() - old_t.ticks());

            if duration.ticks() < 200_000 {
                // filter out weird triggers (less than 0.2 sec, which would be >300 bpm)
                log::info!("Ignoring duration: {:?} (too short)", duration);

                // reset to old_time assuming that this was a false positive
                tap_info.last_time = old_time;
            } else if duration.ticks() > 1_000_000 {
                // filter out weird triggers (more than 1 sec, which would be < 60 bpm)
                log::info!("Ignoring duration: {:?} (too long)", duration);
                tap_info.tap_series_start = Some(current_time);
                tap_info.tap_series_count = 0;

                let interval = tap_info.interval;

                // reset timer speed
                shared
                    .shoot_timer
                    .as_mut()
                    .unwrap()
                    .load_value(interval)
                    .unwrap();

                // start the timer
                shared.shoot_timer.as_mut().unwrap().start();
                should_shoot = true;
            } else {
                log::info!("New duration: {:?}", duration);

                tap_info.tap_series_count += 1;
                let series_duration = MicrosDurationU64::from_ticks(
                    current_time.ticks() - tap_info.tap_series_start.unwrap().ticks(),
                );

                // set new interval to be used in shoots
                tap_info.interval = series_duration / tap_info.tap_series_count as u32;

                // set new timer speed
                shared
                    .shoot_timer
                    .as_mut()
                    .unwrap()
                    .load_value(duration)
                    .unwrap();

                // start the timer
                shared.shoot_timer.as_mut().unwrap().start();
                should_shoot = true;
            }
        }

        if !shared.render_started {
            let timer = shared.render_timer.as_mut().unwrap();
            timer.start();
            shared.render_started = true;
        }

        // reset the interrupt state
        shared.button.as_mut().unwrap().clear_interrupt();
    });

    // shoot outside of the critical_section to avoid double borrow
    if should_shoot {
        shoot_star();
    }
}

fn parse_ip(ip: &str) -> [u8; 4] {
    let mut result = [0u8; 4];
    for (idx, octet) in ip.split(".").into_iter().enumerate() {
        result[idx] = u8::from_str_radix(octet, 10).unwrap();
    }
    result
}
