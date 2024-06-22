use esp_hal::{
    clock::Clocks,
    peripheral::Peripheral,
    peripherals::TIMG0,
    timer::{
        timg::{Timer, TimerGroup},
        PeriodicTimer,
    },
};

pub fn init_timer<'d>(
    clocks: &Clocks,
    peripherals: impl Peripheral<P = TIMG0> + 'd,
) -> PeriodicTimer<Timer<esp_hal::timer::timg::TimerX<TIMG0, 0>, esp_hal::Blocking>> {
    let timg0 = TimerGroup::new(peripherals, &clocks, None);
    PeriodicTimer::new(timg0.timer0)
}
