use esp_hal::{
    gpio::{Gpio25, Input},
    time::current_time,
};
use fugit::{Instant, MicrosDurationU64};

use crate::{SHARED, SHOOT_NOW_SIGNAL};

#[derive(Debug, Clone)]
pub struct TapInfo {
    pub last_time: Option<Instant<u64, 1, 1000000>>,
    pub interval: Option<u64>,
    pub is_stopped: bool,
    pub tap_series_count: u8,
    pub tap_series_start: Option<Instant<u64, 1, 1000000>>,
}

#[embassy_executor::task]
pub async fn button_press_handler(mut button: Input<'static, Gpio25>) {
    loop {
        log::info!("Waiting for button press...");
        button.wait_for_rising_edge().await;
        beat_input();
    }
}

pub fn beat_input() {
    // enter critical section
    critical_section::with(|cs| {
        let mut shared = SHARED.borrow_ref_mut(cs);

        let tap_info = &mut shared.tap_info;
        if tap_info.is_none() {
            tap_info.replace(TapInfo {
                last_time: None,
                interval: None,
                is_stopped: false,
                tap_series_count: 0,
                tap_series_start: None,
            });
        }

        // now, tap_info is definitely Some
        let tap_info = tap_info.as_mut().unwrap();
        tap_info.is_stopped = false;

        let old_time = tap_info.last_time;

        // measure time
        let current_time = current_time();

        // set last time in info
        tap_info.last_time = Some(current_time);
        if tap_info.tap_series_start.is_none() {
            tap_info.tap_series_start = Some(current_time);
        }

        // calc speed and set it
        if let Some(old_t) = old_time {
            let duration = MicrosDurationU64::from_ticks(current_time.ticks() - old_t.ticks());

            if duration.ticks() < 200_000 {
                // filter out weird triggers (less than 0.2 sec, which would be >300 bpm)
                log::info!("Ignoring duration: {:?} (too short)", duration);

                // reset to old_time assuming that this was a false positive
                tap_info.last_time = old_time;
            } else if duration.ticks() > 1_000_000 {
                // filter out weird triggers (more than 1 sec, which would be < 60 bpm)
                log::info!("Ignoring duration: {:?} (too long)", duration);
                tap_info.tap_series_start = Some(current_time);
                tap_info.tap_series_count = 0;
            } else {
                log::info!("New duration: {:?}", duration);

                tap_info.tap_series_count += 1;
                let series_duration = MicrosDurationU64::from_ticks(
                    current_time.ticks() - tap_info.tap_series_start.unwrap().ticks(),
                );

                // set new interval to be used in shoots
                tap_info.interval =
                    Some((series_duration / tap_info.tap_series_count as u32).ticks());
            }
        }
    });

    // signal the shooting task to stop waiting
    SHOOT_NOW_SIGNAL.signal(());
}
