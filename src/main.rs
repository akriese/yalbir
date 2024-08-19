#![no_std]
#![no_main]

extern crate alloc;

mod beat;
mod color;
mod patterns;
mod transmit;
mod util;

use alloc::boxed::Box;
use core::{cell::RefCell, mem::MaybeUninit};

use bleps::asynch::Ble;
use critical_section::Mutex;
use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    gpio::{Gpio26, Input, Io, Level, Output, Pull},
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

use beat::{
    counting::beat_executor,
    tapping::{button_press_handler, TapInfo},
    BeatCount,
};
use patterns::{
    breathing::Breathing,
    caterpillar::CaterPillars,
    partitioned::PartitionedPatterns,
    strobe::{Strobe, StrobeMode},
    LedPattern,
};
use transmit::send_data;
use util::ble::ble_handling;

const N_LEDS: usize = 44 + 11 + 12;
const MAX_INTENSITY: u8 = 30;
const RENDERS_PER_SECOND: usize = 50;
const RENDER_INTERVAL: usize = 1000 / RENDERS_PER_SECOND; // in milliseconds
const HEAP_SIZE: usize = 32 * 1024;

struct SharedItems<'a> {
    tap_info: Option<TapInfo>,
    led: Option<Output<'a, Gpio26>>,
    rgbs: Option<PartitionedPatterns>,
}

static SHARED: Mutex<RefCell<SharedItems>> = Mutex::new(RefCell::new(SharedItems {
    tap_info: None,
    led: None,
    rgbs: None,
}));
static RNG: Mutex<RefCell<Option<Rng>>> = Mutex::new(RefCell::new(None));

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

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

    let rng = Rng::new(peripherals.RNG);

    let led = Output::new(io.pins.gpio26, Level::Low);
    let rgbs = init_rgbs(rng);

    critical_section::with(|cs| {
        let mut shared = SHARED.borrow_ref_mut(cs);
        shared.led.replace(led);
        shared.rgbs.replace(rgbs);
    });

    critical_section::with(|cs| RNG.borrow_ref_mut(cs).replace(rng));

    // create the task that listens to the beat button being pressed
    let button = Input::new(io.pins.gpio25, Pull::Up);
    spawner.spawn(button_press_handler(button)).ok();

    // create the RGB LED strip render task giving it the RMT channel
    let channel = transmit::init_rmt(peripherals.RMT, io.pins.gpio27, &clocks);
    spawner.spawn(render(channel)).ok();

    // create the task that fires in intervals according to the music's beat
    spawner.spawn(beat_executor()).ok();

    // initialize BLE and the task for handling BT commands
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

    spawner.spawn(ble_handling(ble)).ok();
}

fn init_heap() {
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

fn init_rgbs(rng: Rng) -> PartitionedPatterns {
    let mut rgbs = PartitionedPatterns::new(N_LEDS);
    rgbs.add(Box::new(CaterPillars::new(44, None, 120, rng)), None);
    rgbs.add(
        Box::new(Breathing::new(
            10,
            patterns::breathing::BreathingMode::Mixed,
            60,
            rng,
            0.5,
        )),
        None,
    );
    rgbs.add(Box::new(Strobe::new(12, StrobeMode::Unison, rng, 6)), None);

    rgbs
}

#[embassy_executor::task]
async fn render(rmt_channel: Channel<Blocking, 0>) -> ! {
    let channel: Mutex<RefCell<Option<Channel<Blocking, 0>>>> =
        Mutex::new(RefCell::new(Some(rmt_channel)));

    loop {
        let process_start_time = current_time();

        critical_section::with(|cs| {
            // wait clears the interrupt
            let mut shared = SHARED.borrow_ref_mut(cs);

            let rgb_data = shared.rgbs.as_mut().unwrap();

            // ATTENTION: apparently this operation cant simply be moved out of the
            // closure as a side effect is, that the sending is somehow interrupted
            // from time to time leading to weird jittering in the animation.
            let mut ch = channel.borrow_ref_mut(cs);
            let c = send_data(&rgb_data.next(), ch.take().unwrap());
            ch.replace(c);
        });

        // wait less millis accounting for how long the previous render took
        let previous_render_time = (current_time() - process_start_time).to_millis();
        if previous_render_time >= RENDER_INTERVAL as u64 {
            log::info!(
                "Experiencing longer render calc than render interval {} >= {}",
                previous_render_time,
                RENDER_INTERVAL
            );
        } else {
            Timer::after_millis(RENDER_INTERVAL as u64 - previous_render_time).await;
        }
    }
}

fn rgbs_issue_beat(beat_info: &BeatCount) {
    critical_section::with(|cs| {
        let mut shared = SHARED.borrow_ref_mut(cs);

        if let Some(c) = beat_info.n_quarter {
            let led = shared.led.as_mut().unwrap();
            if c % 2 == 0 {
                led.set_high();
            } else {
                led.set_low();
            }
        }

        let rgbs = shared.rgbs.as_mut().unwrap();
        rgbs.beat(beat_info);
    })
}
