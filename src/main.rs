#![no_std]
#![no_main]

use core::cell::RefCell;

use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{Event, Gpio25, Input, Io, Level, Output, Pull},
    peripherals::{Peripherals, TIMG0},
    prelude::*,
    rng::Rng,
    system::SystemControl,
    time::current_time,
    timer::{
        timg::{Timer, TimerX},
        PeriodicTimer,
    },
    Blocking,
};

use critical_section::Mutex;
use fugit::{Instant, MicrosDurationU64};
use patterns::shooting_star::ShootingStar;
use transmit::send_data;
use util::{color::Rgb, timer::init_timer};

mod patterns;
mod transmit;
mod util;

#[derive(Debug, Clone)]
struct TapInfo {
    last_time: Option<Instant<u64, 1, 1000000>>,
    speed: u8, // up to 255 BPM
}

struct SharedItems<'a> {
    tap_info: TapInfo,
    button: Option<Input<'a, Gpio25>>,
    timer: Option<PeriodicTimer<Timer<TimerX<TIMG0, 0>, Blocking>>>,
}
const N_LEDS: usize = 148;
const MAX_INTENSITY: u8 = 30;

static SHARED: Mutex<RefCell<SharedItems>> = Mutex::new(RefCell::new(SharedItems {
    tap_info: TapInfo {
        last_time: None,
        speed: 0,
    },
    button: None,
    timer: None,
}));

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);

    let clocks = ClockControl::max(system.clock_control).freeze();

    esp_println::logger::init_logger_from_env();

    let mut io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    io.set_interrupt_handler(interrupt_handler);

    let mut led = Output::new(io.pins.gpio27, Level::Low);
    let mut button = Input::new(io.pins.gpio25, Pull::Up);

    let timer = init_timer(&clocks, peripherals.TIMG0);

    critical_section::with(|cs| {
        button.listen(Event::FallingEdge);
        let mut shared = SHARED.borrow_ref_mut(cs);
        shared.button.replace(button);
        shared.timer.replace(timer);
    });

    let mut channel = transmit::init_rmt(peripherals.RMT, io.pins.gpio26, &clocks);

    let mut rng = Rng::new(peripherals.RNG);
    let delay = Delay::new(&clocks);
    let mut rgbs = ShootingStar::new((1, 4));

    loop {
        let mut ready_to_shoot = false;
        critical_section::with(|cs| {
            ready_to_shoot = SHARED
                .borrow_ref_mut(cs)
                .timer
                .as_mut()
                .unwrap()
                .wait()
                .is_ok()
        });

        if ready_to_shoot {
            // log::info!("Timer triggered!");
            let color = Rgb::random(&mut rng, MAX_INTENSITY);
            let speed = rng.random() % 4 + 1;
            let tail_length = rng.random() % 18 + 3;
            rgbs.shoot(color, speed as usize, tail_length as usize);
            led.toggle();
        }

        channel = send_data(rgbs.next(), channel);
        delay.delay(10.millis());
    }
}

#[handler]
fn interrupt_handler() {
    // enter critical section
    critical_section::with(|cs| {
        let mut shared = SHARED.borrow_ref_mut(cs);

        let old_time = shared.tap_info.last_time;

        // measure time
        let current_time = current_time();

        // set last time in info
        shared.tap_info.last_time = Some(current_time);

        // calc speed and set it
        if let Some(old_t) = old_time {
            let duration = MicrosDurationU64::from_ticks(current_time.ticks() - old_t.ticks());

            if duration.ticks() < 100_000 {
                // filter out weird triggers (less than 0.1 sec, which would be >600 bpm)
                log::info!("Ignoring duration: {:?} (too short)", duration);

                // reset to old_time assuming that this was a false positive
                shared.tap_info.last_time = old_time;
            } else if duration.ticks() > 2_000_000 {
                // filter out weird triggers (more than 2 sec, which would be < 30 bpm)
                log::info!("Ignoring duration: {:?} (too long)", duration);
            } else {
                // stop the current timer
                let _ = shared.timer.as_mut().unwrap().cancel();

                log::info!("New duration: {:?}", duration);

                // start timer with new speed
                let _ = shared.timer.as_mut().unwrap().start(duration);
            }
        }

        // reset the interrupt state
        shared.button.as_mut().unwrap().clear_interrupt();
    });
}
