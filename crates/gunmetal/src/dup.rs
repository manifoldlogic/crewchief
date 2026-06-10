//! DUP — Message deduplication.
//!
//! Prevents infinite message loops in the P2P mesh by tracking recently seen
//! message IDs. Each ID has a TTL; once expired, it's cleaned up.
//!
//! From the source (`./dup`):
//! ```js
//! function Dup(opt){
//!     var dup = {s:{}}, s = dup.s;
//!     opt = opt || {max: 999, age: 1000 * 9};
//!     dup.check = function(id){ ... }
//!     dup.track = function(id){ ... }
//!     dup.drop = function(age){ ... }
//! }
//! ```
//!
//! For DAM (mesh routing) each entry can also record the peer the message
//! was first heard from (`via`), enabling direct ACK routing: when an ACK
//! references a message ID, the mesh traces the original sender through the
//! dedup table instead of broadcasting (see `mesh.rs`).
//!
//! Timestamps use `state::now_ms()` (wall-clock f64 ms) instead of
//! `std::time::Instant`, which is unavailable on `wasm32-unknown-unknown`.

use std::collections::HashMap;

use crate::state::now_ms;

/// Configuration for the deduplication tracker.
#[derive(Debug, Clone)]
pub struct DupConfig {
    /// Maximum age in milliseconds before an entry expires.
    /// Default: 9000 (9 seconds), matching GUN's `1000 * 9`.
    pub age_ms: u64,

    /// Maximum entries before forcing a cleanup.
    /// Default: 999, matching GUN's default.
    pub max: usize,
}

impl Default for DupConfig {
    fn default() -> Self {
        Self {
            age_ms: 9_000,
            max: 999,
        }
    }
}

/// A tracked message entry.
#[derive(Debug, Clone)]
struct DupEntry {
    /// Wall-clock timestamp (ms) when the ID was last tracked.
    when_ms: f64,
    /// The peer this message was first heard from (DAM ACK routing).
    via: Option<String>,
}

/// A message deduplication tracker.
///
/// Tracks message IDs with timestamps, expiring entries older than `age_ms`.
/// This prevents messages from bouncing endlessly between peers.
pub struct Dup {
    config: DupConfig,
    seen: HashMap<String, DupEntry>,
}

impl Dup {
    /// Create a new deduplication tracker with default config.
    pub fn new() -> Self {
        Self {
            config: DupConfig::default(),
            seen: HashMap::new(),
        }
    }

    /// Create a new deduplication tracker with custom config.
    pub fn with_config(config: DupConfig) -> Self {
        Self {
            config,
            seen: HashMap::new(),
        }
    }

    /// Check if a message ID has been seen recently.
    ///
    /// Returns `true` if the ID is a duplicate (already tracked and not expired).
    /// Returns `false` if unseen or expired.
    pub fn check(&self, id: &str) -> bool {
        if let Some(entry) = self.seen.get(id) {
            now_ms() - entry.when_ms < self.config.age_ms as f64
        } else {
            false
        }
    }

    /// Track a message ID. Returns `true` if it was already tracked (duplicate).
    ///
    /// If the ID is new, it's added with the current timestamp.
    /// If the ID exists, its timestamp is refreshed (the `via` peer is kept).
    /// Triggers cleanup if over capacity.
    pub fn track(&mut self, id: impl Into<String>) -> bool {
        self.track_from(id, None)
    }

    /// Track a message ID, recording the peer it was heard from.
    ///
    /// The first non-`None` `via` wins; refreshes keep the original sender
    /// so ACK routing always targets the peer that introduced the message.
    pub fn track_from(&mut self, id: impl Into<String>, via: Option<String>) -> bool {
        let id = id.into();
        let was_dup = self.check(&id);
        let now = now_ms();
        self.seen
            .entry(id)
            .and_modify(|e| {
                e.when_ms = now;
                if e.via.is_none() {
                    e.via = via.clone();
                }
            })
            .or_insert(DupEntry { when_ms: now, via });

        if self.seen.len() > self.config.max {
            self.drop_expired();
        }

        was_dup
    }

    /// The peer a tracked message was first heard from, if recorded.
    ///
    /// Used by the mesh for direct ACK routing (`mesh.say` with `@`).
    pub fn via(&self, id: &str) -> Option<&str> {
        self.seen.get(id).and_then(|e| e.via.as_deref())
    }

    /// Remove expired entries. If still over capacity, evict the oldest.
    pub fn drop_expired(&mut self) {
        let age = self.config.age_ms as f64;
        let now = now_ms();
        self.seen.retain(|_, e| now - e.when_ms < age);

        // If still over max after removing expired, evict oldest entries
        // to prevent unbounded growth from high-frequency unique IDs.
        if self.seen.len() > self.config.max {
            let mut entries: Vec<(String, f64)> = self
                .seen
                .iter()
                .map(|(k, e)| (k.clone(), e.when_ms))
                .collect();
            entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let to_remove = self.seen.len() - self.config.max;
            for (key, _) in entries.into_iter().take(to_remove) {
                self.seen.remove(&key);
            }
        }
    }

    /// Number of currently tracked IDs.
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// Returns true if no IDs are tracked.
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

impl Default for Dup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn new_id_not_duplicate() {
        let dup = Dup::new();
        assert!(!dup.check("msg1"));
    }

    #[test]
    fn tracked_id_is_duplicate() {
        let mut dup = Dup::new();
        assert!(!dup.track("msg1"));  // first time → not dup
        assert!(dup.track("msg1"));   // second time → dup
        assert!(dup.check("msg1"));   // still tracked
    }

    #[test]
    fn different_ids_independent() {
        let mut dup = Dup::new();
        dup.track("msg1");
        assert!(!dup.check("msg2"));
        assert!(dup.check("msg1"));
    }

    #[test]
    fn expired_entries_removed() {
        let mut dup = Dup::with_config(DupConfig {
            age_ms: 50,  // very short TTL for testing
            max: 999,
        });
        dup.track("msg1");
        assert!(dup.check("msg1"));

        thread::sleep(Duration::from_millis(60));
        assert!(!dup.check("msg1")); // expired
    }

    #[test]
    fn drop_cleans_expired() {
        let mut dup = Dup::with_config(DupConfig {
            age_ms: 50,
            max: 999,
        });
        dup.track("msg1");
        dup.track("msg2");
        assert_eq!(dup.len(), 2);

        thread::sleep(Duration::from_millis(60));
        dup.drop_expired();
        assert_eq!(dup.len(), 0);
    }

    #[test]
    fn max_triggers_cleanup() {
        let mut dup = Dup::with_config(DupConfig {
            age_ms: 50,
            max: 3,
        });
        dup.track("a");
        dup.track("b");
        dup.track("c");
        thread::sleep(Duration::from_millis(60));
        // Adding one more triggers cleanup since we're over max
        dup.track("d");
        // Old expired entries should be gone, only "d" remains
        assert_eq!(dup.len(), 1);
        assert!(dup.check("d"));
        assert!(!dup.check("a"));
    }

    #[test]
    fn via_records_origin_peer() {
        let mut dup = Dup::new();
        dup.track_from("msg1", Some("peerA".into()));
        assert_eq!(dup.via("msg1"), Some("peerA"));
        assert_eq!(dup.via("unknown"), None);
    }

    #[test]
    fn via_first_sender_wins() {
        let mut dup = Dup::new();
        dup.track_from("msg1", Some("peerA".into()));
        dup.track_from("msg1", Some("peerB".into()));
        assert_eq!(dup.via("msg1"), Some("peerA"), "original sender kept");
    }

    #[test]
    fn via_backfilled_when_initially_unknown() {
        let mut dup = Dup::new();
        dup.track("msg1");
        dup.track_from("msg1", Some("peerA".into()));
        assert_eq!(dup.via("msg1"), Some("peerA"));
    }

    #[test]
    fn eviction_removes_oldest_first() {
        let mut dup = Dup::with_config(DupConfig {
            age_ms: 60_000, // nothing expires during the test
            max: 2,
        });
        dup.track("first");
        thread::sleep(Duration::from_millis(2));
        dup.track("second");
        thread::sleep(Duration::from_millis(2));
        dup.track("third"); // over max → oldest ("first") evicted

        assert_eq!(dup.len(), 2);
        assert!(!dup.check("first"));
        assert!(dup.check("second"));
        assert!(dup.check("third"));
    }
}
