//! IndexedDB storage backend (wasm32 only).
//!
//! Implements `AsyncStorageAdapter` using the browser's IndexedDB API.
//!
//! ## Schema
//!
//! - Database: `gunmetal`, version 1
//! - Object store: `graph` with key path `k` (the `soul\x1Bproperty` key)
//! - Value: `{ k, v: json_value, s: state_f64, t: last_access_f64 }`
//!
//! ## Implementation Pattern
//!
//! Each IDB operation returns an `IdbRequest`. We wrap onsuccess/onerror
//! in `js_sys::Promise` and `.await` via `wasm_bindgen_futures::JsFuture`.

use super::StoredValue;
use crate::types::GunValue;

#[derive(Debug, Clone)]
pub struct IndexedDbConfig {
    pub db_name: String,
    pub store_name: String,
    pub version: u32,
}

impl Default for IndexedDbConfig {
    fn default() -> Self {
        Self {
            db_name: "gunmetal".to_string(),
            store_name: "graph".to_string(),
            version: 1,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IdbRecord {
    pub k: String,
    pub v: serde_json::Value,
    pub s: f64,
    pub t: f64,
}

impl IdbRecord {
    pub fn from_stored(key: &str, value: &StoredValue) -> Self {
        let v = match &value.value {
            GunValue::Null => serde_json::Value::Null,
            GunValue::Bool(b) => serde_json::Value::Bool(*b),
            GunValue::Number(n) => serde_json::json!(n),
            GunValue::Text(s) => serde_json::Value::String(s.clone()),
            GunValue::Link(soul) => serde_json::json!({"#": soul}),
        };

        Self {
            k: key.to_string(),
            v,
            s: value.state,
            t: crate::state::now_ms(),
        }
    }

    pub fn to_stored(&self) -> StoredValue {
        let value = if self.v.is_null() {
            GunValue::Null
        } else if let Some(b) = self.v.as_bool() {
            GunValue::Bool(b)
        } else if let Some(n) = self.v.as_f64() {
            GunValue::Number(n)
        } else if let Some(s) = self.v.as_str() {
            GunValue::Text(s.to_string())
        } else if let Some(soul) = self.v.get("#").and_then(|v| v.as_str()) {
            GunValue::Link(soul.to_string())
        } else {
            GunValue::Null
        };

        StoredValue {
            value,
            state: self.s,
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod implementation {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{
        IdbDatabase, IdbFactory, IdbObjectStore, IdbOpenDbRequest, IdbRequest,
        IdbTransaction, IdbTransactionMode,
    };

    use super::{IdbRecord, IndexedDbConfig};
    use crate::storage::StoredValue;

    fn idb_factory() -> Result<IdbFactory, String> {
        let global = js_sys::global();
        let idb: IdbFactory = js_sys::Reflect::get(&global, &JsValue::from_str("indexedDB"))
            .map_err(|_| "indexedDB not available".to_string())?
            .unchecked_into();
        Ok(idb)
    }

    fn record_to_js(record: &IdbRecord) -> Result<JsValue, String> {
        let json = serde_json::to_string(record).map_err(|e| e.to_string())?;
        js_sys::JSON::parse(&json).map_err(|e| format!("{:?}", e))
    }

    fn js_to_record(val: JsValue) -> Result<IdbRecord, String> {
        let json_str = js_sys::JSON::stringify(&val)
            .map_err(|e| format!("{:?}", e))?;
        let s: String = json_str.into();
        serde_json::from_str(&s).map_err(|e| e.to_string())
    }

    fn request_to_future(request: &IdbRequest) -> js_sys::Promise {
        let resolve_cell = std::rc::Rc::new(std::cell::RefCell::new(None::<js_sys::Function>));
        let reject_cell = std::rc::Rc::new(std::cell::RefCell::new(None::<js_sys::Function>));

        let resolve_for_success = resolve_cell.clone();
        let request_for_success = request.clone();
        let on_success = Closure::once(move || {
            if let Some(resolve) = resolve_for_success.borrow().as_ref() {
                let result = request_for_success.result().unwrap_or(JsValue::UNDEFINED);
                let _ = resolve.call1(&JsValue::NULL, &result);
            }
        });

        let reject_for_error = reject_cell.clone();
        let request_for_error = request.clone();
        let on_error = Closure::once(move || {
            if let Some(reject) = reject_for_error.borrow().as_ref() {
                let err = request_for_error
                    .error()
                    .ok()
                    .flatten()
                    .map(|e| JsValue::from_str(&e.message()))
                    .unwrap_or_else(|| JsValue::from_str("IDB request failed"));
                let _ = reject.call1(&JsValue::NULL, &err);
            }
        });

        request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));
        request.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        on_success.forget();
        on_error.forget();

        js_sys::Promise::new(&mut |resolve, reject| {
            *resolve_cell.borrow_mut() = Some(resolve);
            *reject_cell.borrow_mut() = Some(reject);
        })
    }

    pub struct IndexedDbStorage {
        config: IndexedDbConfig,
        db: Option<IdbDatabase>,
    }

    impl IndexedDbStorage {
        pub fn new(config: IndexedDbConfig) -> Self {
            Self { config, db: None }
        }

        pub async fn open(&mut self) -> Result<(), String> {
            let factory = idb_factory()?;
            let request: IdbOpenDbRequest = factory
                .open_with_u32(&self.config.db_name, self.config.version)
                .map_err(|e| format!("{:?}", e))?;

            let store_name = self.config.store_name.clone();
            let on_upgrade = Closure::once(move |e: web_sys::IdbVersionChangeEvent| {
                let db: IdbDatabase = e
                    .target()
                    .unwrap()
                    .unchecked_into::<IdbOpenDbRequest>()
                    .result()
                    .unwrap()
                    .unchecked_into();

                let params = web_sys::IdbObjectStoreParameters::new();
                params.set_key_path(&JsValue::from_str("k"));
                let _ = db.create_object_store_with_optional_parameters(
                    &store_name,
                    &params,
                );
            });

            request.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));
            on_upgrade.forget();

            let promise = request_to_future(request.unchecked_ref());
            let result = wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map_err(|e| format!("{:?}", e))?;

            self.db = Some(result.unchecked_into());
            Ok(())
        }

        fn store(&self, mode: IdbTransactionMode) -> Result<IdbObjectStore, String> {
            let db = self.db.as_ref().ok_or("Database not opened")?;
            let tx: IdbTransaction = db
                .transaction_with_str_and_mode(&self.config.store_name, mode)
                .map_err(|e| format!("{:?}", e))?;
            tx.object_store(&self.config.store_name)
                .map_err(|e| format!("{:?}", e))
        }

        pub async fn put(&self, key: &str, value: &StoredValue) -> Result<(), String> {
            let store = self.store(IdbTransactionMode::Readwrite)?;
            let record = IdbRecord::from_stored(key, value);
            let js_val = record_to_js(&record)?;
            let request = store.put(&js_val).map_err(|e| format!("{:?}", e))?;
            let promise = request_to_future(&request);
            wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map_err(|e| format!("{:?}", e))?;
            Ok(())
        }

        pub async fn get(&self, key: &str) -> Result<Option<StoredValue>, String> {
            let store = self.store(IdbTransactionMode::Readonly)?;
            let request = store
                .get(&JsValue::from_str(key))
                .map_err(|e| format!("{:?}", e))?;
            let promise = request_to_future(&request);
            let result = wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map_err(|e| format!("{:?}", e))?;

            if result.is_undefined() || result.is_null() {
                return Ok(None);
            }

            let record = js_to_record(result)?;
            Ok(Some(record.to_stored()))
        }

        pub async fn get_all_with_prefix(
            &self,
            prefix: &str,
        ) -> Result<Vec<(String, StoredValue)>, String> {
            let store = self.store(IdbTransactionMode::Readonly)?;

            let upper = {
                let mut chars: Vec<char> = prefix.chars().collect();
                if let Some(last) = chars.last_mut() {
                    *last = char::from_u32(*last as u32 + 1).unwrap_or('\u{FFFF}');
                }
                chars.into_iter().collect::<String>()
            };

            let range = web_sys::IdbKeyRange::bound(
                &JsValue::from_str(prefix),
                &JsValue::from_str(&upper),
            )
            .map_err(|e| format!("{:?}", e))?;

            let request = store
                .get_all_with_key(&range)
                .map_err(|e| format!("{:?}", e))?;
            let promise = request_to_future(&request);
            let result = wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map_err(|e| format!("{:?}", e))?;

            let array: js_sys::Array = result.unchecked_into();
            let mut items = Vec::new();
            for i in 0..array.length() {
                let val = array.get(i);
                if let Ok(record) = js_to_record(val) {
                    items.push((record.k.clone(), record.to_stored()));
                }
            }
            Ok(items)
        }

        pub async fn delete(&self, key: &str) -> Result<(), String> {
            let store = self.store(IdbTransactionMode::Readwrite)?;
            let request = store
                .delete(&JsValue::from_str(key))
                .map_err(|e| format!("{:?}", e))?;
            let promise = request_to_future(&request);
            wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map_err(|e| format!("{:?}", e))?;
            Ok(())
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use implementation::IndexedDbStorage;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idb_record_roundtrip_null() {
        let stored = StoredValue {
            value: GunValue::Null,
            state: 100.0,
        };
        let record = IdbRecord::from_stored("test\x1Bkey", &stored);
        let back = record.to_stored();
        assert_eq!(back.value, GunValue::Null);
        assert_eq!(back.state, 100.0);
    }

    #[test]
    fn idb_record_roundtrip_text() {
        let stored = StoredValue {
            value: GunValue::Text("hello".into()),
            state: 200.0,
        };
        let record = IdbRecord::from_stored("soul\x1Bprop", &stored);
        let back = record.to_stored();
        assert_eq!(back.value, GunValue::Text("hello".into()));
    }

    #[test]
    fn idb_record_roundtrip_number() {
        let stored = StoredValue {
            value: GunValue::Number(42.5),
            state: 300.0,
        };
        let record = IdbRecord::from_stored("a\x1Bb", &stored);
        let back = record.to_stored();
        assert_eq!(back.value, GunValue::Number(42.5));
    }

    #[test]
    fn idb_record_roundtrip_bool() {
        let stored = StoredValue {
            value: GunValue::Bool(true),
            state: 400.0,
        };
        let record = IdbRecord::from_stored("a\x1Bb", &stored);
        let back = record.to_stored();
        assert_eq!(back.value, GunValue::Bool(true));
    }

    #[test]
    fn idb_record_roundtrip_link() {
        let stored = StoredValue {
            value: GunValue::Link("target_soul".into()),
            state: 500.0,
        };
        let record = IdbRecord::from_stored("a\x1Bb", &stored);
        let back = record.to_stored();
        assert_eq!(back.value, GunValue::Link("target_soul".into()));
    }

    #[test]
    fn idb_config_defaults() {
        let config = IndexedDbConfig::default();
        assert_eq!(config.db_name, "gunmetal");
        assert_eq!(config.store_name, "graph");
        assert_eq!(config.version, 1);
    }

    #[test]
    fn idb_record_serialization() {
        let record = IdbRecord {
            k: "soul\x1Bprop".into(),
            v: serde_json::json!("hello"),
            s: 123.456,
            t: 789.0,
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: IdbRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.k, record.k);
        assert_eq!(back.s, record.s);
    }
}
