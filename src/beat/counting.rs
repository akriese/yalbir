use embassy_futures::select::select;
use embassy_time::Timer;
use esp_hal::time::current_time;
use fugit::Instant;

use crate::{rgbs_issue_beat, LAST_SHOT, SHARED, SHOOT_NOW_SIGNAL};

#[embassy_executor::task]
pub async fn beat_executor() {
    let mut is_repeating = false;
    let mut interval = 0;
    let mut last_loop_process_time = 0;

    loop {
        let input_signal = SHOOT_NOW_SIGNAL.wait();
        if is_repeating {
            select(
                input_signal,
                Timer::after_micros(interval - last_loop_process_time),
            )
            .await;
        } else {
            // wait for the first and second beat input to be triggered
            input_signal.await;
        }

        let process_start_time = current_time();

        critical_section::with(|cs| {
            let mut shared = SHARED.borrow_ref_mut(cs);
            let tap_info = shared.tap_info.as_mut();
            if let Some(info) = tap_info {
                if info.is_stopped {
                    is_repeating = false;
                } else if let Some(interv) = info.interval {
                    interval = interv;
                    is_repeating = true;
                }
            }
        });

        let last_shot = critical_section::with(|cs| {
            LAST_SHOT
                .borrow_ref_mut(cs)
                .unwrap_or(Instant::<u64, 1, 1000000>::from_ticks(0))
        });
        let current_t = current_time();
        log::info!("Shoot triggered! {:?}", current_t - last_shot);
        critical_section::with(|cs| LAST_SHOT.borrow_ref_mut(cs).replace(current_t));

        rgbs_issue_beat();

        last_loop_process_time = (current_time() - process_start_time).to_micros();
    }
}
