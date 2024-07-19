use esp_hal::rng::Rng;

pub fn from_range(range: (usize, usize), rng: &mut Rng) -> usize {
    if range.0 == range.1 {
        range.0
    } else {
        rng.random() as usize % (range.1 - range.0) + range.0
    }
}
