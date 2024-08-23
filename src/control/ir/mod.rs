use alloc::{string::String, vec::Vec};
use esp_hal::{
    rmt::{asynch::RxChannelAsync, Channel, PulseCode},
    Async,
};
use infrared::{
    protocol::{nec::Nec16Command, Nec},
    Receiver,
};
use remotes::get_action_from_command;

use crate::RMT_CLOCK_DIVIDER;
use executor::ActionExecutor;

mod executor;
mod remotes;

enum Actions {
    Power,
    Up,
    Down,
    Sound,
    Next,
    Previous,
    Mute,
    VolUp,
    VolDown,
    Light1,
    Light2,
    LightMixed,
    Button1,
    Button2,
    Button3,
    Beat,
    Time1,
    Time2,
    Time3,
    Time4,
    Shift,
    Fade,
    Unknown,
}

fn coerce(value: u16) -> u32 {
    // we have to multiply with the clock divider of the channel
    // somehow, simply setting the divider to 1 would result in errors...
    return value as u32 * RMT_CLOCK_DIVIDER as u32;
}

#[embassy_executor::task]
pub async fn ir_receive(mut channel: Channel<Async, 1>) {
    let mut ir_receiver: Receiver<Nec<Nec16Command>> = infrared::Receiver::new(80_000_000);
    let mut action_executor = ActionExecutor::new();

    let mut data = [PulseCode {
        level1: true,
        length1: 1,
        level2: false,
        length2: 1,
    }; 48];

    loop {
        channel.receive(&mut data).await.unwrap();

        for entry in data {
            if entry.length1 == 0 {
                break;
            }

            let res = ir_receiver.event(coerce(entry.length1), entry.level1);

            if res.is_err() {
                log::info!("{:?}", res);
                break;
            }

            if let Some(cmd) = res.unwrap() {
                // execute command
                let action = get_action_from_command(&cmd);
                if let Some(action) = action {
                    let res = action_executor.execute(action, cmd.repeat);
                    if res.is_err() {
                        // TODO: give feedback to user (e.g. red led blinks)
                    }
                }
                break;
            }

            if entry.length2 == 0 {
                break;
            }

            let res = ir_receiver.event(coerce(entry.length2), entry.level2);

            if res.is_err() {
                log::info!("{:?}", res);
                break;
            }

            if let Some(cmd) = res.unwrap() {
                // execute command
                let action = get_action_from_command(&cmd);
                if let Some(action) = action {
                    let res = action_executor.execute(action, cmd.repeat);
                    if res.is_err() {
                        // TODO: give feedback to user (e.g. red led blinks)
                    }
                }
                break;
            }
        }
    }
}
