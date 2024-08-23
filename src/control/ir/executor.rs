use alloc::{format, string::String};
use anyhow::anyhow;

use crate::{beat::tapping::beat_input, patterns::PatternCommand, SHARED};

use super::Actions;

pub struct ActionExecutor {
    selected_patterns: [bool; 3],
    selected_effect: Option<String>,
    is_beat_reacting: [bool; 3],
    is_rendering: [bool; 3],
    pattern_kind_index: [usize; 3],
}

impl ActionExecutor {
    pub fn execute(&mut self, action: Actions, repeat: bool) -> anyhow::Result<()> {
        // requirements of a remote control:
        // - toggle beat reaction / rendering of specific patterns / all patterns
        // - select parameters of patterns and change them (e.g. increase, decrease, rotate)
        // - change the pattern of a specific slot to a different pattern

        match action {
            Actions::Power => {
                // turn on/off selected pattern or all patterns
                critical_section::with(|cs| {
                    let mut shared = SHARED.borrow_ref_mut(cs);
                    let rgbs = shared.rgbs.as_mut().unwrap();

                    // if no patterns are selected, applies function to all of them
                    let apply_to_all = !self.selected_patterns.iter().any(|id| *id);

                    for (i, selected) in self.selected_patterns.iter().enumerate() {
                        if *selected || apply_to_all {
                            self.is_rendering[i] = !self.is_rendering[i];
                            let cmd = if self.is_rendering[i] { "S" } else { "s" };
                            rgbs.execute_command(&format!("p{}{}", i, cmd)[..]);
                        }
                    }
                })
            }
            Actions::Up => {
                // increase value
            }
            Actions::Down => {
                // decrease value
            }
            Actions::Sound => {
                // change beat reaction
            }
            Actions::Next => {
                // cycle new patterns
                // only if only exactly one pattern is selected
                if self
                    .selected_patterns
                    .iter()
                    .map(|b| if *b { 1 } else { 0 })
                    .sum::<i32>()
                    == 1
                {
                    let selected_idx = self.selected_patterns.iter().position(|id| *id).unwrap();

                    let pattern_init_cmds: [&str; 4] = [
                        &"br(10,m,200,2)",
                        &"str(10,s,0)",
                        &"shst(44,200,400)",
                        &"cat()",
                    ];
                    self.pattern_kind_index[selected_idx] += 1;
                    self.pattern_kind_index[selected_idx] =
                        self.pattern_kind_index[selected_idx] % 4;

                    critical_section::with(|cs| {
                        let mut shared = SHARED.borrow_ref_mut(cs);
                        let rgbs = shared.rgbs.as_mut().unwrap();

                        rgbs.execute_command(
                            &format!(
                                "p{}C{}",
                                selected_idx,
                                pattern_init_cmds[self.pattern_kind_index[selected_idx]]
                            )[..],
                        );
                    });
                }
            }
            Actions::Previous => {
                // cycle selected effects
            }
            Actions::Mute => {
                // start/stop beat reaction for selected pattern(s)
                critical_section::with(|cs| {
                    let mut shared = SHARED.borrow_ref_mut(cs);
                    let rgbs = shared.rgbs.as_mut().unwrap();

                    // if no patterns are selected, applies function to all of them
                    let apply_to_all = !self.selected_patterns.iter().any(|id| *id);

                    for (i, selected) in self.selected_patterns.iter().enumerate() {
                        if *selected || apply_to_all {
                            self.is_beat_reacting[i] = !self.is_beat_reacting[i];
                            let cmd = if self.is_beat_reacting[i] { "B" } else { "b" };
                            rgbs.execute_command(&format!("p{}{}", i, cmd)[..]);
                        }
                    }
                })
            }
            Actions::VolUp => {
                // increase smth
            }
            Actions::VolDown => {
                // decrease smth
            }
            Actions::Light1 => todo!(),
            Actions::Light2 => todo!(),
            Actions::LightMixed => {
                // select/unselect all patterns
                // only unselect if all patterns are currently selected
                if self.selected_patterns.iter().all(|id| *id) {
                    self.selected_patterns.iter_mut().for_each(|b| *b = false);
                } else {
                    self.selected_patterns.iter_mut().for_each(|b| *b = true);
                }
            }
            Actions::Button1 => {
                // select pattern 1
                self.selected_patterns[0] = !self.selected_patterns[0];
            }
            Actions::Button2 => {
                // select pattern 2
                self.selected_patterns[1] = !self.selected_patterns[1];
            }
            Actions::Button3 => {
                // select pattern 3
                self.selected_patterns[2] = !self.selected_patterns[2];
            }
            Actions::Beat => {
                // beat input
                if !repeat {
                    beat_input();
                }
            }
            Actions::Time1 => {
                // beat reaction quarters
            }
            Actions::Time2 => {
                // beat reaction 8ths
            }
            Actions::Time3 => {
                // beat reaction 16ths
            }
            Actions::Time4 => {
                // beat reaction 32ths
            }
            Actions::Shift => {
                //
            }
            Actions::Fade => {
                // unselect effect and then pattern selection
            }
            Actions::Unknown => return Err(anyhow!("Unknown action!")),
        }
        Ok(())
    }

    pub fn new() -> Self {
        Self {
            selected_patterns: [false; 3],
            selected_effect: None,
            is_beat_reacting: [true; 3],
            is_rendering: [true; 3],
            pattern_kind_index: [0; 3],
        }
    }
}
