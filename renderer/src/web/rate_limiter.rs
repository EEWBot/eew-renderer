use std::time::{Duration, Instant};

use moka::{
    ops::compute::{CompResult, Op},
    sync::Cache,
};

#[derive(Debug, Clone)]
pub struct ResponseRateLimiter {
    minimum_response_interval: Duration,
    last_respond_ats: Cache<[u8; 20], Instant>,
}

impl ResponseRateLimiter {
    pub fn new(minimum_response_interval: Duration) -> Self {
        Self {
            minimum_response_interval,
            last_respond_ats: Cache::builder()
                .time_to_live(Duration::from_secs(5))
                .build(),
        }
    }

    pub fn schedule(&self, sha1: [u8; 20], identity: &str) -> Instant {
        let now = Instant::now();

        let schedule_responce_at =
            self.last_respond_ats
                .entry(sha1)
                .and_compute_with(|maybe_entry| match maybe_entry {
                    Some(v) => {
                        if *v.value() + self.minimum_response_interval < now {
                            Op::Put(now)
                        } else {
                            let schedule_at = *v.value() + self.minimum_response_interval;

                            tracing::info!(
                                "Scheduled after {:?} ({identity})",
                                schedule_at.duration_since(now)
                            );

                            Op::Put(schedule_at)
                        }
                    }
                    None => Op::Put(now),
                });

        match schedule_responce_at {
            CompResult::Inserted(v) | CompResult::ReplacedWith(v) => *v.value(),
            CompResult::StillNone(_) | CompResult::Removed(_) | CompResult::Unchanged(_) => {
                unreachable!()
            }
        }
    }
}
