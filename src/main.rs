#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    gpio::{Input, Io, Level, Output, Pull},
    peripherals::Peripherals,
    prelude::*,
    rmt::Rmt,
    rng::Rng,
    system::SystemControl,
};
use fugit::HertzU32;

use transmit::{send_data, Rgb};

mod transmit;
const N_LEDS: usize = 148;
const MAX_INTENSITY: u8 = 20;

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

    loop {
        if button.is_high() && !is_pressed {
            is_pressed = true;
        } else if button.is_low() && is_pressed {
            is_pressed = false;
            led.toggle();
            let mut colors = [Rgb::default(); N_LEDS];
            for col in colors.iter_mut() {
                col.r = (rng.random() % MAX_INTENSITY as u32) as u8;
                col.g = (rng.random() % MAX_INTENSITY as u32) as u8;
                col.b = (rng.random() % MAX_INTENSITY as u32) as u8;
            }
            // log::info!("colors: {:?}", colors);

            channel = send_data(&colors, channel);
        }
    }
}
