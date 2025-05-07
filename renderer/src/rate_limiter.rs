use std::time::{Duration, Instant};

use moka::{
    ops::compute::{CompResult, Op},
    sync::{Cache, CacheBuilder},
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
            last_respond_ats: CacheBuilder::new(512)
                .time_to_live(Duration::from_secs(10))
                .build(),
        }
    }

    pub fn schedule(&self, sha1: [u8; 20]) -> Instant {
        let schedule_responce_at =
            self.last_respond_ats
                .entry(sha1)
                .and_compute_with(|maybe_entry| match maybe_entry {
                    Some(v) => {
                        let now = Instant::now();

                        if *v.value() + self.minimum_response_interval < now {
                            Op::Put(Instant::now())
                        } else {
                            Op::Put(*v.value() + self.minimum_response_interval)
                        }
                    }
                    None => Op::Put(Instant::now()),
                });

        match schedule_responce_at {
            CompResult::Inserted(v) | CompResult::ReplacedWith(v) => *v.value(),
            CompResult::StillNone(_) | CompResult::Removed(_) | CompResult::Unchanged(_) => {
                unreachable!()
            }
        }
    }
}
