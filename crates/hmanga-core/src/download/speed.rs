use std::time::{Duration, Instant};

/// SpeedTracker computes rolling window download speed in bytes/sec.
#[derive(Debug)]
pub struct SpeedTracker {
    window_secs: f64,
    samples: Vec<(Instant, u64)>, // (timestamp, bytes_at_timestamp)
}

impl SpeedTracker {
    pub fn new(window_secs: f64) -> Self {
        Self {
            window_secs,
            samples: Vec::new(),
        }
    }

    /// Record that `bytes` were downloaded at the current time.
    pub fn add_sample(&mut self, bytes: u64) {
        let now = Instant::now();
        self.samples.push((now, bytes));
        self.prune(now);
    }

    /// Return estimated bytes/sec over the rolling window.
    pub fn speed_bps(&mut self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }
        let now = Instant::now();
        self.prune(now);

        if self.samples.len() < 2 {
            return 0.0;
        }

        let (t0, b0) = self.samples.first().unwrap();
        let (t1, b1) = self.samples.last().unwrap();

        let elapsed = t1.duration_since(*t0).as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }

        (b1 - b0) as f64 / elapsed
    }

    fn prune(&mut self, now: Instant) {
        let cutoff = now - Duration::from_secs_f64(self.window_secs);
        self.samples.retain(|(t, _)| *t >= cutoff);
    }
}

impl Default for SpeedTracker {
    fn default() -> Self {
        Self::new(5.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn speed_tracker_returns_zero_with_one_sample() {
        let mut tracker = SpeedTracker::new(5.0);
        tracker.add_sample(100);
        assert_eq!(tracker.speed_bps(), 0.0);
    }
}
