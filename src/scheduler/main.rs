use std::path::PathBuf;
use std::time::Duration;
use std::process::exit;


use time::get_time;

use time_util::ToMsec;
use hash::hash;
use watchdog::{Alarm, ExitOnReturn};
use shared::{Id, SharedState};
use scheduler::Schedule;


pub struct Settings {
    pub id: Id,
    pub hostname: String,
    pub config_dir: PathBuf,
}


pub fn main(state: SharedState, settings: Settings, mut alarm: Alarm) -> ! {
    let _guard = ExitOnReturn(92);
    let mut scheduler = {
        let _alarm = alarm.after(Duration::from_secs(10));
        match super::read(settings.id,
                          settings.hostname,
                          &settings.config_dir)
        {
            Ok(s) => s,
            Err(e) => {
                error!("Scheduler load failed: {}", e);
                exit(4);
            }
        }
    };
    loop {
        state.wait_schedule_update(Duration::from_secs(5));

        // TODO(tailhook) check if peers are outdated
        if let Some(peers) = state.peers() {
            let cfg = state.config();
            let timestamp = get_time();
            let _alarm = alarm.after(Duration::from_secs(1));
            let scheduler_result = match scheduler.execute(&*cfg, &peers.1) {
                Ok(j) => j,
                Err(e) => {
                    error!("Scheduling failed: {}", e);
                    continue;
                }
            };

            let hash = hash(scheduler_result.to_string());
            if scheduler.previous_schedule_hash.as_ref() == Some(&hash) {
                debug!("Config did not change ({})", hash);
                continue;
            }
            info!("Got scheduling of {}: {}", hash, scheduler_result);

            scheduler.previous_schedule_hash = Some(hash.clone());
            state.set_schedule_by_leader(Schedule {
                timestamp: timestamp.to_msec(),
                hash: hash,
                data: scheduler_result,
                origin: scheduler.id.clone(),
            });
        } else {
            warn!("No peers data, don't try to rebuild config");
        }
    }
}
