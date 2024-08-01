use alloc::{boxed::Box, vec, vec::Vec};

use crate::{beat::BeatCount, util::color::Rgb};

use super::{LedPattern, PatternCommand};

struct Background {
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
        todo!()
    }
}

impl PatternCommand for Background {
    fn execute_command(&mut self, command: &str) -> anyhow::Result<()> {
        // for now, simply forward the command
        self.pattern.execute_command(command)
    }
}
