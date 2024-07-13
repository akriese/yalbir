#![no_std]
#![no_main]

extern crate alloc;
use alloc::boxed::Box;
use bleps::asynch::Ble;
use core::{cell::RefCell, mem::MaybeUninit};
use critical_section::Mutex;
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
use esp_wifi::{ble::controller::asynch::BleConnector, initialize, EspWifiInitFor};
use fugit::{Instant, MicrosDurationU64};
use patterns::{
    breathing::Breathing,
    shooting_star::ShootingStar,
    strobe::{Strobe, StrobeMode},
    LedPattern, PartitionedPatterns,
};
use transmit::send_data;

mod patterns;
mod transmit;
mod util;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

#[derive(Debug, Clone)]
struct TapInfo {
    last_time: Option<Instant<u64, 1, 1000000>>,
    interval: Option<u64>,
    is_stopped: bool,
    tap_series_count: u8,
    tap_series_start: Option<Instant<u64, 1, 1000000>>,
}

struct SharedItems<'a> {
    tap_info: Option<TapInfo>,
    rmt_channel: Option<Channel<Blocking, 0>>,
    led: Option<Output<'a, Gpio27>>,
    rgbs: Option<PartitionedPatterns>,
    rng: Option<Rng>,
}
const N_LEDS: usize = 149;
const MAX_INTENSITY: u8 = 30;
const RENDERS_PER_SECOND: usize = 50;
const RENDER_INTERVAL: usize = 1000 / RENDERS_PER_SECOND; // in milliseconds

static SHARED: Mutex<RefCell<SharedItems>> = Mutex::new(RefCell::new(SharedItems {
    tap_info: None,
    rmt_channel: None,
    led: None,
    rgbs: None,
    rng: None,
}));

static LAST_SHOT: Mutex<RefCell<Option<Instant<u64, 1, 1000000>>>> = Mutex::new(RefCell::new(None));
static SHOOT_NOW_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

#[main]
async fn main(spawner: Spawner) {
    init_heap();

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

    let led = Output::new(io.pins.gpio27, Level::Low);
    let channel = transmit::init_rmt(peripherals.RMT, io.pins.gpio26, &clocks);

    let rgbs = init_rgbs(rng);

    critical_section::with(|cs| {
        let mut shared = SHARED.borrow_ref_mut(cs);
        shared.led.replace(led);
        shared.rgbs.replace(rgbs);
        shared.rng.replace(rng);
        shared.rmt_channel.replace(channel);
    });

    spawner.spawn(button_press_handler(button)).ok();
    spawner.spawn(render()).ok();
    spawner.spawn(shoot()).ok();
    spawner.spawn(util::ble::ble_handling(ble, pin_ref)).ok();
}

fn init_rgbs(rng: Rng) -> PartitionedPatterns {
    let mut rgbs = PartitionedPatterns::new();
    rgbs.add(
        Box::new(Strobe::<4>::new(StrobeMode::Single, rng.clone(), 30)),
        (0, 4),
    );
    rgbs.add(
        Box::new(Strobe::<4>::new(StrobeMode::Individual, rng.clone(), 5)),
        (4, 8),
    );
    rgbs.add(
        Box::new(Strobe::<4>::new(StrobeMode::Unison, rng.clone(), 25)),
        (8, 12),
    );
    rgbs.add(
        Box::new(Breathing::<4>::new(
            patterns::breathing::BreathingMode::Mixed,
            60,
            &mut rng.clone(),
            2.0,
        )),
        (12, 16),
    );
    rgbs.add(
        Box::new(ShootingStar::<{ N_LEDS - 16 }, 20>::new(400, rng.clone())),
        (16, N_LEDS),
    );

    rgbs
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
            let channel = send_data(rgbs.next(), channel.unwrap());
            shared.rmt_channel.replace(channel);
        });

        Timer::after_millis(RENDER_INTERVAL as u64).await;
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
                if info.is_stopped {
                    is_repeating = false;
                } else {
                    if let Some(interv) = info.interval {
                        interval = interv;
                        is_repeating = true;
                    }
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

        let rgbs = shared.rgbs.as_mut().unwrap();
        rgbs.beat();

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
                is_stopped: false,
                tap_series_count: 0,
                tap_series_start: None,
            });
        }

        // now, tap_info is definitely Some
        let tap_info = tap_info.as_mut().unwrap();
        tap_info.is_stopped = false;

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
    });

    // signal the shooting task to stop waiting
    SHOOT_NOW_SIGNAL.signal(());
}
