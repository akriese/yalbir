#![no_std]
#![no_main]

use core::cell::RefCell;

use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    gpio::{Gpio25, Gpio27, Input, Io, Level, Output, Pull},
    peripherals::Peripherals,
    prelude::*,
    rmt::Channel,
    rng::Rng,
    system::SystemControl,
    time::current_time,
    timer::timg::TimerGroup,
    Blocking,
};

use critical_section::Mutex;
use esp_backtrace as _;
use fugit::{Instant, MicrosDurationU64};
use patterns::shooting_star::ShootingStar;
use transmit::send_data;
use util::color::Rgb;

mod patterns;
mod transmit;
mod util;

#[derive(Debug, Clone)]
struct TapInfo {
    last_time: Option<Instant<u64, 1, 1000000>>,
    interval: Option<u64>,
    tap_series_count: u8,
    tap_series_start: Option<Instant<u64, 1, 1000000>>,
}

struct SharedItems<'a> {
    tap_info: Option<TapInfo>,
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
    rmt_channel: None,
    led: None,
    rgbs: None,
    rng: None,
    render_started: false,
}));

static LAST_SHOT: Mutex<RefCell<Option<Instant<u64, 1, 1000000>>>> = Mutex::new(RefCell::new(None));
static EARLY_SHOOT_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

#[main]
async fn main(spawner: Spawner) {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);

    let clocks = ClockControl::max(system.clock_control).freeze();

    esp_println::logger::init_logger_from_env();

    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    esp_hal_embassy::init(&clocks, timg0);

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let button = Input::new(io.pins.gpio25, Pull::Up);

    spawner.spawn(button_press_handler(button)).ok();
    spawner.spawn(render()).ok();
    spawner.spawn(shoot()).ok();

    let led = Output::new(io.pins.gpio27, Level::Low);

    let channel = transmit::init_rmt(peripherals.RMT, io.pins.gpio26, &clocks);

    let rng = Rng::new(peripherals.RNG);

    let rgbs = ShootingStar::new(400);

    critical_section::with(|cs| {
        let mut shared = SHARED.borrow_ref_mut(cs);
        shared.led.replace(led);
        shared.rgbs.replace(rgbs);
        shared.rng.replace(rng);
        shared.rmt_channel.replace(channel);
    });

    log::info!("before loop");

    loop {
        Timer::after_secs(1).await;
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
        tap_info.interval = Some((tap_info.interval.unwrap() as f32 * 1f32 / factor) as u64);
    });
}

#[embassy_executor::task]
async fn render() -> ! {
    loop {
        critical_section::with(|cs| {
            // wait clears the interrupt
            let mut shared = SHARED.borrow_ref_mut(cs);

            let channel = shared.rmt_channel.take();
            let rgbs = shared.rgbs.as_mut().unwrap();
            let channel = send_data(*rgbs.next(), channel.unwrap());
            shared.rmt_channel.replace(channel);
        });

        Timer::after_millis(RENDER_INTERVAL).await;
    }
}

#[embassy_executor::task]
async fn shoot() {
    let mut interval = 1000;
    let mut shoot = false;

    loop {
        select(EARLY_SHOOT_SIGNAL.wait(), Timer::after_micros(interval)).await;

        critical_section::with(|cs| {
            let mut shared = SHARED.borrow_ref_mut(cs);
            let tap_info = shared.tap_info.as_mut();
            if let Some(info) = tap_info {
                if let Some(interv) = info.interval {
                    interval = interv;
                    shoot = true;
                }
            }
        });

        if !shoot {
            continue;
        }

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

#[embassy_executor::task]
async fn button_press_handler(mut button: Input<'static, Gpio25>) {
    loop {
        log::info!("Waiting for button press...");
        button.wait_for_rising_edge().await;
        beat_button();
    }
}

fn beat_button() {
    // enter critical section
    critical_section::with(|cs| {
        let mut shared = SHARED.borrow_ref_mut(cs);

        let tap_info = &mut shared.tap_info;
        if tap_info.is_none() {
            tap_info.replace(TapInfo {
                last_time: None,
                interval: None,
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
            } else {
                log::info!("New duration: {:?}", duration);

                tap_info.tap_series_count += 1;
                let series_duration = MicrosDurationU64::from_ticks(
                    current_time.ticks() - tap_info.tap_series_start.unwrap().ticks(),
                );

                // set new interval to be used in shoots
                tap_info.interval =
                    Some((series_duration / tap_info.tap_series_count as u32).ticks());
            }
        }

        if !shared.render_started {
            // let timer = shared.render_timer.as_mut().unwrap();
            // timer.start();
            shared.render_started = true;
        }
    });

    // signal the shooting task to stop waiting
    EARLY_SHOOT_SIGNAL.signal(());
}
