pub mod counting;
pub mod tapping;

/// Structure to signal the position of a beat trigger in a 4/4 music environment.
/// for each field, if it is `None`, it is not triggered, while if it is `Some(n)`, then
/// it is triggering and `n` tells about the position in the current 4/4 bar.
///
/// * `n_full`: Only `Some(1)` if the very first beat of the current bar is triggered.
/// * `n_half`: Triggers twice per bar.
/// * `n_quarter`: Triggers four times per bar.
/// * `n8th`: Triggers eight times per bar.
/// * `n16th`: Triggers 16 times per bar.
/// * `n32th`: Triggers 32 times per bar.
pub struct BeatCount {
    n_full: Option<usize>,
    n_half: Option<usize>,
    n_quarter: Option<usize>,
    n8th: Option<usize>,
    n16th: Option<usize>,
    n32th: Option<usize>,
}
