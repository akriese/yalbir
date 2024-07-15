use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

pub mod counting;
pub mod tapping;

static SHOOT_NOW_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Structure to signal the position of a beat trigger in a 4/4 music environment.
/// for each field, if it is `None`, it is not triggered, while if it is `Some(n)`, then
/// it is triggering and `n` tells about the position in the current 4/4 bar.
///
/// All measures are zero indexed for easier handling.
///
/// * `n_full`: Only `Some(0)` if the very first beat of the current bar is triggered.
/// * `n_half`: Triggers twice per bar.
/// * `n_quarter`: Triggers four times per bar.
/// * `n8th`: Triggers eight times per bar.
/// * `n16th`: Triggers 16 times per bar.
/// * `n32th`: Triggers 32 times per bar.
#[derive(Debug, Copy, Clone)]
pub struct BeatCount {
    pub n_full: Option<usize>,
    pub n_half: Option<usize>,
    pub n_quarter: Option<usize>,
    pub n8th: Option<usize>,
    pub n16th: Option<usize>,
    pub n32th: usize,
}

impl Default for BeatCount {
    fn default() -> Self {
        Self {
            n_full: Some(0),
            n_half: Some(0),
            n_quarter: Some(0),
            n8th: Some(0),
            n16th: Some(0),
            n32th: 0,
        }
    }
}

impl BeatCount {
    pub fn increment(&mut self) {
        // increment only the lowest counter
        self.n32th = (self.n32th + 1) % 32;

        // then update the other fields from there
        self.n16th = if self.n32th % 2 == 0 {
            Some(self.n32th / 2)
        } else {
            None
        };
        self.n8th = if self.n32th % 4 == 0 {
            Some(self.n32th / 4)
        } else {
            None
        };
        self.n_quarter = if self.n32th % 8 == 0 {
            Some(self.n32th / 8)
        } else {
            None
        };
        self.n_half = if self.n32th % 16 == 0 {
            Some(self.n32th / 16)
        } else {
            None
        };
        self.n_full = if self.n32th % 32 == 0 {
            Some(self.n32th / 32)
        } else {
            None
        };
    }
}
