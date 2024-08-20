use crate::control::ir::Actions::{self, *};

use super::RemoteControlVariant;

/// Implementation for a remote control shipped with a star projector lamp
///

struct SpaceShipRemote {}

impl RemoteControlVariant for SpaceShipRemote {
    const ADDRESS: u16 = 0;

    fn cmd_to_action(self, cmd: u8) -> Actions {
        match cmd {
            31 => Power,
            0 => Up,
            8 => Down,
            2 => VolUp,
            10 => VolDown,
            5 => Previous,
            7 => Next,
            6 => Sound,
            4 => Shift, // LightMode
            11 => Beat,
            12 => Fade, // Fade
            14 => Light1,
            15 => Light2,
            13 => LightMixed,
            16 => Mute,
            17 => Button1,
            18 => Button2,
            19 => Button3,
            20 => Time1,
            21 => Time2,
            22 => Time3,
            23 => Time4,
            _ => todo!(),
        }
    }
}
