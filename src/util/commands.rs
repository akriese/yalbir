use crate::{beat::tapping::beat_input, patterns::PatternCommand, SHARED};

fn change_speed(factor: f32) {
    critical_section::with(|cs| {
        let mut shared = SHARED.borrow_ref_mut(cs);
        let tap_info = shared.tap_info.as_mut().unwrap();
        tap_info.interval = Some((tap_info.interval.unwrap() as f32 * 1f32 / factor) as u64);
    });
}

pub fn handle_wireless_input(request: &str) -> anyhow::Result<()> {
    match request {
        "beat" => beat_input(),
        "half" => change_speed(0.5),
        "double" => change_speed(2.0),
        "stop" => critical_section::with(|cs| {
            let mut shared = SHARED.borrow_ref_mut(cs);
            shared.tap_info.as_mut().unwrap().is_stopped = true;
        }),
        cmd => critical_section::with(|cs| {
            SHARED
                .borrow_ref_mut(cs)
                .rgbs
                .as_mut()
                .unwrap()
                .execute_command(cmd)
        })?,
    }

    Ok(())
}
