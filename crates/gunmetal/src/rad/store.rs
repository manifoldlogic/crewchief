//! RadStore — the pluggable chunk storage backend for RAD.
//!
//! GUN's radisk only needs a store with `put(key, data, cb)` and
//! `get(key, cb)` (plus an optional `list(cb)` for startup import). We use
//! idiomatic synchronous `Result` returns instead of literal JS callbacks;
//! the batching/callback layer lives in [`crate::rad::radisk`].
//!
//! Contract (mirrors rad.md "Storage Interface"):
//! - `put` persists a serialized chunk string under a URL-encoded file name
//! - `get` returns `Ok(None)` for missing files — *not* an error
//! - `list` enumerates stored (URL-encoded) file names for directory import
//!
//! Backends:
//! - [`MemoryRadStore`] — in-memory, all targets (tests, ephemeral, WASM)
//! - [`crate::rad::fs_store::FsStore`] — filesystem, native only
//! - [`crate::rad::idb_store::IndexedDbRadStore`] — IndexedDB, wasm only

use std::collections::BTreeMap;

use crate::concurrency::{SharedMut, lock_mut, new_shared_mut};

// ── Platform Send marker ────────────────────────────────────────────

/// `Send` on native (stores cross threads via the flush timer), nothing on
/// WASM (single-threaded, `Rc`-based state can't be `Send`).
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send> MaybeSend for T {}

#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}
#[cfg(target_arch = "wasm32")]
impl<T> MaybeSend for T {}

// ── RadStore trait ──────────────────────────────────────────────────

/// Chunk storage backend for [`crate::rad::Radisk`].
///
/// File names passed in are already URL-encoded (see [`crate::rad::ename`]).
pub trait RadStore: MaybeSend {
    /// Persist a serialized chunk under `file`.
    fn put(&self, file: &str, data: &str) -> Result<(), String>;

    /// Retrieve a chunk. Missing files are `Ok(None)`, never `Err`.
    fn get(&self, file: &str) -> Result<Option<String>, String>;

    /// Enumerate all stored (URL-encoded) file names. Used once at startup
    /// to import the directory. Backends without enumeration return `[]`.
    fn list(&self) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
}

// ── In-memory store ─────────────────────────────────────────────────

/// In-memory `RadStore` (BTreeMap of file → data).
///
/// Cloning shares the underlying map, which lets tests hold a handle to
/// inspect what radisk actually wrote.
#[derive(Clone, Default)]
pub struct MemoryRadStore {
    data: SharedMut<BTreeMap<String, String>>,
    puts: SharedMut<usize>,
}

impl MemoryRadStore {
    pub fn new() -> Self {
        Self {
            data: new_shared_mut(BTreeMap::new()),
            puts: new_shared_mut(0),
        }
    }

    /// Number of `put` calls made (for batching tests).
    pub fn put_count(&self) -> usize {
        *lock_mut(&self.puts)
    }

    /// Stored file names, sorted.
    pub fn files(&self) -> Vec<String> {
        lock_mut(&self.data).keys().cloned().collect()
    }

    /// Raw stored data for a file (test inspection).
    pub fn raw(&self, file: &str) -> Option<String> {
        lock_mut(&self.data).get(file).cloned()
    }

    /// Overwrite raw data directly (test corruption injection).
    pub fn inject(&self, file: &str, data: &str) {
        lock_mut(&self.data).insert(file.to_string(), data.to_string());
    }
}

impl RadStore for MemoryRadStore {
    fn put(&self, file: &str, data: &str) -> Result<(), String> {
        *lock_mut(&self.puts) += 1;
        lock_mut(&self.data).insert(file.to_string(), data.to_string());
        Ok(())
    }

    fn get(&self, file: &str) -> Result<Option<String>, String> {
        Ok(lock_mut(&self.data).get(file).cloned())
    }

    fn list(&self) -> Result<Vec<String>, String> {
        Ok(self.files())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_put_get() {
        let s = MemoryRadStore::new();
        s.put("!", r#"{"a":{"":1}}"#).unwrap();
        assert_eq!(s.get("!").unwrap().unwrap(), r#"{"a":{"":1}}"#);
    }

    #[test]
    fn memory_get_missing_is_none_not_err() {
        let s = MemoryRadStore::new();
        assert_eq!(s.get("nope").unwrap(), None);
    }

    #[test]
    fn memory_list() {
        let s = MemoryRadStore::new();
        s.put("!", "{}").unwrap();
        s.put("m", "{}").unwrap();
        assert_eq!(s.list().unwrap(), vec!["!".to_string(), "m".to_string()]);
    }

    #[test]
    fn memory_clone_shares_data() {
        let s1 = MemoryRadStore::new();
        let s2 = s1.clone();
        s1.put("!", "{}").unwrap();
        assert_eq!(s2.get("!").unwrap().unwrap(), "{}");
        assert_eq!(s2.put_count(), 1);
    }
}
