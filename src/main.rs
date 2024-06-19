#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{Input, Io, Level, Output, Pull},
    peripherals::Peripherals,
    prelude::*,
    rmt::Rmt,
    rng::Rng,
    system::SystemControl,
};
use fugit::HertzU32;

use patterns::{
    breathing::{Breathing, BreathingMode},
    shooting_star::ShootingStar,
};
use transmit::send_data;
use util::color::Rgb;

mod patterns;
mod transmit;
mod util;

const N_LEDS: usize = 148;
const MAX_INTENSITY: u8 = 30;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);

    let clocks = ClockControl::max(system.clock_control).freeze();
    let rmt = Rmt::new(peripherals.RMT, HertzU32::MHz(80), &clocks, None).unwrap();

    esp_println::logger::init_logger_from_env();

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let mut led = Output::new(io.pins.gpio15, Level::Low);
    let button = Input::new(io.pins.gpio4, Pull::Up);

    let mut is_pressed = false;

    let mut channel = transmit::init_rmt(rmt, io.pins.gpio2);
    let mut rng = Rng::new(peripherals.RNG);
    let delay = Delay::new(&clocks);

    let mut rgbs = ShootingStar::new((1, 4));

    loop {
        channel = send_data(&mut rgbs.next(), channel);
        delay.delay(50.millis());

        if button.is_high() && !is_pressed {
            is_pressed = true;
        } else if button.is_low() && is_pressed {
            is_pressed = false;
            let color = Rgb::random(&mut rng, MAX_INTENSITY);
            let speed = rng.random() % 4 + 1;
            let tail_length = rng.random() % 18 + 3;
            rgbs.shoot(color, speed as usize, tail_length as usize);
        }
    }
}
