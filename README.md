# YaLbiR - Yet another LED strip controller (but in Rust)

This is a learning project to get to know the usage of an ESP32 with Rust and the
control of a WS2812 type LED strip with individually accessible LEDs.

## Setup

See `setup-steps.txt` to look for the dev environment setup. Steps might vary between
different operating systems.

I generally used the following links for starters:

- [Rust on ESP book](https://docs.esp-rs.org/book/introduction.html)
- [ESP32 documentation](https://www.espressif.com/sites/default/files/documentation/esp32_technical_reference_manual_en.pdf)

## Approach

Using the esp-hal crate, the access of things like the RMT to send out data via a GPIO
are pretty straight forward. The LED strip expects the data for all LEDs to be sent out
in a certain order (highest bit first; order green, blue, red) and consumes the RGB data
down the strip. More information be found in the
[official documentation](https://cdn-shop.adafruit.com/datasheets/WS2812B.pdf).

[This post](https://wp.josh.com/2014/05/13/ws2812-neopixels-are-not-so-finicky-once-you-get-to-know-them/)
helped me to get the timings to encode single bits right.

## Run

Connect your ESP32 via the USB port to your PC and make sure serial to USB driver and
other tools mentioned in `setup-steps.txt` are installed.
Execute `cargo run` to compile and upload the binary to the controller. If no connection
can be made, press the BOOT button on the ESP for a second or two while the console says
"Connecting...".

For some examples, buttons or other devices must be connected to the controller at
specific GPIO ports. Read the comments or commit messages for more info about that.

## Roadmap

General features:

- [x] LED strip RGB encoding
- [ ] Sound reaction
- [ ] Wi-Fi setup
- [ ] Socket for requests
- [ ] Beat setting via button

Patterns:

- [x] Classic Breathing (random colors)
- [ ] Individual breathing
- [x] Shooting Stars
- [ ] Sliding Rainbow
