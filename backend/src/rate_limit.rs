//! Tiny in-memory sliding-window rate limiter (used for `POST /api/auth/login`).
//! Good enough for a single instance; use a shared store (e.g. Redis) when scaling out.

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
    max: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max: usize, window: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max,
            window,
        }
    }

    /// Record an attempt for `key`. Returns `false` when the limit is exceeded.
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut map = self.inner.lock().unwrap_or_else(|p| p.into_inner());

        // Opportunistic cleanup so the map does not grow forever.
        if map.len() > 10_000 {
            map.retain(|_, q| {
                q.back()
                    .is_some_and(|t| now.duration_since(*t) < self.window)
            });
        }

        let attempts = map.entry(key.to_string()).or_default();
        while attempts
            .front()
            .is_some_and(|t| now.duration_since(*t) >= self.window)
        {
            attempts.pop_front();
        }
        if attempts.len() >= self.max {
            return false;
        }
        attempts.push_back(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_after_max_attempts() {
        let rl = RateLimiter::new(5, Duration::from_secs(60));
        for _ in 0..5 {
            assert!(rl.check("1.2.3.4"));
        }
        assert!(!rl.check("1.2.3.4"));
        assert!(rl.check("5.6.7.8"));
    }
}
