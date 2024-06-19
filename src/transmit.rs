use esp_hal::{
    rmt::{Channel, PulseCode, Rmt, TxChannel, TxChannelConfig, TxChannelCreator},
    Blocking,
};
use esp_println::logger;

use crate::N_LEDS;

const NS_PER_CLOCK_CYCLE: u16 = (1000_000_000_f32 / 80_000_000_f32) as u16;
const T0_HIGH: u16 = 350 / NS_PER_CLOCK_CYCLE;
const T1_HIGH: u16 = 700 / NS_PER_CLOCK_CYCLE;
const T_LOW_GAP: u16 = 600 / NS_PER_CLOCK_CYCLE;
const T_LOW_RESET: u16 = 6000 / NS_PER_CLOCK_CYCLE;

pub fn init_rmt<'d, P: esp_hal::gpio::OutputPin>(
    rmt: Rmt<'d, Blocking>,
    pin: impl esp_hal::peripheral::Peripheral<P = P> + 'd,
) -> Channel<Blocking, 0> {
    let channel = rmt
        .channel0
        .configure(
            pin,
            TxChannelConfig {
                clk_divider: 1,
                idle_output_level: false,
                idle_output: false,
                carrier_modulation: false,
                carrier_high: 1,
                carrier_low: 1,
                carrier_level: false,
            },
        )
        .unwrap();

    return channel;
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub fn send_data(data: &[Rgb; N_LEDS], channel: Channel<Blocking, 0>) -> Channel<Blocking, 0> {
    let mut send_data = [PulseCode::default(); N_LEDS * 24];
    let mut ch = channel;
    for (i, rgb) in data.iter().enumerate() {
        // WS2818 LED strip expects the order green, then red, then blue
        // each component's most significant bit has to be sent first up until the least
        // significant bit. Timings inspired by
        // https://wp.josh.com/2014/05/13/ws2812-neopixels-are-not-so-finicky-once-you-get-to-know-them/
        for (j, col) in [rgb.g, rgb.r, rgb.b].iter().enumerate() {
            for bit_pos in 0..8 {
                let bit_is_high: bool = (col >> bit_pos) & 0b1 == 1;
                let code = if bit_is_high {
                    PulseCode {
                        level1: true,
                        length1: T1_HIGH,
                        level2: false,
                        length2: T_LOW_GAP,
                    }
                } else {
                    PulseCode {
                        level1: true,
                        length1: T0_HIGH,
                        level2: false,
                        length2: T_LOW_GAP,
                    }
                };
                send_data[i * 24 + j * 8 + 7 - bit_pos] = code;
            }
        }
    }

    // tell the RMT to stop transmitting with a zero-length second pulse
    send_data.last_mut().unwrap().length2 = 0;

    let transaction = ch.transmit(&send_data);
    ch = transaction.wait().unwrap();

    ch
}
