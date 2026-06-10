//! Exponential backoff with jitter for WebSocket reconnection.
//!
//! Pure Rust — no platform-specific dependencies. Used by both
//! native and WASM WebSocket implementations.
//!
//! ## Algorithm
//!
//! `delay = min(max_delay, base_delay * 2^attempt) + jitter`
//!
//! Jitter is added to prevent thundering herd when many clients
//! reconnect simultaneously after a server restart.

use std::time::Duration;

/// Configuration for reconnection backoff.
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Base delay before first retry. Default: 1 second.
    pub base_delay: Duration,
    /// Maximum delay between retries. Default: 30 seconds.
    pub max_delay: Duration,
    /// Maximum jitter to add (0.0–1.0 fraction of computed delay). Default: 0.25.
    pub jitter_fraction: f64,
    /// Maximum number of reconnection attempts (None = unlimited). Default: None.
    pub max_attempts: Option<u32>,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            jitter_fraction: 0.25,
            max_attempts: None,
        }
    }
}

/// Tracks reconnection state and computes backoff delays.
#[derive(Debug)]
pub struct ReconnectState {
    config: ReconnectConfig,
    attempt: u32,
}

impl ReconnectState {
    /// Create a new reconnection state.
    pub fn new(config: ReconnectConfig) -> Self {
        Self { config, attempt: 0 }
    }

    /// Create with default config.
    pub fn default_config() -> Self {
        Self::new(ReconnectConfig::default())
    }

    /// Get the next backoff delay and increment the attempt counter.
    ///
    /// Returns `None` if max attempts has been reached.
    pub fn next_delay(&mut self) -> Option<Duration> {
        if let Some(max) = self.config.max_attempts {
            if self.attempt >= max {
                return None;
            }
        }

        let delay = self.compute_delay();
        self.attempt += 1;
        Some(delay)
    }

    /// Peek at the next delay without incrementing the counter.
    pub fn peek_delay(&self) -> Option<Duration> {
        if let Some(max) = self.config.max_attempts {
            if self.attempt >= max {
                return None;
            }
        }
        Some(self.compute_delay())
    }

    /// Reset the backoff state (e.g., after a successful connection).
    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    /// Current attempt number (0-indexed).
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Whether max attempts has been reached.
    pub fn exhausted(&self) -> bool {
        self.config
            .max_attempts
            .is_some_and(|max| self.attempt >= max)
    }

    /// Get the config.
    pub fn config(&self) -> &ReconnectConfig {
        &self.config
    }

    fn compute_delay(&self) -> Duration {
        let base_ms = self.config.base_delay.as_millis() as f64;
        let max_ms = self.config.max_delay.as_millis() as f64;

        // Exponential: base * 2^attempt, capped at max
        let exp_ms = (base_ms * 2.0f64.powi(self.attempt as i32)).min(max_ms);

        // Add deterministic jitter based on attempt number.
        // Real jitter would use OsRng, but for predictability in the core
        // algorithm we use a simple hash. Callers can add true randomness.
        let jitter_ms = exp_ms * self.config.jitter_fraction * self.jitter_factor();

        Duration::from_millis((exp_ms + jitter_ms) as u64)
    }

    /// Deterministic jitter factor (0.0–1.0) based on attempt number.
    /// Uses a simple hash to distribute values pseudo-randomly.
    fn jitter_factor(&self) -> f64 {
        // Simple multiplicative hash for deterministic but varied jitter
        let hash = (self.attempt as u64).wrapping_mul(2654435761) % 1000;
        hash as f64 / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = ReconnectConfig::default();
        assert_eq!(config.base_delay, Duration::from_secs(1));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert_eq!(config.jitter_fraction, 0.25);
        assert!(config.max_attempts.is_none());
    }

    #[test]
    fn exponential_backoff() {
        let config = ReconnectConfig {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            jitter_fraction: 0.0, // no jitter for predictable testing
            max_attempts: None,
        };
        let mut state = ReconnectState::new(config);

        // Attempt 0: 100ms * 2^0 = 100ms
        let d0 = state.next_delay().unwrap();
        assert_eq!(d0, Duration::from_millis(100));

        // Attempt 1: 100ms * 2^1 = 200ms
        let d1 = state.next_delay().unwrap();
        assert_eq!(d1, Duration::from_millis(200));

        // Attempt 2: 100ms * 2^2 = 400ms
        let d2 = state.next_delay().unwrap();
        assert_eq!(d2, Duration::from_millis(400));

        // Attempt 3: 100ms * 2^3 = 800ms
        let d3 = state.next_delay().unwrap();
        assert_eq!(d3, Duration::from_millis(800));
    }

    #[test]
    fn max_delay_cap() {
        let config = ReconnectConfig {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            jitter_fraction: 0.0,
            max_attempts: None,
        };
        let mut state = ReconnectState::new(config);

        // Skip ahead through several attempts
        for _ in 0..10 {
            let d = state.next_delay().unwrap();
            assert!(d <= Duration::from_secs(5), "delay {:?} exceeds max", d);
        }

        // At attempt 10, delay should be capped at 5s
        let d = state.next_delay().unwrap();
        assert_eq!(d, Duration::from_secs(5));
    }

    #[test]
    fn max_attempts_exhaustion() {
        let config = ReconnectConfig {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            jitter_fraction: 0.0,
            max_attempts: Some(3),
        };
        let mut state = ReconnectState::new(config);

        assert!(!state.exhausted());
        assert!(state.next_delay().is_some()); // 0
        assert!(state.next_delay().is_some()); // 1
        assert!(state.next_delay().is_some()); // 2
        assert!(state.exhausted());
        assert!(state.next_delay().is_none()); // exhausted
    }

    #[test]
    fn reset_clears_attempts() {
        let config = ReconnectConfig {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            jitter_fraction: 0.0,
            max_attempts: Some(3),
        };
        let mut state = ReconnectState::new(config);

        state.next_delay();
        state.next_delay();
        assert_eq!(state.attempt(), 2);

        state.reset();
        assert_eq!(state.attempt(), 0);
        assert!(!state.exhausted());
    }

    #[test]
    fn jitter_adds_variance() {
        let config = ReconnectConfig {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            jitter_fraction: 0.25,
            max_attempts: None,
        };
        let mut state = ReconnectState::new(config);

        let d0 = state.next_delay().unwrap();
        // With jitter, delay should be >= base but not more than base * 1.25
        assert!(d0 >= Duration::from_millis(1000));
        assert!(d0 <= Duration::from_millis(1250));
    }

    #[test]
    fn peek_does_not_advance() {
        let config = ReconnectConfig {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            jitter_fraction: 0.0,
            max_attempts: None,
        };
        let state = ReconnectState::new(config);

        let peek1 = state.peek_delay().unwrap();
        let peek2 = state.peek_delay().unwrap();
        assert_eq!(peek1, peek2); // same — not advanced
        assert_eq!(state.attempt(), 0);
    }

    #[test]
    fn delays_are_monotonically_increasing() {
        let config = ReconnectConfig {
            base_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(30),
            jitter_fraction: 0.0,
            max_attempts: None,
        };
        let mut state = ReconnectState::new(config);

        let mut prev = Duration::ZERO;
        for _ in 0..10 {
            let d = state.next_delay().unwrap();
            assert!(d >= prev, "delay {:?} < previous {:?}", d, prev);
            prev = d;
        }
    }
}
