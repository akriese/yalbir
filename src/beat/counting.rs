use core::cell::RefCell;

use critical_section::Mutex;
use embassy_futures::select::{select, Either};
use embassy_time::Timer;
use esp_hal::time::current_time;
use fugit::Instant;

use crate::{rgbs_issue_beat, SHARED};

use super::{BeatCount, SHOOT_NOW_SIGNAL};

static LAST_SHOT: Mutex<RefCell<Option<Instant<u64, 1, 1000000>>>> = Mutex::new(RefCell::new(None));
const MIN_TIME_PIECE: usize = 32;

#[embassy_executor::task]
pub async fn beat_executor() {
    let mut is_repeating = false;
    let mut interval = 0;
    let mut last_loop_process_time = 0;

    let mut beat_count = BeatCount::default();

    loop {
        let input_signal = SHOOT_NOW_SIGNAL.wait();
        let mut is_signaled = false;
        if is_repeating {
            let either = select(
                input_signal,
                Timer::after_micros(interval - last_loop_process_time),
            )
            .await;

            if let Either::First(_) = either {
                is_signaled = true;
            };
        } else {
            // wait for the first and second beat input to be triggered
            input_signal.await;
            is_signaled = true;
        }

        let process_start_time = current_time();

        // either start at 1 or increment the counting measure
        if is_signaled {
            beat_count = BeatCount::default();
        } else {
            beat_count.increment();
        }

        critical_section::with(|cs| {
            let mut shared = SHARED.borrow_ref_mut(cs);
            let tap_info = shared.tap_info.as_mut();
            if let Some(info) = tap_info {
                if info.is_stopped {
                    is_repeating = false;
                } else if let Some(interv) = info.interval {
                    // the tapping interval is expected as quarters of a bar
                    // thus we wait for the smallest time piece used in the system
                    // instead of waiting for quarters
                    interval = interv / (MIN_TIME_PIECE / 4) as u64;
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

        rgbs_issue_beat(&beat_count);

        last_loop_process_time = (current_time() - process_start_time).to_micros();
    }
}
