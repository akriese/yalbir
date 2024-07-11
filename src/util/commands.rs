use crate::{beat_button, change_speed, SHARED};

pub fn handle_wireless_input(request: &str) {
    match request {
        "beat" => beat_button(),
        "half" => change_speed(0.5),
        "double" => change_speed(2.0),
        "stop" => critical_section::with(|cs| {
            let mut shared = SHARED.borrow_ref_mut(cs);
            shared.tap_info.as_mut().unwrap().is_stopped = true;
        }),
        _ => (),
    }

    if request.starts_with('m') {}
}
