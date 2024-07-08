use esp_hal::{
    clock::Clocks,
    peripheral::Peripheral,
    peripherals::RMT,
    rmt::{
        Channel, PulseCode, Rmt, SingleShotTxTransaction, TxChannel, TxChannelConfig,
        TxChannelCreator,
    },
    Blocking,
};
use fugit::HertzU32;

use crate::{util::color::Rgb, N_LEDS};

const NS_PER_CLOCK_CYCLE: u16 = (1_000_000_000_f32 / 80_000_000_f32) as u16;
const T0_HIGH: u16 = 350 / NS_PER_CLOCK_CYCLE;
const T1_HIGH: u16 = 700 / NS_PER_CLOCK_CYCLE;
const T_LOW_GAP: u16 = 600 / NS_PER_CLOCK_CYCLE;
const T_LOW_RESET: u16 = 6000 / NS_PER_CLOCK_CYCLE;

static mut RMT_ENCODING: [PulseCode; N_LEDS * 24] = [PulseCode {
    level1: true,
    length1: 0,
    level2: true,
    length2: 0,
}; N_LEDS * 24];

pub fn init_rmt<'d, P: esp_hal::gpio::OutputPin>(
    rmt: impl Peripheral<P = RMT> + 'd,
    pin: impl Peripheral<P = P> + 'd,
    clocks: &Clocks,
) -> Channel<Blocking, 0> {
    let _rmt = Rmt::new(rmt, HertzU32::MHz(80), clocks, None).unwrap();
    _rmt.channel0
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
        .unwrap()
}

pub fn send_data(data: [Rgb; N_LEDS], channel: Channel<Blocking, 0>) -> Channel<Blocking, 0> {
    let transaction = send_data_no_wait(data, channel);
    transaction.wait().unwrap()
}

pub fn send_data_no_wait(
    rgb_data: [Rgb; N_LEDS],
    channel: Channel<Blocking, 0>,
) -> SingleShotTxTransaction<'static, Channel<Blocking, 0>, PulseCode> {
    for (i, rgb) in rgb_data.iter().enumerate() {
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
                unsafe { RMT_ENCODING[i * 24 + j * 8 + 7 - bit_pos] = code };
            }
        }
    }

    // tell the RMT to stop transmitting with a zero-length second pulse
    unsafe { RMT_ENCODING.last_mut().unwrap().length2 = 0 };

    channel.transmit(unsafe { &RMT_ENCODING })
}
