use std::time::Instant;

use thiserror::Error;

/// An incoming message rate limit was exceeded.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
#[error("message rate limit exceeded")]
pub struct RateLimitError;

/// A simple and high-performance token-bucket rate limiter.
#[derive(Debug, Clone)]
pub(crate) struct TokenBucketRateLimiter {
    burst: f64,
    refill_rate: f64,
    tokens: f64,
    last_refill: Instant,
    enabled: bool,
}

impl TokenBucketRateLimiter {
    /// Creates a new token bucket rate limiter.
    ///
    /// If `burst` or `refill_per_sec` is 0, the limiter is disabled and will always allow messages.
    pub(crate) fn new(burst: u32, refill_per_sec: u32) -> Self {
        if burst == 0 || refill_per_sec == 0 {
            return Self {
                burst: 0.0,
                refill_rate: 0.0,
                tokens: 0.0,
                last_refill: Instant::now(),
                enabled: false,
            };
        }

        let burst_f = burst as f64;
        Self {
            burst: burst_f,
            refill_rate: refill_per_sec as f64,
            tokens: burst_f,
            last_refill: Instant::now(),
            enabled: true,
        }
    }

    /// Attempts to acquire a single token for an incoming message.
    ///
    /// Returns `true` if permitted, or `false` if the rate limit is exceeded.
    pub(crate) fn try_acquire(&mut self) -> bool {
        if !self.enabled {
            return true;
        }

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.last_refill = now;

        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.burst);

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Returns whether rate limiting is active.
    #[cfg(test)]
    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for TokenBucketRateLimiter {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        thread,
        time::Duration,
    };

    use super::*;

    #[test]
    fn default_limiter_is_disabled() {
        let mut limiter = TokenBucketRateLimiter::default();
        assert!(!limiter.is_enabled());
        assert!(limiter.try_acquire());
    }

    #[test]
    fn disabled_limiter_always_permits() {
        let mut limiter = TokenBucketRateLimiter::new(0, 0);
        for _ in 0..100 {
            assert!(limiter.try_acquire());
        }
    }

    #[test]
    fn burst_permits_up_to_limit_and_rejects() {
        let mut limiter = TokenBucketRateLimiter::new(3, 1);
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire());
    }

    #[test]
    fn tokens_refill_over_time() {
        let mut limiter = TokenBucketRateLimiter::new(2, 10);
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire());

        thread::sleep(Duration::from_millis(210));
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire());
    }
}
