extern crate alloc;

use alloc::boxed::Box;

use crate::{util::color::Rgb, N_LEDS};

pub mod breathing;
pub mod shooting_star;
pub mod strobe;

pub trait LedPattern: Send + Sync {
    fn next(&mut self) -> &[Rgb];

    fn beat(&mut self);
}

pub trait PatternCommand {
    fn execute_command(&mut self, command: &str);
}

struct PatternSection {
    range: (usize, usize),
    pattern: Box<dyn LedPattern>,
}

pub struct PartitionedPatterns {
    rgbs: [Rgb; N_LEDS],
    patterns: [Option<PatternSection>; 10],
}

impl PartitionedPatterns {
    pub const fn new() -> Self {
        Self {
            rgbs: [Rgb { r: 0, g: 0, b: 0 }; N_LEDS],
            patterns: [None, None, None, None, None, None, None, None, None, None],
        }
    }

    pub fn add(&mut self, pattern: Box<dyn LedPattern>, range: (usize, usize)) {
        for p in self.patterns.iter_mut() {
            if p.is_some() {
                continue;
            }

            p.replace(PatternSection { pattern, range });
            break;
        }
    }
}

impl LedPattern for PartitionedPatterns {
    fn next(&mut self) -> &[Rgb] {
        for ps in self.patterns.iter_mut() {
            if let Some(section) = ps.as_mut() {
                let rgbs = section.pattern.as_mut().next();
                let (a, b) = (section.range.0, section.range.1);
                self.rgbs[a..b].copy_from_slice(&rgbs[..b - a]);
            }
        }
        &self.rgbs
    }

    fn beat(&mut self) {
        for ps in self.patterns.iter_mut() {
            if let Some(section) = ps.as_mut() {
                section.pattern.beat();
            }
        }
    }
}
