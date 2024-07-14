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

use beat::{
    counting::shoot,
    tapping::{button_press_handler, TapInfo},
};
use patterns::{
    breathing::Breathing,
    shooting_star::ShootingStar,
    strobe::{Strobe, StrobeMode},
    LedPattern, PartitionedPatterns,
};
use transmit::send_data;

mod beat;
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

fn rgbs_issue_beat() {
    critical_section::with(|cs| {
        let mut shared = SHARED.borrow_ref_mut(cs);

        let rgbs = shared.rgbs.as_mut().unwrap();
        rgbs.beat();

        let led = shared.led.as_mut().unwrap();
        led.toggle();
    })
}
