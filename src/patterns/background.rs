use alloc::{boxed::Box, vec, vec::Vec};
use anyhow::anyhow;
use nom::{bytes::complete::tag, sequence::pair};

use crate::{beat::BeatCount, color::Rgb};

use super::{
    command::hex_rgb, pattern_with_args_from_command, LedPattern, PatternCommand, PatternKind,
};

pub struct Background {
    rgbs: Vec<Rgb>,
    color: Rgb,
    pattern: Box<dyn LedPattern>,
}

impl Background {
    fn new(pattern: Box<dyn LedPattern>, color: Rgb) -> Self {
        Self {
            rgbs: vec![Rgb::default(); pattern.size()],
            color,
            pattern,
        }
    }
}

impl LedPattern for Background {
    fn next(&mut self) -> &[Rgb] {
        self.rgbs.copy_from_slice(self.pattern.next());

        for rgb in self.rgbs.iter_mut() {
            if *rgb == Rgb::default() {
                *rgb = self.color;
            }
        }

        &self.rgbs
    }

    fn beat(&mut self, beat_info: &BeatCount) {
        self.pattern.beat(beat_info);
    }

    fn size(&self) -> usize {
        self.rgbs.len()
    }

    fn from_str(args: &str) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        // fn new(pattern: Box<dyn LedPattern>, color: Rgb) -> Self {
        let (remainder, (pattern_kind, args)) = pattern_with_args_from_command(args).map_err(
            |_: nom::Err<nom::error::Error<&str>>| anyhow!("Could not parse pattern and args!"),
        )?;

        // create the pattern with the given args
        let pattern: Box<dyn LedPattern> = PatternKind::try_from(pattern_kind)?.to_pattern(args)?;

        let (remainder, _) =
            tag(",")(remainder).map_err(|_err: nom::Err<nom::error::Error<&str>>| {
                anyhow!("Missing comma after background pattern args")
            })?;

        let (_, background) =
            hex_rgb(remainder).map_err(|err: nom::Err<nom::error::Error<&str>>| {
                if let nom::Err::Error(error) = err {
                    if let nom::error::ErrorKind::Tag = error.code {
                        anyhow!("Invalid hex representation (Missing '#' sign?)")
                    } else {
                        anyhow!("Invalid hex numbers")
                    }
                } else {
                    anyhow!("Unknown error while parsing hex: {:?}", err)
                }
            })?;

        Ok(Self::new(pattern, background))
    }
}

impl PatternCommand for Background {
    fn execute_command(&mut self, command: &str) -> anyhow::Result<()> {
        // for now, simply forward the command
        self.pattern.execute_command(command)
    }
}
