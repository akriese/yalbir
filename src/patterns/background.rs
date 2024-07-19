use alloc::{boxed::Box, vec::Vec};

use crate::{beat::BeatCount, util::color::Rgb};

use super::{LedPattern, PatternCommand};

struct Background {
    rgbs: Vec<Rgb>,
    color: Rgb,
    pattern: Box<dyn LedPattern>,
}

impl Background {
    fn new();
}

impl LedPattern for Background {
    fn next(&mut self) -> &[Rgb] {
        self.pattern.next()
    }

    fn beat(&mut self, beat_info: &BeatCount) {
        todo!()
    }

    fn size(&self) -> usize {
        self.rgbs.len()
    }
}

impl PatternCommand for Background {
    fn execute_command(&mut self, command: &str) -> Result<(), ()> {
        todo!()
    }
}
