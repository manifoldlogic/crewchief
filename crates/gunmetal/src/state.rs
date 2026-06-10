//! GUN state clock — monotonic timestamps for conflict resolution.
//!
//! Mirrors the `./state` module from gun.js. The clock produces monotonically
//! increasing f64 timestamps based on wall clock time, with sub-millisecond
//! resolution achieved by incrementing a fractional counter when multiple
//! calls occur within the same millisecond.
//!
//! From the source:
//! ```js
//! function State(){
//!     var t = +new Date;
//!     if(last < t){ return N = 0, last = t + State.drift; }
//!     return last = t + ((N += 1) / D) + State.drift;
//! }
//! State.drift = 0;
//! var NI = -Infinity, N = 0, D = 999, last = NI;
//! ```

use std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

/// Divisor for sub-millisecond resolution. From the source:
/// > WARNING! In the future, on machines that are D times faster than 2016AD
/// > machines, you will want to increase D by another several orders of
/// > magnitude so the processing speed never out paces the decimal resolution.
const D: f64 = 999.0;

struct ClockInner {
    last: f64,
    n: f64,
    drift: f64,
}

/// A monotonic state clock for GUN's CRDT conflict resolution.
///
/// Produces timestamps that are:
/// - Based on wall clock time (milliseconds since Unix epoch)
/// - Monotonically increasing (never goes backwards)
/// - Sub-millisecond resolution via fractional counter when called rapidly
/// - Adjustable via drift for clock synchronization
pub struct State {
    inner: Mutex<ClockInner>,
}

impl State {
    /// Create a new state clock with zero drift.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(ClockInner {
                last: f64::NEG_INFINITY,
                n: 0.0,
                drift: 0.0,
            }),
        }
    }

    /// Create a new state clock with a specified drift offset.
    ///
    /// Drift is added to all timestamps, useful for synchronizing clocks
    /// between peers that have slightly different wall clock times.
    pub fn with_drift(drift: f64) -> Self {
        Self {
            inner: Mutex::new(ClockInner {
                last: f64::NEG_INFINITY,
                n: 0.0,
                drift,
            }),
        }
    }

    /// Get the current state timestamp.
    ///
    /// Returns a monotonically increasing f64 representing milliseconds
    /// since the Unix epoch, with sub-millisecond fractional resolution.
    pub fn now(&self) -> f64 {
        let t = now_ms();
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());

        if inner.last < t {
            inner.n = 0.0;
            inner.last = t + inner.drift;
        } else {
            inner.n += 1.0;
            // M9: cap n to prevent sub-ms fraction from exceeding 1.0
            // which would cause timestamp overlap with future milliseconds.
            if inner.n >= D {
                inner.n = 0.0;
                inner.last += 1.0; // advance by 1ms to maintain monotonicity
            } else {
                inner.last = t + (inner.n / D) + inner.drift;
            }
        }

        inner.last
    }

    /// Set the clock drift.
    pub fn set_drift(&self, drift: f64) {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).drift = drift;
    }

    /// Get the current drift value.
    pub fn drift(&self) -> f64 {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).drift
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current time in milliseconds since Unix epoch.
/// L4: handle pre-epoch clocks gracefully instead of panicking
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// Get current time in milliseconds since Unix epoch (WASM version).
#[cfg(target_arch = "wasm32")]
pub(crate) fn now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monotonically_increasing() {
        let clock = State::new();
        let mut prev = 0.0;
        for _ in 0..1000 {
            let t = clock.now();
            assert!(t > prev, "expected {} > {}", t, prev);
            prev = t;
        }
    }

    #[test]
    fn drift_applied() {
        let clock = State::with_drift(1000.0);
        let t = clock.now();
        let raw = now_ms();
        // The timestamp should be at least 1000ms ahead of raw time
        // (minus a small epsilon for the time between the two calls)
        assert!(t >= raw + 999.0, "expected {} >= {}", t, raw + 999.0);
    }

    #[test]
    fn sub_millisecond_resolution() {
        let clock = State::new();
        let t1 = clock.now();
        let t2 = clock.now();
        let t3 = clock.now();
        // Even if called in the same ms, each should be unique and increasing
        assert!(t2 > t1);
        assert!(t3 > t2);
        // The fractional part should be small (< 1ms)
        let diff = t2 - t1;
        assert!(diff < 1.0, "sub-ms diff should be < 1.0, got {}", diff);
    }
}
