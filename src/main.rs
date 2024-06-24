#![no_std]
#![no_main]

use core::cell::RefCell;

use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
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
use fugit::{Instant, MicrosDurationU64};
use patterns::shooting_star::ShootingStar;
use transmit::send_data;
use util::color::Rgb;

mod patterns;
mod transmit;
mod util;

type TimerN<const N: u8> = Timer<TimerX<TIMG0, N>, Blocking>;

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
    shoot_timer: Option<TimerN<0>>,
    render_timer: Option<TimerN<1>>,
    rmt_channel: Option<Channel<Blocking, 0>>,
    led: Option<Output<'a, Gpio27>>,
    rgbs: Option<ShootingStar>,
    rng: Option<Rng>,
}
const N_LEDS: usize = 148;
const MAX_INTENSITY: u8 = 30;

static SHARED: Mutex<RefCell<SharedItems>> = Mutex::new(RefCell::new(SharedItems {
    tap_info: None,
    button: None,
    shoot_timer: None,
    render_timer: None,
    rmt_channel: None,
    led: None,
    rgbs: None,
    rng: None,
}));

static LAST_SHOT: Mutex<RefCell<Option<Instant<u64, 1, 1000000>>>> = Mutex::new(RefCell::new(None));
const RENDER_INTERVAL: u64 = 50;

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

    let channel = transmit::init_rmt(peripherals.RMT, io.pins.gpio26, &clocks);

    let rng = Rng::new(peripherals.RNG);
    let rgbs = ShootingStar::new((1, 4));

    let handlers = TimerInterrupts {
        timer0_t0: Some(shoot_timer_handler),
        timer0_t1: Some(render_timer_handler),
        ..Default::default()
    };
    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks, Some(handlers));

    let render_timer = timg0.timer1;
    render_timer.load_value(RENDER_INTERVAL.millis()).unwrap();
    render_timer.start();
    render_timer.listen();

    let shoot_timer = timg0.timer0;
    shoot_timer.listen();

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

    loop {}
}

#[handler]
fn render_timer_handler() {
    critical_section::with(|cs| {
        // wait clears the interrupt
        let mut shared = SHARED.borrow_ref_mut(cs);
        let timer = shared.render_timer.as_mut().unwrap();
        timer.clear_interrupt();
        timer.load_value(RENDER_INTERVAL.millis()).unwrap();
        timer.start();

        let channel = shared.rmt_channel.take();
        let rgbs = shared.rgbs.as_mut().unwrap();
        let channel = send_data(rgbs.next(), channel.unwrap());
        shared.rmt_channel.replace(channel);
    });
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
            } else {
                log::info!("New duration: {:?}", duration);

                tap_info.tap_series_count += 1;
                let series_duration = MicrosDurationU64::from_ticks(
                    current_time.ticks() - tap_info.tap_series_start.unwrap().ticks(),
                );
                tap_info.interval = series_duration / tap_info.tap_series_count as u32;

                // start timer with new speed
                shared
                    .shoot_timer
                    .as_mut()
                    .unwrap()
                    .load_value(duration)
                    .unwrap();

                // stop the current timer
                shared.shoot_timer.as_mut().unwrap().start();
                should_shoot = true;
            }
        }

        // reset the interrupt state
        shared.button.as_mut().unwrap().clear_interrupt();
    });

    // shoot outside of the critical_section to avoid double borrow
    if should_shoot {
        shoot_star();
    }
}
