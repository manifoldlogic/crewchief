//! IndexedDbRadStore — RAD chunk storage over IndexedDB (wasm32 only).
//!
//! Bridges the async [`crate::storage::indexeddb::IndexedDbStorage`] to the
//! synchronous [`RadStore`] contract with a write-through in-memory mirror:
//!
//! - [`IndexedDbRadStore::open`] hydrates the mirror from IndexedDB once
//!   (all chunk records live under the `rad/` key prefix).
//! - `get`/`list` are served synchronously from the mirror.
//! - `put` updates the mirror synchronously and persists to IndexedDB in a
//!   background task (`spawn_local`). Persistence errors are surfaced on
//!   the *next* store operation.
//!
//! Chunk data (a JSON string) is stored as a `StoredValue` with
//! `GunValue::Text(data)` under key `rad/<encoded-file-name>`, so RAD
//! chunks coexist with regular graph records in the same object store.
//!
//! This mirrors GUN's `rindexed.js` behavior in spirit: radisk already
//! batches and buffers, and the browser store is inherently async, so
//! durability is eventual (flush ➜ microtask ➜ IDB transaction).

#![cfg(target_arch = "wasm32")]

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::concurrency::{SharedMut, lock_mut, new_shared_mut};
use crate::storage::StoredValue;
use crate::storage::indexeddb::{IndexedDbConfig, IndexedDbStorage};
use crate::types::GunValue;

use super::store::RadStore;

/// Key namespace for RAD chunks inside the IndexedDB object store.
const RAD_PREFIX: &str = "rad/";

/// IndexedDB-backed [`RadStore`] with a synchronous write-through mirror.
pub struct IndexedDbRadStore {
    db: Rc<IndexedDbStorage>,
    mirror: SharedMut<BTreeMap<String, String>>,
    last_err: SharedMut<Option<String>>,
}

impl IndexedDbRadStore {
    /// Open the database and hydrate the mirror with all existing chunks.
    pub async fn open(config: IndexedDbConfig) -> Result<Self, String> {
        let mut storage = IndexedDbStorage::new(config);
        storage.open().await?;
        let db = Rc::new(storage);

        let mut mirror = BTreeMap::new();
        for (key, stored) in db.get_all_with_prefix(RAD_PREFIX).await? {
            if let (Some(file), GunValue::Text(data)) =
                (key.strip_prefix(RAD_PREFIX), &stored.value)
            {
                mirror.insert(file.to_string(), data.clone());
            }
        }

        Ok(Self {
            db,
            mirror: new_shared_mut(mirror),
            last_err: new_shared_mut(None),
        })
    }

    /// Return (and clear) any error from a previous background write.
    fn take_err(&self) -> Result<(), String> {
        match lock_mut(&self.last_err).take() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

impl RadStore for IndexedDbRadStore {
    fn put(&self, file: &str, data: &str) -> Result<(), String> {
        self.take_err()?;
        lock_mut(&self.mirror).insert(file.to_string(), data.to_string());

        let db = self.db.clone();
        let last_err = self.last_err.clone();
        let key = format!("{}{}", RAD_PREFIX, file);
        let stored = StoredValue {
            value: GunValue::Text(data.to_string()),
            state: 0.0,
        };
        crate::runtime::spawn_async(async move {
            if let Err(e) = db.put(&key, &stored).await {
                *lock_mut(&last_err) = Some(format!("rindexed put: {}", e));
            }
        });
        Ok(())
    }

    fn get(&self, file: &str) -> Result<Option<String>, String> {
        self.take_err()?;
        Ok(lock_mut(&self.mirror).get(file).cloned())
    }

    fn list(&self) -> Result<Vec<String>, String> {
        self.take_err()?;
        Ok(lock_mut(&self.mirror).keys().cloned().collect())
    }
}
