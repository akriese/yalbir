use super::Actions;

pub mod spaceship;

trait RemoteControlVariant {
    const ADDRESS: u16;

    fn cmd_to_action(self, cmd: u8) -> Actions;
}
