#![no_std]
#![no_main]

use core::cell::RefCell;

use bleps::{
    ad_structure::{
        create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
    },
    async_attribute_server::AttributeServer,
    asynch::Ble,
    attribute_server::NotificationData,
    gatt,
};
use critical_section::Mutex;
use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    gpio::{Gpio14, Gpio25, Gpio27, Input, Io, Level, Output, Pull},
    peripherals::Peripherals,
    prelude::*,
    rmt::Channel,
    rng::Rng,
    system::SystemControl,
    time::current_time,
    timer::timg::TimerGroup,
    Blocking,
};
use esp_wifi::{ble::controller::asynch::BleConnector, initialize, EspWifiInitFor};
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
static SHOOT_NOW_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

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

    let rng = Rng::new(peripherals.RNG);

    let timer = TimerGroup::new(peripherals.TIMG1, &clocks, None).timer0;
    let init = initialize(
        EspWifiInitFor::Ble,
        timer,
        rng,
        peripherals.RADIO_CLK,
        &clocks,
    )
    .unwrap();

    let bluetooth = peripherals.BT;

    let connector = BleConnector::new(&init, bluetooth);
    let ble = Ble::new(connector, esp_wifi::current_millis);
    log::info!("Connector created");

    let ble_button = Input::new(io.pins.gpio14, Pull::Up);
    let pin_ref = RefCell::new(ble_button);

    spawner.spawn(button_press_handler(button)).ok();
    spawner.spawn(render()).ok();
    spawner.spawn(shoot()).ok();
    spawner.spawn(ble_handling(ble, pin_ref)).ok();

    let led = Output::new(io.pins.gpio27, Level::Low);

    let channel = transmit::init_rmt(peripherals.RMT, io.pins.gpio26, &clocks);

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

fn handle_wireless_input(request: &str) {
    match request {
        "beat" => beat_button(),
        "half" => change_speed(0.5),
        "double" => change_speed(2.0),
        "stop" => critical_section::with(|cs| {
            let mut shared = SHARED.borrow_ref_mut(cs);
            shared.tap_info.as_mut().unwrap().interval = None;
        }),
        _ => (),
    }
}

#[embassy_executor::task]
async fn ble_handling(mut ble: Ble<BleConnector<'static>>, pin: RefCell<Input<'static, Gpio14>>) {
    loop {
        log::info!("{:?}", ble.init().await);
        log::info!("{:?}", ble.cmd_set_le_advertising_parameters().await);
        log::info!(
            "{:?}",
            ble.cmd_set_le_advertising_data(
                create_advertising_data(&[
                    AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                    AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1809)]),
                    AdStructure::CompleteLocalName("antons-esp"),
                ])
                .unwrap()
            )
            .await
        );
        log::info!("{:?}", ble.cmd_set_le_advertise_enable(true).await);

        log::info!("started advertising");

        let mut write_callback = |offset: usize, data: &[u8]| {
            log::info!("RECEIVED: Offset {}, data {:?}", offset, data);
            handle_wireless_input(core::str::from_utf8(data).unwrap());
        };

        gatt!([service {
            uuid: "937312e0-2354-11eb-9f10-fbc30a62cf38",
            characteristics: [characteristic {
                name: "socket",
                uuid: "987312e0-2354-11eb-9f10-fbc30a62cf38",
                notify: true,
                write: write_callback,
            },],
        },]);

        let mut rng = bleps::no_rng::NoRng;
        let mut srv = AttributeServer::new(&mut ble, &mut gatt_attributes, &mut rng);

        let counter = RefCell::new(0u8);
        let counter = &counter;

        let mut notifier = || async {
            pin.borrow_mut().wait_for_rising_edge().await;
            log::info!("button pressed");
            let mut data = [0u8; 13];
            data.copy_from_slice(b"Notification0");
            {
                let mut counter = counter.borrow_mut();
                data[data.len() - 1] += *counter;
                *counter = (*counter + 1) % 10;
            }
            NotificationData::new(socket_handle, &data)
        };

        srv.run(&mut notifier).await.unwrap();
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
    let mut is_repeating = false;
    let mut interval = 0;

    loop {
        let input_signal = SHOOT_NOW_SIGNAL.wait();
        if is_repeating {
            select(input_signal, Timer::after_micros(interval)).await;
        } else {
            // wait for the first and second beat input to be triggered
            input_signal.await;
        }

        critical_section::with(|cs| {
            let mut shared = SHARED.borrow_ref_mut(cs);
            let tap_info = shared.tap_info.as_mut();
            if let Some(info) = tap_info {
                if let Some(interv) = info.interval {
                    interval = interv;
                    is_repeating = true;
                }
            }
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
            shared.render_started = true;
        }
    });

    // signal the shooting task to stop waiting
    SHOOT_NOW_SIGNAL.signal(());
}
