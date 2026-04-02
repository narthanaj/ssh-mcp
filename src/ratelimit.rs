use std::time::Instant;

use dashmap::DashMap;

use crate::error::SshMcpError;

pub struct RateLimiter {
    buckets: DashMap<String, TokenBucket>,
    max_tokens: u32,
    refill_rate: f64, // tokens per second
}

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a rate limiter allowing `per_minute` calls per minute per session.
    pub fn new(per_minute: u32) -> Self {
        Self {
            buckets: DashMap::new(),
            max_tokens: per_minute,
            refill_rate: f64::from(per_minute) / 60.0,
        }
    }

    /// Check if a request is allowed for the given session. Returns Ok(()) if
    /// allowed, or Err(RateLimited) if the session has exceeded its limit.
    pub fn check(&self, session_id: &str) -> Result<(), SshMcpError> {
        let mut bucket = self
            .buckets
            .entry(session_id.to_string())
            .or_insert_with(|| TokenBucket {
                tokens: f64::from(self.max_tokens),
                last_refill: Instant::now(),
            });

        let now = Instant::now();
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens =
            (bucket.tokens + elapsed * self.refill_rate).min(f64::from(self.max_tokens));
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            Err(SshMcpError::RateLimited {
                session_id: session_id.to_string(),
            })
        }
    }

    pub fn remove_session(&self, session_id: &str) {
        self.buckets.remove(session_id);
    }
}
