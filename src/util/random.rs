use esp_hal::rng::Rng;

use crate::RNG;

pub fn get_rng() -> Rng {
    critical_section::with(|cs| RNG.borrow_ref(cs).unwrap())
}

pub fn from_range(range: (usize, usize), rng: &mut Rng) -> usize {
    assert!(range.1 >= range.0);

    if range.0 == range.1 {
        range.0
    } else {
        rng.random() as usize % (range.1 - range.0) + range.0
    }
}
