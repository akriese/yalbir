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
//!         todo!();
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

use crate::{beat::BeatCount, color::Rgb};
use alloc::boxed::Box;
use anyhow::{anyhow, Result};
use background::Background;
use breathing::Breathing;
use caterpillar::CaterPillars;
use nom::{
    bytes::complete::{tag, take_until, take_while},
    sequence::delimited,
    IResult,
};
use partitioned::PartitionedPatterns;
use shooting_star::ShootingStar;
use strobe::Strobe;

pub mod background;
pub mod breathing;
pub mod caterpillar;
pub mod command;
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

    fn from_str(args: &str) -> anyhow::Result<Self>
    where
        Self: Sized;
}

pub trait PatternCommand {
    fn execute_command(&mut self, command: &str) -> anyhow::Result<()>;
}

#[derive(Clone, Debug)]
enum PatternKind {
    Background,
    Breathing,
    Caterpillar,
    Partitioned,
    ShootingStar,
    Strobe,
}

impl TryFrom<&str> for PatternKind {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "br" => Ok(PatternKind::Breathing),
            "ba" => Ok(PatternKind::Background),
            "cat" => Ok(PatternKind::Caterpillar),
            "pt" => Ok(PatternKind::Partitioned),
            "shst" => Ok(PatternKind::ShootingStar),
            "str" => Ok(PatternKind::Strobe),
            c => Err(anyhow!("Invalid PatternKind {:?}. Available types are: br - Breathing; ba - Background; cat - CaterPillars; pt - Partitioned; shst - ShootingStar; str - Strobe", c)),
        }
    }
}

impl PatternKind {
    pub fn to_pattern(&self, args: &str) -> anyhow::Result<Box<dyn LedPattern>> {
        let res: Box<dyn LedPattern> = match self {
            PatternKind::Background => Box::new(Background::from_str(args)?),
            PatternKind::Breathing => Box::new(Breathing::from_str(args)?),
            PatternKind::Caterpillar => Box::new(CaterPillars::from_str(args)?),
            PatternKind::Partitioned => Box::new(PartitionedPatterns::from_str(args)?),
            PatternKind::ShootingStar => Box::new(ShootingStar::from_str(args)?),
            PatternKind::Strobe => Box::new(Strobe::from_str(args)?),
        };

        Ok(res)
    }
}

pub fn pattern_with_args_from_command(input: &str) -> IResult<&str, (&str, &str)> {
    let (remainder, pattern_kind) = take_until("(")(input)?;

    // parse the args for pattern creation between the parentheses
    let (remainder, args) = delimited(
        tag("("),
        take_while(|c: char| c.is_alphanumeric() || c == ',' || c == '.'),
        tag(")"),
    )(remainder)?;

    Ok((remainder, (pattern_kind, args)))
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

    fn change(&mut self, command: char) -> anyhow::Result<()> {
        match command {
            'f' => self.faster(),
            's' => self.slower(),
            c => {
                let res = Self::try_from(c);
                if res.is_err() {
                    return Err(anyhow!("Invalid speed change parameter {}!", c));
                } else {
                    *self = res.unwrap();
                }
            }
        };

        Ok(())
    }
}

impl TryFrom<char> for PatternSpeed {
    type Error = anyhow::Error;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            '0' => Ok(PatternSpeed::N1),
            '1' => Ok(PatternSpeed::N2),
            '2' => Ok(PatternSpeed::N4),
            '3' => Ok(PatternSpeed::N8),
            '4' => Ok(PatternSpeed::N16),
            '5' => Ok(PatternSpeed::N32),
            _ => Err(anyhow!("Invalid PatternSpeed character {}", value)),
        }
    }
}

fn invalid_cmd(pattern_kind: &str, cmd: &str, help: &str) -> Result<()> {
    Err(anyhow!(
        "Invalid command {} for {}; Available commands are: {}",
        cmd,
        pattern_kind,
        help
    ))
}
