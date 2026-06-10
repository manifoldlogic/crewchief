//! Storage — persistence adapters for the GUN graph.
//!
//! GUN's storage model (from `lib/store.js`):
//! - Keys are `soul + ESC + key` where ESC is `\x1B` (ASCII 27)
//! - Values are `{ ":": gun_value, ">": state_timestamp }`
//! - On every "put" event, the key-value is persisted
//! - On "get", data is loaded and fed back into the graph
//!
//! This module provides:
//! - `StorageAdapter` — sync trait for pluggable backends
//! - `AsyncStorageAdapter` — async trait for I/O-bound backends (IndexedDB, etc.)
//! - `MemoryStorage` — in-memory (for testing and ephemeral use)
//! - `StorageEngine` — sync integration with `Gun`
//! - `BatchWriter` — buffered async write engine

pub mod engine;
pub mod indexeddb;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::events::ListenerId;
use crate::instance::Gun;
use crate::types::{GunValue, Node};
use crate::wire;

/// The separator between soul and key in storage keys.
/// Matches GUN's `String.fromCharCode(27)`.
pub const ESC: char = '\x1B';

/// A stored value: the GunValue plus its state timestamp.
#[derive(Debug, Clone)]
pub struct StoredValue {
    pub value: GunValue,
    pub state: f64,
}

/// Trait for pluggable storage backends.
///
/// Implementations could target:
/// - In-memory (testing)
/// - IndexedDB (WASM/browser)
/// - Filesystem (native/Node.js)
/// - SQLite, S3, etc.
pub trait StorageAdapter {
    /// Store a single key-value pair.
    ///
    /// The key is in `soul\x1Bproperty` format.
    fn put(&mut self, key: &str, value: &StoredValue) -> Result<(), String>;

    /// Retrieve a single key-value pair.
    fn get(&self, key: &str) -> Result<Option<StoredValue>, String>;

    /// Retrieve all keys matching a prefix.
    ///
    /// Used for loading entire nodes (`soul\x1B` prefix) or
    /// range queries.
    fn scan(&self, prefix: &str) -> Result<Vec<(String, StoredValue)>, String>;

    /// Delete a key.
    fn delete(&mut self, key: &str) -> Result<(), String>;
}

/// Async trait for I/O-bound storage backends.
///
/// Used for backends where operations are inherently async:
/// - IndexedDB (browser, all operations return Promises)
/// - Network-backed storage
/// - Any backend requiring non-blocking I/O
///
/// The sync `StorageAdapter` stays for simple backends like `MemoryStorage`.
pub trait AsyncStorageAdapter: Send + Sync {
    /// Store a single key-value pair asynchronously.
    fn put(
        &self,
        key: String,
        value: StoredValue,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + '_>>;

    /// Retrieve a single key-value pair asynchronously.
    fn get(
        &self,
        key: String,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<StoredValue>, String>> + '_>,
    >;

    /// Retrieve all keys matching a prefix asynchronously.
    fn scan(
        &self,
        prefix: String,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<(String, StoredValue)>, String>> + '_>,
    >;

    /// Delete a key asynchronously.
    fn delete(
        &self,
        key: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + '_>>;
}

/// Async in-memory storage for testing async code paths.
///
/// Wraps `MemoryStorage` behind a mutex and returns ready futures.
pub struct AsyncMemoryStorage {
    inner: std::sync::Mutex<MemoryStorage>,
}

impl AsyncMemoryStorage {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(MemoryStorage::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for AsyncMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncStorageAdapter for AsyncMemoryStorage {
    fn put(
        &self,
        key: String,
        value: StoredValue,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + '_>> {
        Box::pin(async move {
            let mut store = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            store.put(&key, &value)
        })
    }

    fn get(
        &self,
        key: String,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<StoredValue>, String>> + '_>,
    > {
        Box::pin(async move {
            let store = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            store.get(&key)
        })
    }

    fn scan(
        &self,
        prefix: String,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<(String, StoredValue)>, String>> + '_>,
    > {
        Box::pin(async move {
            let store = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            store.scan(&prefix)
        })
    }

    fn delete(
        &self,
        key: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + '_>> {
        Box::pin(async move {
            let mut store = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            store.delete(&key)
        })
    }
}

/// Build a storage key from soul and property name.
///
/// Returns None if soul or key contain the ESC separator, which would
/// cause key injection (M6 fix).
pub fn storage_key(soul: &str, key: &str) -> Option<String> {
    if soul.contains(ESC) || key.contains(ESC) {
        return None; // reject to prevent storage key injection
    }
    Some(format!("{}{}{}", soul, ESC, key))
}

/// Parse a storage key into (soul, property).
pub fn parse_storage_key(key: &str) -> Option<(&str, &str)> {
    key.split_once(ESC)
}

// ── In-memory storage ───────────────────────────────────────────────────

/// In-memory storage backend using a BTreeMap.
///
/// Data is sorted lexicographically, which makes prefix scans efficient.
/// All data is lost when the process exits.
#[derive(Debug, Default)]
pub struct MemoryStorage {
    data: BTreeMap<String, StoredValue>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    /// Number of stored entries.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl StorageAdapter for MemoryStorage {
    fn put(&mut self, key: &str, value: &StoredValue) -> Result<(), String> {
        self.data.insert(key.to_string(), value.clone());
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<StoredValue>, String> {
        Ok(self.data.get(key).cloned())
    }

    fn scan(&self, prefix: &str) -> Result<Vec<(String, StoredValue)>, String> {
        let results: Vec<_> = self
            .data
            .range(prefix.to_string()..)
            .take_while(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Ok(results)
    }

    fn delete(&mut self, key: &str) -> Result<(), String> {
        self.data.remove(key);
        Ok(())
    }
}

// ── Storage engine (integrates with Gun) ────────────────────────────────

/// Bridges a `StorageAdapter` to a `Gun` instance.
///
/// - Listens for "put" events and persists data to the adapter
/// - Provides `load()` to hydrate the graph from storage
pub struct StorageEngine {
    gun: Gun,
    adapter: Arc<Mutex<Box<dyn StorageAdapter>>>,
    _listener_id: Option<ListenerId>,
}

impl StorageEngine {
    /// Create a storage engine and start auto-persisting writes.
    pub fn new(gun: Gun, adapter: impl StorageAdapter + 'static) -> Self {
        let adapter: Arc<Mutex<Box<dyn StorageAdapter>>> =
            Arc::new(Mutex::new(Box::new(adapter)));

        // Listen for "put" events and persist to storage
        let adapter_clone = adapter.clone();
        let listener_id = gun.on_event("put", move |event| {
            if let (Some(value), Some(key)) = (&event.value, &event.key) {
                if let Some(storage_k) = storage_key(&event.soul, key) {
                    let stored = StoredValue {
                        value: value.clone(),
                        state: event.state,
                    };
                    let mut adapter = adapter_clone.lock().unwrap_or_else(|e| e.into_inner());
                    let _ = adapter.put(&storage_k, &stored);
                }
            }
        });

        Self {
            gun,
            adapter,
            _listener_id: Some(listener_id),
        }
    }

    /// Load a node from storage into the graph.
    ///
    /// Reads all keys for the given soul from the storage adapter
    /// and merges them into the Gun graph via `receive()`.
    pub fn load(&self, soul: &str) {
        let prefix = format!("{}{}", soul, ESC);
        // Read from storage first, then release the lock before calling receive()
        // to avoid deadlock (receive triggers "put" listener which locks adapter).
        let entries = {
            let adapter = self.adapter.lock().unwrap_or_else(|e| e.into_inner());
            adapter.scan(&prefix).unwrap_or_default()
        };

        if entries.is_empty() {
            return;
        }

        let mut node = Node::new(soul);
        for (storage_key, stored) in entries {
            if let Some((_, key)) = parse_storage_key(&storage_key) {
                node.put(key, stored.value, stored.state);
            }
        }

        let msg = wire::put_message("storage_load", &[&node]);
        self.gun.receive(&msg);
    }

    /// Load all data from storage into the graph.
    ///
    /// Scans the entire storage and rebuilds the graph.
    pub fn load_all(&self) {
        // Read everything from storage, then release the lock.
        let entries = {
            let adapter = self.adapter.lock().unwrap_or_else(|e| e.into_inner());
            adapter.scan("").unwrap_or_default()
        };

        // Group entries by soul
        let mut nodes: BTreeMap<String, Node> = BTreeMap::new();
        for (storage_key, stored) in entries {
            if let Some((soul, key)) = parse_storage_key(&storage_key) {
                let node = nodes
                    .entry(soul.to_string())
                    .or_insert_with(|| Node::new(soul));
                node.put(key, stored.value, stored.state);
            }
        }

        // Merge all nodes into the graph (adapter lock is NOT held here)
        for (i, (_, node)) in nodes.iter().enumerate() {
            let msg_id = format!("storage_load_{}", i);
            let msg = wire::put_message(&msg_id, &[node]);
            self.gun.receive(&msg);
        }
    }

    /// Get a reference to the Gun instance.
    pub fn gun(&self) -> &Gun {
        &self.gun
    }

    /// Access the storage adapter directly.
    pub fn adapter(&self) -> Arc<Mutex<Box<dyn StorageAdapter>>> {
        self.adapter.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::GunOptions;

    fn new_gun() -> Gun {
        Gun::new(GunOptions::default())
    }

    // ── Storage key tests ───────────────────────────────────────────

    #[test]
    fn storage_key_format() {
        let key = storage_key("mark", "name").unwrap();
        assert_eq!(key, "mark\x1Bname");
    }

    #[test]
    fn parse_storage_key_roundtrip() {
        let key = storage_key("mark", "name").unwrap();
        let (soul, prop) = parse_storage_key(&key).unwrap();
        assert_eq!(soul, "mark");
        assert_eq!(prop, "name");
    }

    #[test]
    fn parse_storage_key_with_esc_in_soul() {
        let result = parse_storage_key("no-esc-here");
        assert!(result.is_none());
    }

    #[test]
    fn storage_key_rejects_esc_in_soul() {
        // M6: souls containing ESC should be rejected
        assert!(storage_key("evil\x1Binjection", "val").is_none());
        assert!(storage_key("normal", "evil\x1Bkey").is_none());
    }

    // ── MemoryStorage tests ─────────────────────────────────────────

    #[test]
    fn memory_put_get() {
        let mut store = MemoryStorage::new();
        let val = StoredValue {
            value: GunValue::Text("hello".into()),
            state: 100.0,
        };
        store.put("mark\x1Bname", &val).unwrap();
        let got = store.get("mark\x1Bname").unwrap().unwrap();
        assert_eq!(got.value, GunValue::Text("hello".into()));
        assert_eq!(got.state, 100.0);
    }

    #[test]
    fn memory_get_missing() {
        let store = MemoryStorage::new();
        assert!(store.get("nope").unwrap().is_none());
    }

    #[test]
    fn memory_scan_prefix() {
        let mut store = MemoryStorage::new();
        store
            .put(
                "mark\x1Bname",
                &StoredValue {
                    value: GunValue::Text("Mark".into()),
                    state: 100.0,
                },
            )
            .unwrap();
        store
            .put(
                "mark\x1Bage",
                &StoredValue {
                    value: GunValue::Number(30.0),
                    state: 100.0,
                },
            )
            .unwrap();
        store
            .put(
                "alice\x1Bname",
                &StoredValue {
                    value: GunValue::Text("Alice".into()),
                    state: 100.0,
                },
            )
            .unwrap();

        let results = store.scan("mark\x1B").unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|(k, _)| k.starts_with("mark\x1B")));
    }

    #[test]
    fn memory_delete() {
        let mut store = MemoryStorage::new();
        store
            .put(
                "x\x1By",
                &StoredValue {
                    value: GunValue::Null,
                    state: 1.0,
                },
            )
            .unwrap();
        assert!(store.get("x\x1By").unwrap().is_some());
        store.delete("x\x1By").unwrap();
        assert!(store.get("x\x1By").unwrap().is_none());
    }

    // ── StorageEngine integration tests ─────────────────────────────

    #[test]
    fn auto_persist_on_write() {
        let gun = new_gun();
        let engine = StorageEngine::new(gun.clone(), MemoryStorage::new());

        gun.get("mark").put_kv("name", GunValue::Text("Mark".into()));

        // Check that storage has the data
        let adapter = engine.adapter();
        let store = adapter.lock().unwrap();
        let stored = store.get("mark\x1Bname").unwrap().unwrap();
        assert_eq!(stored.value, GunValue::Text("Mark".into()));
    }

    #[test]
    fn persist_multiple_keys() {
        let gun = new_gun();
        let engine = StorageEngine::new(gun.clone(), MemoryStorage::new());

        gun.get("alice").put(vec![
            ("name".into(), GunValue::Text("Alice".into())),
            ("age".into(), GunValue::Number(25.0)),
        ]);

        let adapter = engine.adapter();
        let store = adapter.lock().unwrap();
        let results = store.scan("alice\x1B").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn load_from_storage() {
        let gun = new_gun();
        let mut store = MemoryStorage::new();

        // Pre-populate storage
        store
            .put(
                "bob\x1Bname",
                &StoredValue {
                    value: GunValue::Text("Bob".into()),
                    state: 100.0,
                },
            )
            .unwrap();
        store
            .put(
                "bob\x1Bemail",
                &StoredValue {
                    value: GunValue::Text("bob@example.com".into()),
                    state: 100.0,
                },
            )
            .unwrap();

        let engine = StorageEngine::new(gun.clone(), store);
        engine.load("bob");

        assert_eq!(
            gun.get("bob").get("name").val(),
            Some(GunValue::Text("Bob".into()))
        );
        assert_eq!(
            gun.get("bob").get("email").val(),
            Some(GunValue::Text("bob@example.com".into()))
        );
    }

    #[test]
    fn load_all_from_storage() {
        let gun = new_gun();
        let mut store = MemoryStorage::new();

        store
            .put(
                "a\x1Bx",
                &StoredValue {
                    value: GunValue::Number(1.0),
                    state: 100.0,
                },
            )
            .unwrap();
        store
            .put(
                "b\x1By",
                &StoredValue {
                    value: GunValue::Number(2.0),
                    state: 100.0,
                },
            )
            .unwrap();

        let engine = StorageEngine::new(gun.clone(), store);
        engine.load_all();

        assert_eq!(gun.get("a").get("x").val(), Some(GunValue::Number(1.0)));
        assert_eq!(gun.get("b").get("y").val(), Some(GunValue::Number(2.0)));
    }

    #[test]
    fn persist_then_load_roundtrip() {
        // Write to gun1 with storage → data persists
        let gun1 = new_gun();
        let engine1 = StorageEngine::new(gun1.clone(), MemoryStorage::new());

        gun1.get("config").put_kv("theme", GunValue::Text("dark".into()));
        gun1.get("config").put_kv("lang", GunValue::Text("en".into()));

        // Extract the storage
        let adapter = engine1.adapter();

        // Create gun2, load from same storage → data restored
        let gun2 = new_gun();
        let store2 = adapter.lock().unwrap();
        let entries = store2.scan("").unwrap();
        drop(store2);

        // Rebuild a fresh store with the same data
        let mut fresh_store = MemoryStorage::new();
        for (k, v) in entries {
            fresh_store.put(&k, &v).unwrap();
        }

        let engine2 = StorageEngine::new(gun2.clone(), fresh_store);
        engine2.load_all();

        assert_eq!(
            gun2.get("config").get("theme").val(),
            Some(GunValue::Text("dark".into()))
        );
        assert_eq!(
            gun2.get("config").get("lang").val(),
            Some(GunValue::Text("en".into()))
        );
    }

    #[test]
    fn persist_links() {
        let gun = new_gun();
        let engine = StorageEngine::new(gun.clone(), MemoryStorage::new());

        gun.get("mark").put_kv("boss", GunValue::Link("fluffy".into()));

        let adapter = engine.adapter();
        let store = adapter.lock().unwrap();
        let stored = store.get("mark\x1Bboss").unwrap().unwrap();
        assert_eq!(stored.value, GunValue::Link("fluffy".into()));
    }

    #[test]
    fn persist_null_tombstone() {
        let gun = new_gun();
        let engine = StorageEngine::new(gun.clone(), MemoryStorage::new());

        gun.get("item").put_kv("deleted", GunValue::Null);

        let adapter = engine.adapter();
        let store = adapter.lock().unwrap();
        let stored = store.get("item\x1Bdeleted").unwrap().unwrap();
        assert_eq!(stored.value, GunValue::Null);
    }

    // ── AsyncMemoryStorage tests ────────────────────────────────────

    /// Minimal single-threaded executor for async tests.
    fn block_on<F: std::future::Future<Output = T>, T>(f: F) -> T {
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

        fn dummy_raw_waker() -> RawWaker {
            fn no_op(_: *const ()) {}
            fn clone(p: *const ()) -> RawWaker {
                RawWaker::new(p, &VTABLE)
            }
            const VTABLE: RawWakerVTable =
                RawWakerVTable::new(clone, no_op, no_op, no_op);
            RawWaker::new(std::ptr::null(), &VTABLE)
        }

        let waker = unsafe { Waker::from_raw(dummy_raw_waker()) };
        let mut cx = Context::from_waker(&waker);
        let mut f = std::pin::pin!(f);

        loop {
            match f.as_mut().poll(&mut cx) {
                Poll::Ready(val) => return val,
                Poll::Pending => panic!("unexpected Pending in test executor"),
            }
        }
    }

    #[test]
    fn async_put_and_get() {
        let store = AsyncMemoryStorage::new();
        block_on(store.put(
            "mark\x1Bname".to_string(),
            StoredValue {
                value: GunValue::Text("Mark".into()),
                state: 100.0,
            },
        ))
        .unwrap();

        let got = block_on(store.get("mark\x1Bname".to_string()))
            .unwrap()
            .unwrap();
        assert_eq!(got.value, GunValue::Text("Mark".into()));
        assert_eq!(got.state, 100.0);
    }

    #[test]
    fn async_get_missing() {
        let store = AsyncMemoryStorage::new();
        let result = block_on(store.get("nope".to_string())).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn async_scan() {
        let store = AsyncMemoryStorage::new();
        block_on(store.put(
            "mark\x1Bname".to_string(),
            StoredValue {
                value: GunValue::Text("Mark".into()),
                state: 100.0,
            },
        ))
        .unwrap();
        block_on(store.put(
            "mark\x1Bage".to_string(),
            StoredValue {
                value: GunValue::Number(30.0),
                state: 100.0,
            },
        ))
        .unwrap();
        block_on(store.put(
            "alice\x1Bname".to_string(),
            StoredValue {
                value: GunValue::Text("Alice".into()),
                state: 100.0,
            },
        ))
        .unwrap();

        let results = block_on(store.scan("mark\x1B".to_string())).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn async_delete() {
        let store = AsyncMemoryStorage::new();
        block_on(store.put(
            "x\x1By".to_string(),
            StoredValue {
                value: GunValue::Null,
                state: 1.0,
            },
        ))
        .unwrap();
        assert!(block_on(store.get("x\x1By".to_string())).unwrap().is_some());

        block_on(store.delete("x\x1By".to_string())).unwrap();
        assert!(block_on(store.get("x\x1By".to_string())).unwrap().is_none());
    }

    #[test]
    fn async_memory_storage_len() {
        let store = AsyncMemoryStorage::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);

        block_on(store.put(
            "a\x1Bb".to_string(),
            StoredValue {
                value: GunValue::Null,
                state: 1.0,
            },
        ))
        .unwrap();
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);
    }
}
