//! RAD — GUN's radix storage engine (`lib/radix.js` + `lib/radisk.js` +
//! `lib/rfs.js`/`lib/rindexed.js`), ported per `sources/docs/rad.md`.
//!
//! ```text
//!   Gun graph (put/get events)
//!        │  soul + \x1B + key  →  {":": value, ">": state}
//!        ▼
//!   RadStorageAdapter        — implements the crate's sync StorageAdapter
//!        ▼
//!   Radisk (radisk.rs)       — batching, chunk split, directory, healing
//!        ▼
//!   RadStore (store.rs)      — put/get/list over URL-encoded file names
//!        ▼
//!   FsStore (native) │ IndexedDbRadStore (wasm) │ MemoryRadStore (any)
//! ```
//!
//! | Submodule | GUN source | Purpose |
//! |-----------|-----------|---------|
//! | [`radix`] | `lib/radix.js` | In-memory radix tree, `Radix.map` iteration |
//! | [`radisk`] | `lib/radisk.js` | Batching, chunking, directory, self-healing |
//! | [`store`] | adapter contract | `RadStore` trait + `MemoryRadStore` |
//! | [`fs_store`] | `lib/rfs.js` | Atomic filesystem store (native) |
//! | [`idb_store`] | `lib/rindexed.js` | IndexedDB store (wasm) |
//!
//! Graph values are stored under `soul + \x1B + key` wrapped in a
//! `{":": value, ">": state}` envelope, identical to GUN's `lib/store.js`,
//! so a gunmetal RAD data directory is readable by GUN.js and vice versa
//! (JSON mode only; the legacy `\x1F` binary format is not read).

pub mod fs_store;
pub mod idb_store;
pub mod radisk;
pub mod radix;
pub mod store;

use serde_json::Value;

use crate::storage::{StorageAdapter, StoredValue};
use crate::types::GunValue;
use crate::wire;

pub use radisk::{DIR_FILE, FROM, RadAck, Radisk, RadiskOptions, ReadOpt, dename, ename};
pub use radix::{MapOpt, Radix, RadixGet};
pub use store::{MemoryRadStore, RadStore};

#[cfg(not(target_arch = "wasm32"))]
pub use fs_store::FsStore;

// ── Envelope: {":": value, ">": state} ──────────────────────────────

/// Wrap a `GunValue` + state in GUN's storage envelope.
pub fn envelope(value: &GunValue, state: f64) -> Value {
    let mut m = serde_json::Map::new();
    m.insert(":".to_string(), wire::value_to_json(value));
    m.insert(
        ">".to_string(),
        serde_json::Number::from_f64(state)
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );
    Value::Object(m)
}

/// Unwrap a storage envelope back into a `StoredValue`.
/// Returns `None` for tombstones / non-envelope leaves.
pub fn parse_envelope(v: &Value) -> Option<StoredValue> {
    let obj = v.as_object()?;
    let value = wire::json_to_value(obj.get(":")?)?;
    let state = obj.get(">").and_then(|s| s.as_f64()).unwrap_or(0.0);
    Some(StoredValue { value, state })
}

// ── StorageAdapter bridge ───────────────────────────────────────────

/// RAD-backed implementation of the crate's sync [`StorageAdapter`], so a
/// relay server (or `StorageEngine`) can persist the graph through radisk.
///
/// Keys are the usual `soul\x1Bkey` storage keys; values are stored as
/// `{":": value, ">": state}` envelopes. Writes are batched by radisk —
/// call [`RadStorageAdapter::flush`] (or rely on the 250 ms timer / drop)
/// for durability. Reads see buffered writes immediately.
pub struct RadStorageAdapter {
    radisk: Radisk,
}

impl RadStorageAdapter {
    /// Wrap an existing radisk.
    pub fn new(radisk: Radisk) -> Self {
        Self { radisk }
    }

    /// Build over any store backend with default options.
    pub fn with_store(store: Box<dyn RadStore>) -> Self {
        Self::new(Radisk::with_store(store))
    }

    /// Access the underlying radisk (e.g. for range queries).
    pub fn radisk(&self) -> &Radisk {
        &self.radisk
    }

    /// Flush pending writes to the backend now.
    pub fn flush(&self) -> Result<(), String> {
        self.radisk.flush()
    }
}

impl StorageAdapter for RadStorageAdapter {
    fn put(&mut self, key: &str, value: &StoredValue) -> Result<(), String> {
        self.radisk
            .put(key, envelope(&value.value, value.state), None)
    }

    fn get(&self, key: &str) -> Result<Option<StoredValue>, String> {
        match self.radisk.get(key)? {
            Some(leaf) => Ok(parse_envelope(&leaf)),
            None => Ok(None),
        }
    }

    fn scan(&self, prefix: &str) -> Result<Vec<(String, StoredValue)>, String> {
        let mut out = Vec::new();
        let p = prefix.to_string();
        self.radisk.each::<(), _>(
            &ReadOpt {
                start: Some(p.clone()),
                end: None,
                reverse: false,
            },
            &mut |v, k| {
                if !k.starts_with(&p) {
                    return Some(()); // ordered iteration → past the prefix, stop
                }
                if let Some(stored) = parse_envelope(v) {
                    out.push((k.to_string(), stored));
                }
                None
            },
        )?;
        Ok(out)
    }

    fn delete(&mut self, key: &str) -> Result<(), String> {
        // GUN has no physical delete — write a null tombstone leaf.
        self.radisk.put(key, Value::Null, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{ESC, storage_key};

    fn adapter() -> (RadStorageAdapter, MemoryRadStore) {
        let store = MemoryRadStore::new();
        let radisk = Radisk::new(
            Box::new(store.clone()),
            RadiskOptions {
                until_ms: 10_000,
                ..Default::default()
            },
        );
        (RadStorageAdapter::new(radisk), store)
    }

    fn stored(value: GunValue, state: f64) -> StoredValue {
        StoredValue { value, state }
    }

    // ── Envelope format ─────────────────────────────────────────────

    #[test]
    fn envelope_format_matches_gun() {
        let env = envelope(&GunValue::Text("Mark".into()), 100.0);
        assert_eq!(
            serde_json::to_string(&env).unwrap(),
            r#"{":":"Mark",">":100.0}"#
        );

        let link = envelope(&GunValue::Link("other".into()), 2.0);
        assert_eq!(
            serde_json::to_string(&link).unwrap(),
            r##"{":":{"#":"other"},">":2.0}"##
        );
    }

    #[test]
    fn envelope_roundtrip_all_value_types() {
        for (v, s) in [
            (GunValue::Null, 1.0),
            (GunValue::Text("hi".into()), 2.0),
            (GunValue::Bool(true), 3.0),
            (GunValue::Number(3.25), 4.0),
            (GunValue::Link("soul".into()), 5.0),
        ] {
            let env = envelope(&v, s);
            let back = parse_envelope(&env).unwrap();
            assert_eq!(back.value, v);
            assert_eq!(back.state, s);
        }
    }

    #[test]
    fn parse_envelope_rejects_non_envelopes() {
        assert!(parse_envelope(&serde_json::json!("plain")).is_none());
        assert!(parse_envelope(&serde_json::json!({"x": 1})).is_none());
        assert!(parse_envelope(&Value::Null).is_none());
    }

    // ── StorageAdapter behavior ─────────────────────────────────────

    #[test]
    fn adapter_put_get() {
        let (mut a, _) = adapter();
        let key = storage_key("mark", "name").unwrap();
        a.put(&key, &stored(GunValue::Text("Mark".into()), 100.0))
            .unwrap();
        let got = a.get(&key).unwrap().unwrap();
        assert_eq!(got.value, GunValue::Text("Mark".into()));
        assert_eq!(got.state, 100.0);
    }

    #[test]
    fn adapter_persists_envelopes_on_disk() {
        let (mut a, store) = adapter();
        let key = storage_key("mark", "name").unwrap();
        a.put(&key, &stored(GunValue::Text("Mark".into()), 100.0))
            .unwrap();
        a.flush().unwrap();
        assert_eq!(
            store.raw("!").unwrap(),
            "{\"mark\\u001bname\":{\"\":{\":\":\"Mark\",\">\":100.0}}}"
        );
    }

    #[test]
    fn adapter_scan_prefix() {
        let (mut a, _) = adapter();
        a.put(
            &storage_key("mark", "name").unwrap(),
            &stored(GunValue::Text("Mark".into()), 1.0),
        )
        .unwrap();
        a.put(
            &storage_key("mark", "age").unwrap(),
            &stored(GunValue::Number(30.0), 1.0),
        )
        .unwrap();
        a.put(
            &storage_key("alice", "name").unwrap(),
            &stored(GunValue::Text("Alice".into()), 1.0),
        )
        .unwrap();

        let results = a.scan(&format!("mark{}", ESC)).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|(k, _)| k.starts_with("mark\u{1B}")));
        // Sorted: age before name.
        assert_eq!(results[0].0, "mark\u{1B}age");
        assert_eq!(results[1].0, "mark\u{1B}name");
    }

    #[test]
    fn adapter_delete_tombstones() {
        let (mut a, _) = adapter();
        let key = storage_key("x", "y").unwrap();
        a.put(&key, &stored(GunValue::Number(1.0), 1.0)).unwrap();
        assert!(a.get(&key).unwrap().is_some());
        a.delete(&key).unwrap();
        assert!(a.get(&key).unwrap().is_none());
        assert!(a.scan("").unwrap().is_empty());
    }

    #[test]
    fn adapter_with_storage_engine() {
        use crate::instance::{Gun, GunOptions};
        use crate::storage::StorageEngine;

        let store = MemoryRadStore::new();
        let radisk = Radisk::new(
            Box::new(store.clone()),
            RadiskOptions {
                until_ms: 10_000,
                ..Default::default()
            },
        );
        let rad_handle = radisk.clone();

        let gun = Gun::new(GunOptions::default());
        let _engine = StorageEngine::new(gun.clone(), RadStorageAdapter::new(radisk));

        gun.get("mark")
            .put_kv("name", GunValue::Text("Mark".into()));
        rad_handle.flush().unwrap();

        // Persisted through radisk into the backing store.
        let raw = store.raw("!").unwrap();
        assert!(raw.contains(r#"":":"Mark""#), "raw chunk: {}", raw);

        // A second engine over the same store hydrates a fresh graph.
        let gun2 = Gun::new(GunOptions::default());
        let radisk2 = Radisk::with_store(Box::new(store.clone()));
        let engine2 = StorageEngine::new(gun2.clone(), RadStorageAdapter::new(radisk2));
        engine2.load_all();
        assert_eq!(
            gun2.get("mark").get("name").val(),
            Some(GunValue::Text("Mark".into()))
        );
    }
}
