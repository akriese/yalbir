use crate::{beat::BeatCount, util::color::Rgb};

pub mod breathing;
pub mod partitioned;
pub mod shooting_star;
pub mod strobe;

pub trait LedPattern: Send + Sync + PatternCommand {
    fn next(&mut self) -> &[Rgb];

    fn beat(&mut self, beat_info: &BeatCount);
}

pub trait PatternCommand {
    fn execute_command(&mut self, command: &str) -> Result<(), ()>;
}

#[derive(Copy, Clone, Debug, Default)]
enum PatternSpeed {
    N32,
    N16,
    N8,
    #[default]
    N4,
    N2,
    N1,
}

impl PatternSpeed {
    fn faster(&mut self) {
        *self = match self {
            Self::N32 => Self::N32,
            Self::N16 => Self::N32,
            Self::N8 => Self::N16,
            Self::N4 => Self::N8,
            Self::N2 => Self::N4,
            Self::N1 => Self::N2,
        }
    }

    fn slower(&mut self) {
        *self = match self {
            Self::N32 => Self::N16,
            Self::N16 => Self::N8,
            Self::N8 => Self::N4,
            Self::N4 => Self::N2,
            Self::N2 => Self::N1,
            Self::N1 => Self::N1,
        }
    }

    fn is_triggered(&self, beat_info: &BeatCount) -> bool {
        match self {
            Self::N32 => true,
            Self::N16 => beat_info.n16th.is_some(),
            Self::N8 => beat_info.n8th.is_some(),
            Self::N4 => beat_info.n_quarter.is_some(),
            Self::N2 => beat_info.n_half.is_some(),
            Self::N1 => beat_info.n_full.is_some(),
        }
    }
}
