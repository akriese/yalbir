//! Animation patterns to be displayed on an LED strip
//!
//! If you want to create a new pattern, copy the following snippet:
//!
//! struct NewPattern {
//!     rgbs: Vec<Rgb>,
//!     // ...
//! }
//!
//! impl NewPattern {
//!     pub fn new() -> Self {
//!         //
//!     }
//! }
//!
//! impl LedPattern for NewPattern {
//!     fn next(&mut self) -> &[Rgb] {
//!         todo!();
//!     }
//!
//!     fn beat(&mut self, beat_info: &BeatCount) {
//!         todo!();
//!     }
//!
//!     fn size(&self) -> usize {
//!         todo!();
//!     }
//! }
//!
//! impl PatternCommand for NewPattern {
//!     fn execute_command(&mut self, command: &str) -> Result<(), ()> {
//!         todo!();
//!     }
//! }

use crate::{beat::BeatCount, util::color::Rgb};

// pub mod background;
pub mod breathing;
pub mod caterpillar;
pub mod partitioned;
pub mod shooting_star;
pub mod strobe;

pub trait LedPattern: Send + Sync + PatternCommand {
    // render function to get the next RGB state of the pattern
    fn next(&mut self) -> &[Rgb];

    // react to a music beat
    fn beat(&mut self, beat_info: &BeatCount);

    // number of LEDs inside the pattern
    fn size(&self) -> usize;
}

pub trait PatternCommand {
    fn execute_command(&mut self, command: &str) -> Result<(), ()>;
}

#[derive(Copy, Clone, Debug, Default)]
pub enum PatternSpeed {
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

    fn change(&mut self, command: char) -> Result<(), ()> {
        match command {
            '0' => *self = PatternSpeed::N1,
            '1' => *self = PatternSpeed::N2,
            '2' => *self = PatternSpeed::N4,
            '3' => *self = PatternSpeed::N8,
            '4' => *self = PatternSpeed::N16,
            '5' => *self = PatternSpeed::N32,
            'f' => self.faster(),
            's' => self.slower(),
            _ => return Err(()),
        };

        Ok(())
    }
}
