//! WASM bindings for gunmetal via wasm-bindgen.
//!
//! Exposes the complete GUN API to JavaScript: data operations, SEA crypto,
//! and User authentication.
//!
//! # Usage from JavaScript
//!
//! ```js
//! import init, { WasmGun, WasmSEA, WasmUser } from './gunmetal.js';
//!
//! await init();
//! const gun = new WasmGun();
//! const sea = new WasmSEA();
//! const user = new WasmUser(gun);
//!
//! // Create account & login
//! const result = user.create("alice", "password123");
//! console.log("Created:", result);
//!
//! // Write to user namespace
//! user.put("profile", JSON.stringify("Alice"));
//!
//! // SEA crypto
//! const pair = sea.pair();
//! const signed = sea.sign(JSON.stringify("hello"), pair.priv, pair.pub);
//! const verified = sea.verify(signed, pair.pub);
//! ```

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::cert::{CertWhat, CertWho, Certificate};
#[cfg(target_arch = "wasm32")]
use crate::events::ListenerId;
#[cfg(target_arch = "wasm32")]
use crate::instance::{Gun, GunOptions};
#[cfg(target_arch = "wasm32")]
use crate::sea;
#[cfg(target_arch = "wasm32")]
use crate::types::GunValue;
#[cfg(target_arch = "wasm32")]
use crate::user::{AuthResult, CreateResult, User};
#[cfg(target_arch = "wasm32")]
use crate::wire;

// ═══════════════════════════════════════════════════════════════════════
// WasmGun — Core database operations
// ═══════════════════════════════════════════════════════════════════════

/// Live networking state: the DAM mesh plus the browser WebSocket
/// transport feeding it. Created lazily on the first `connect()`.
#[cfg(target_arch = "wasm32")]
struct WasmNet {
    mesh: crate::mesh::Mesh,
    transport: std::rc::Rc<crate::transport::ws_wasm::WsWasmTransport>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmGun {
    inner: Gun,
    net: std::rc::Rc<std::cell::RefCell<Option<WasmNet>>>,
    status: std::rc::Rc<std::cell::RefCell<Option<js_sys::Function>>>,
    /// `(direction, peer, raw)` wire tap — see [`Self::on_wire`].
    wire_tap: std::rc::Rc<std::cell::RefCell<Option<js_sys::Function>>>,
    /// Guard against double persistence registration (listener leak +
    /// duplicate IDB writes).
    persistence_on: std::rc::Rc<std::cell::Cell<bool>>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmGun {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::from_gun(Gun::new(GunOptions::default()))
    }

    fn from_gun(gun: Gun) -> Self {
        Self {
            inner: gun,
            net: std::rc::Rc::new(std::cell::RefCell::new(None)),
            status: std::rc::Rc::new(std::cell::RefCell::new(None)),
            wire_tap: std::rc::Rc::new(std::cell::RefCell::new(None)),
            persistence_on: std::rc::Rc::new(std::cell::Cell::new(false)),
        }
    }

    /// Construct with a JSON options object — the wasm analogue of
    /// `Gun(options)`. Recognized keys (all optional):
    /// `{"peers": [..], "localStorage": bool, "radisk": bool,
    ///   "axe": bool, "pid": "..", "gap": ms, "mob": n}`.
    /// Peers are recorded in the options; call `connect(url)` to dial.
    #[wasm_bindgen(js_name = "withOptions")]
    pub fn with_options(json: &str) -> Result<WasmGun, JsValue> {
        #[derive(serde::Deserialize)]
        struct JsOptions {
            peers: Option<Vec<String>>,
            #[serde(alias = "localStorage")]
            local_storage: Option<bool>,
            radisk: Option<bool>,
            axe: Option<bool>,
            pid: Option<String>,
            /// Batching gap window in milliseconds.
            gap: Option<f64>,
            mob: Option<usize>,
        }
        let parsed: JsOptions = serde_json::from_str(json)
            .map_err(|e| JsValue::from_str(&format!("Invalid options: {}", e)))?;
        let mut options = GunOptions::default();
        if let Some(peers) = parsed.peers {
            options.peers = peers;
        }
        if let Some(v) = parsed.local_storage {
            options.local_storage = v;
        }
        if let Some(v) = parsed.radisk {
            options.radisk = v;
        }
        if let Some(v) = parsed.axe {
            options.axe = v;
        }
        if parsed.pid.is_some() {
            options.pid = parsed.pid;
        }
        if let Some(ms) = parsed.gap {
            options.gap = std::time::Duration::from_millis(ms.max(0.0) as u64);
        }
        if let Some(mob) = parsed.mob {
            options.mob = mob;
        }
        Ok(Self::from_gun(Gun::new(options)))
    }

    // ── Write operations ────────────────────────────────────────────

    #[wasm_bindgen(js_name = "putText")]
    pub fn put_text(&self, soul: &str, key: &str, value: &str) {
        self.inner
            .get(soul)
            .put_kv(key, GunValue::Text(value.to_string()));
    }

    #[wasm_bindgen(js_name = "putNumber")]
    pub fn put_number(&self, soul: &str, key: &str, value: f64) {
        if let Some(v) = GunValue::number(value) {
            self.inner.get(soul).put_kv(key, v);
        }
    }

    #[wasm_bindgen(js_name = "putBool")]
    pub fn put_bool(&self, soul: &str, key: &str, value: bool) {
        self.inner.get(soul).put_kv(key, GunValue::Bool(value));
    }

    #[wasm_bindgen(js_name = "putNull")]
    pub fn put_null(&self, soul: &str, key: &str) {
        self.inner.get(soul).put_kv(key, GunValue::Null);
    }

    #[wasm_bindgen(js_name = "putLink")]
    pub fn put_link(&self, soul: &str, key: &str, target_soul: &str) {
        self.inner
            .get(soul)
            .put_kv(key, GunValue::Link(target_soul.to_string()));
    }

    /// Write a JSON object. Each key becomes a property on the node.
    #[wasm_bindgen(js_name = "putObject")]
    pub fn put_object(&self, soul: &str, json: &str) -> Result<(), JsValue> {
        let parsed: serde_json::Value =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;

        if let Some(obj) = parsed.as_object() {
            let data: Vec<(String, GunValue)> = obj
                .iter()
                .filter_map(|(k, v)| wire::json_to_value(v).map(|gv| (k.clone(), gv)))
                .collect();
            self.inner.get(soul).put(data);
            Ok(())
        } else {
            Err(JsValue::from_str("Expected a JSON object"))
        }
    }

    // ── Collections ─────────────────────────────────────────────────

    /// Add a JSON object to a set: creates an item node under a
    /// time-sortable unique soul, links it into the set node, and
    /// returns the item's soul. Equivalent to `gun.get(set).set(obj)`.
    #[wasm_bindgen(js_name = "setObject")]
    pub fn set_object(&self, set_soul: &str, json: &str) -> Result<String, JsValue> {
        let item_soul = format!("{}/{}", set_soul, crate::uuid::generate_uuid());
        self.put_object(&item_soul, json)?;
        self.inner
            .get(set_soul)
            .put_kv(&item_soul, GunValue::Link(item_soul.clone()));
        Ok(item_soul)
    }

    /// Add a primitive JSON value to a set under a generated
    /// time-sortable UUID key (keys sort in insertion-time order).
    /// Returns the key.
    #[wasm_bindgen(js_name = "setValue")]
    pub fn set_value(&self, set_soul: &str, json_value: &str) -> Result<String, JsValue> {
        let parsed: serde_json::Value = serde_json::from_str(json_value)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let value = wire::json_to_value(&parsed)
            .ok_or_else(|| JsValue::from_str("Unsupported value type"))?;
        Ok(self.inner.get(set_soul).set_value(value))
    }

    // ── Persistence ─────────────────────────────────────────────────

    /// Wire IndexedDB-backed persistence into this instance: existing
    /// rows hydrate through the normal merge path (HAM applies; nothing
    /// re-broadcasts), then every accepted write persists. `db_name`
    /// namespaces the store — callers sharing an origin (e.g. iframes)
    /// MUST use distinct names per logical session. Resolves once
    /// hydration completes.
    #[wasm_bindgen(js_name = "enablePersistence")]
    pub fn enable_persistence(&self, db_name: &str) -> js_sys::Promise {
        use crate::storage::indexeddb::IndexedDbStorage;
        use crate::storage::StoredValue;

        const ROW_PREFIX: &str = "n!";

        // Idempotent: a second call must not register a second put
        // listener (duplicate IDB writes for the instance's lifetime).
        let guard = self.persistence_on.clone();
        if guard.replace(true) {
            return js_sys::Promise::resolve(&JsValue::TRUE);
        }

        let gun = self.inner.clone();
        let config = crate::storage::indexeddb::IndexedDbConfig {
            db_name: db_name.to_string(),
            ..Default::default()
        };

        wasm_bindgen_futures::future_to_promise(async move {
            let mut idb = IndexedDbStorage::new(config);
            if let Err(e) = idb.open().await {
                guard.set(false); // allow a retry after a failed open
                return Err(JsValue::from_str(&e));
            }

            // Hydrate: rebuild nodes with their ORIGINAL states and
            // replay them through receive() — the same code path a
            // network put takes, so HAM and subscriptions behave
            // identically.
            let rows = idb
                .get_all_with_prefix(ROW_PREFIX)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            let mut nodes: std::collections::HashMap<String, crate::types::Node> =
                std::collections::HashMap::new();
            for (row_key, stored) in rows {
                let Some(rest) = row_key.strip_prefix(ROW_PREFIX) else {
                    continue;
                };
                // M6: same validation as the native storage path — a key
                // with more than one ESC is ambiguous (injection); the
                // round-trip through parse must be exact.
                let Some((soul, key)) = crate::storage::parse_storage_key(rest) else {
                    continue;
                };
                if crate::storage::storage_key(soul, key).as_deref() != Some(rest) {
                    continue;
                }
                nodes
                    .entry(soul.to_string())
                    .or_insert_with(|| crate::types::Node::new(soul))
                    .put(key, stored.value, stored.state);
            }
            if !nodes.is_empty() {
                let refs: Vec<&crate::types::Node> = nodes.values().collect();
                let msg = wire::put_message(&crate::uuid::random_message_id(9), &refs);
                let _ = gun.receive(&msg);
            }

            // Persist every accepted write from here on.
            let idb = std::rc::Rc::new(idb);
            crate::concurrency::lock_mut(&gun.events).on("put", move |event| {
                let (Some(value), Some(key)) = (&event.value, &event.key) else {
                    return;
                };
                // M6: souls/keys are attacker-controlled wire strings — an
                // embedded ESC would make the row ambiguous and let a
                // remote peer spoof writes onto a different soul after the
                // next hydration. storage_key() rejects those.
                let Some(suffix) = crate::storage::storage_key(&event.soul, key) else {
                    return;
                };
                let row_key = format!("{}{}", ROW_PREFIX, suffix);
                let stored = StoredValue {
                    value: value.clone(),
                    state: event.state,
                };
                let idb = idb.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = idb.put(&row_key, &stored).await;
                });
            });

            Ok(JsValue::TRUE)
        })
    }

    // ── Read operations ─────────────────────────────────────────────

    /// Read a value. Returns JSON string or null.
    #[wasm_bindgen(js_name = "get")]
    pub fn get(&self, soul: &str, key: &str) -> JsValue {
        match self.inner.get(soul).get(key).val() {
            Some(val) => {
                let json = wire::value_to_json(&val);
                JsValue::from_str(&json.to_string())
            }
            None => JsValue::NULL,
        }
    }

    /// Read a full node as JSON.
    #[wasm_bindgen(js_name = "getNode")]
    pub fn get_node(&self, soul: &str) -> JsValue {
        match self.inner.get(soul).node_data() {
            Some(data) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in data {
                    obj.insert(k, wire::value_to_json(&v));
                }
                JsValue::from_str(&serde_json::Value::Object(obj).to_string())
            }
            None => JsValue::NULL,
        }
    }

    // ── Subscriptions ───────────────────────────────────────────────

    /// Subscribe to a key. Callback: `(jsonValue: string, key: string)`.
    #[wasm_bindgen(js_name = "on")]
    pub fn on(&self, soul: &str, key: &str, callback: js_sys::Function) -> u32 {
        let id = self.inner.get(soul).get(key).on(move |val, k| {
            let json_val = wire::value_to_json(&val).to_string();
            let _ = callback.call2(
                &JsValue::NULL,
                &JsValue::from_str(&json_val),
                &JsValue::from_str(&k),
            );
        });
        id.0 as u32
    }

    /// Subscribe to all changes on a node.
    #[wasm_bindgen(js_name = "onNode")]
    pub fn on_node(&self, soul: &str, callback: js_sys::Function) -> u32 {
        let id = self.inner.get(soul).on(move |val, k| {
            let json_val = wire::value_to_json(&val).to_string();
            let _ = callback.call2(
                &JsValue::NULL,
                &JsValue::from_str(&json_val),
                &JsValue::from_str(&k),
            );
        });
        id.0 as u32
    }

    #[wasm_bindgen(js_name = "off")]
    pub fn off(&self, soul: &str, key: &str, listener_id: u32) {
        self.inner
            .get(soul)
            .get(key)
            .off(ListenerId(listener_id as u64));
    }

    // ── Sync / Wire ─────────────────────────────────────────────────

    #[wasm_bindgen(js_name = "state")]
    pub fn state(&self) -> f64 {
        self.inner.state()
    }

    /// Process incoming wire message from a peer.
    #[wasm_bindgen(js_name = "receive")]
    pub fn receive(&self, json: &str) -> Result<(), JsValue> {
        let msg =
            wire::parse_message(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.inner.receive(&msg);
        Ok(())
    }

    /// Serialize a node for sending to peers.
    #[wasm_bindgen(js_name = "outgoing")]
    pub fn outgoing(&self, soul: &str) -> JsValue {
        self.inner.graph(|graph| match graph.get_node(soul) {
            Some(node) => {
                let msg_id = format!("gm{}", self.inner.state() as u64);
                let msg = wire::put_message(&msg_id, &[node]);
                match wire::serialize_message(&msg) {
                    Ok(json) => JsValue::from_str(&json),
                    Err(_) => JsValue::NULL,
                }
            }
            None => JsValue::NULL,
        })
    }
}


#[cfg(target_arch = "wasm32")]
impl Default for WasmGun {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WasmGun networking — DAM mesh over browser WebSockets
// ═══════════════════════════════════════════════════════════════════════

// NOTE: both helpers CLONE the Function out of the RefCell before
// invoking — a callback that re-registers itself (onWire/onStatus from
// inside the callback) would otherwise hit an outstanding borrow and
// abort the wasm instance with a BorrowMutError.

#[cfg(target_arch = "wasm32")]
fn fire_wire_tap(
    slot: &std::rc::Rc<std::cell::RefCell<Option<js_sys::Function>>>,
    direction: &str,
    peer: &str,
    raw: &str,
) {
    let cb = slot.borrow().clone();
    if let Some(cb) = cb {
        let _ = cb.call3(
            &JsValue::NULL,
            &JsValue::from_str(direction),
            &JsValue::from_str(peer),
            &JsValue::from_str(raw),
        );
    }
}

#[cfg(target_arch = "wasm32")]
fn fire_status(
    slot: &std::rc::Rc<std::cell::RefCell<Option<js_sys::Function>>>,
    event: &str,
    url: &str,
) {
    let cb = slot.borrow().clone();
    if let Some(cb) = cb {
        let _ = cb.call2(
            &JsValue::NULL,
            &JsValue::from_str(event),
            &JsValue::from_str(url),
        );
    }
}

#[cfg(target_arch = "wasm32")]
impl WasmGun {
    /// Lazily build the mesh + transport pair and wire their callbacks.
    /// The mesh handshake (`hi`) runs on the transport's open callback —
    /// a browser WebSocket throws on send before OPEN. The peer id is
    /// the relay URL.
    fn ensure_net(&self) -> (crate::mesh::Mesh, std::rc::Rc<crate::transport::ws_wasm::WsWasmTransport>) {
        use crate::transport::ws_wasm::WsWasmTransport;
        use std::rc::Rc;

        if self.net.borrow().is_none() {
            let config = crate::mesh::MeshConfig::from_options(self.inner.options());
            let mesh = crate::mesh::Mesh::new(self.inner.clone(), config);
            let transport = Rc::new(WsWasmTransport::new(Default::default()));

            let mesh_for_msg = mesh.clone();
            let tap_in = self.wire_tap.clone();
            transport.set_on_message(move |peer, raw| {
                fire_wire_tap(&tap_in, "in", &peer, &raw);
                mesh_for_msg.hear(&raw, &peer);
            });

            let mesh_for_open = mesh.clone();
            let transport_for_open = transport.clone();
            let status_open = self.status.clone();
            let tap_out = self.wire_tap.clone();
            transport.set_on_open(move |url| {
                let t = transport_for_open.clone();
                let target = url.clone();
                let tap = tap_out.clone();
                let sender: crate::mesh::PeerSender = Rc::new(move |raw: &str| {
                    fire_wire_tap(&tap, "out", &target, raw);
                    let _ = t.send(&target, raw);
                });
                mesh_for_open.hi(&url, Some(url.clone()), Some(sender));
                fire_status(&status_open, "open", &url);
            });

            let mesh_for_close = mesh.clone();
            let status_close = self.status.clone();
            transport.set_on_close(move |url, _code, _reason| {
                mesh_for_close.bye(&url);
                fire_status(&status_close, "close", &url);
            });

            let status_err = self.status.clone();
            transport.set_on_error(move |url, _message| {
                fire_status(&status_err, "error", &url);
            });

            *self.net.borrow_mut() = Some(WasmNet { mesh, transport });
        }

        let net = self.net.borrow();
        let net = net.as_ref().expect("net initialized above");
        (net.mesh.clone(), net.transport.clone())
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmGun {
    /// Dial a relay (`ws://host:8765/gun`). Idempotent per URL. The DAM
    /// handshake runs automatically on open; `peerPid(url)` turning
    /// non-null is the "handshake acked" signal.
    ///
    /// Writes and `registerBye` issued BEFORE the socket opens target
    /// zero peers and are not queued — wait for `onStatus("open")` (or
    /// poll `peerPid`) before relying on them reaching the relay.
    #[wasm_bindgen(js_name = "connect")]
    pub fn connect(&self, url: &str) -> Result<(), JsValue> {
        let (_mesh, transport) = self.ensure_net();
        if transport.is_connected(url) {
            return Ok(());
        }
        transport.connect(url).map_err(|e| JsValue::from_str(&e))
    }

    #[wasm_bindgen(js_name = "disconnect")]
    pub fn disconnect(&self, url: &str) {
        if let Some(net) = self.net.borrow().as_ref() {
            net.transport.close(url);
            net.mesh.bye(url);
        }
    }

    #[wasm_bindgen(js_name = "isConnected")]
    pub fn is_connected(&self, url: &str) -> bool {
        self.net
            .borrow()
            .as_ref()
            .is_some_and(|net| net.transport.is_connected(url))
    }

    /// URLs of peers with an OPEN WebSocket.
    #[wasm_bindgen(js_name = "connectedPeers")]
    pub fn connected_peers(&self) -> js_sys::Array {
        let out = js_sys::Array::new();
        if let Some(net) = self.net.borrow().as_ref() {
            for url in net.transport.connected_urls() {
                out.push(&JsValue::from_str(&url));
            }
        }
        out
    }

    /// The remote process id learned from the DAM `?` handshake, or null
    /// while the handshake hasn't completed.
    #[wasm_bindgen(js_name = "peerPid")]
    pub fn peer_pid(&self, url: &str) -> Option<String> {
        self.net.borrow().as_ref().and_then(|net| net.mesh.peer_pid(url))
    }

    /// Register a `(event, url)` callback for connection lifecycle:
    /// "open" | "close" | "error".
    #[wasm_bindgen(js_name = "onStatus")]
    pub fn on_status(&self, callback: js_sys::Function) {
        *self.status.borrow_mut() = Some(callback);
    }

    /// Wire tap: `(direction, peer, raw)` for every frame this client
    /// sends ("out") or receives ("in") — heartbeats, DAM handshakes,
    /// puts, gets, and acks, exactly as they cross the WebSocket. This
    /// is the wire-inspector's feed; register before `connect()` to see
    /// the handshake itself.
    #[wasm_bindgen(js_name = "onWire")]
    pub fn on_wire(&self, callback: js_sys::Function) {
        *self.wire_tap.borrow_mut() = Some(callback);
    }

    /// Ask the mesh for a soul's current state (an outgoing GET). Pair
    /// with `on()` to receive the answer when it arrives.
    #[wasm_bindgen(js_name = "fetchSoul")]
    pub fn fetch_soul(&self, soul: &str) {
        if let Some(net) = self.net.borrow().as_ref() {
            net.mesh.say(
                crate::wire::get_message(&crate::uuid::random_message_id(9), soul, None),
                None,
            );
        }
    }

    /// Flush any gap-batched outgoing frames immediately.
    #[wasm_bindgen(js_name = "flushMesh")]
    pub fn flush_mesh(&self) {
        if let Some(net) = self.net.borrow().as_ref() {
            net.mesh.flush();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WasmGun — Extended chain API (gun/lib/* equivalents)
// ═══════════════════════════════════════════════════════════════════════

#[cfg(all(target_arch = "wasm32", feature = "extended-api"))]
#[wasm_bindgen]
impl WasmGun {
    /// Promise-based read (`gun/lib/then.js`): resolves with the JSON value
    /// at `soul.key` (or `null` if missing), using `.once()` semantics.
    ///
    /// Exported as `once`, NOT `then`: a `then` method would make every
    /// `WasmGun` a JS thenable, so `await somePromise` resolving to a gun
    /// instance would invoke it with resolver functions and corrupt the
    /// await. (GUN.js carries this foot-gun; we don't.)
    #[wasm_bindgen(js_name = "once")]
    pub fn then_promise(&self, soul: &str, key: &str) -> js_sys::Promise {
        let chain = self.inner.get(soul).get(key);
        wasm_bindgen_futures::future_to_promise(async move {
            Ok(match chain.then().await {
                Some(val) => JsValue::from_str(&wire::value_to_json(&val).to_string()),
                None => JsValue::NULL,
            })
        })
    }

    /// One-shot deep document load (`gun/lib/load.js`): fires `callback`
    /// once with the full document tree as a JSON string.
    #[wasm_bindgen(js_name = "load")]
    pub fn load(&self, soul: &str, callback: js_sys::Function) {
        self.inner
            .get(soul)
            .load(crate::extended::OpenOptions::default(), move |doc| {
                let _ = callback.call1(&JsValue::NULL, &JsValue::from_str(&doc.to_string()));
            });
    }

    /// Dot-notation path read (`gun/lib/path.js`): resolves `soul` then the
    /// dot-delimited `path`, returning the value as JSON or `null`.
    #[wasm_bindgen(js_name = "pathVal")]
    pub fn path_val(&self, soul: &str, path: &str) -> JsValue {
        match self.inner.get(soul).path(path).val() {
            Some(val) => JsValue::from_str(&wire::value_to_json(&val).to_string()),
            None => JsValue::NULL,
        }
    }

    /// Remove a node from a set (`gun/lib/unset.js`): nulls the link to
    /// `item_soul` inside the set at `set_soul`.
    #[wasm_bindgen(js_name = "unset")]
    pub fn unset(&self, set_soul: &str, item_soul: &str) {
        let item = self.inner.get(item_soul);
        self.inner.get(set_soul).unset(&item);
    }

    /// Absence detection (`gun/lib/not.js`): resolves `true` if no data
    /// exists at `soul.key` (or at the soul when `key` is empty) within
    /// `timeout_ms`, `false` once data is present. Cannot GUARANTEE
    /// absence in a distributed system — peers may hold data we haven't
    /// seen (documented caveat).
    #[wasm_bindgen(js_name = "notWithin")]
    pub fn not_within(&self, soul: &str, key: &str, timeout_ms: f64) -> js_sys::Promise {
        let chain = if key.is_empty() {
            self.inner.get(soul)
        } else {
            self.inner.get(soul).get(key)
        };
        let timeout = std::time::Duration::from_millis(timeout_ms.max(0.0) as u64);
        wasm_bindgen_futures::future_to_promise(async move {
            let absent = std::rc::Rc::new(std::cell::Cell::new(false));
            let flag = absent.clone();
            chain.not_within(timeout, move |_| flag.set(true)).await;
            Ok(JsValue::from_bool(absent.get()))
        })
    }

    /// Register a disconnect write (`gun/lib/bye.js`): when this client's
    /// relay connection drops, the relay writes `json_value` to
    /// `soul.key`. Requires an active `connect()`; the registration is
    /// sent to every connected relay.
    #[wasm_bindgen(js_name = "registerBye")]
    pub fn register_bye(&self, soul: &str, key: &str, json_value: &str) -> Result<(), JsValue> {
        let parsed: serde_json::Value = serde_json::from_str(json_value)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let value = wire::json_to_value(&parsed)
            .ok_or_else(|| JsValue::from_str("Unsupported value type"))?;
        let msg = self.inner.get(soul).get(key).bye().put(value);
        let Some(net) = self.net.borrow().as_ref().map(|n| n.mesh.clone()) else {
            return Err(JsValue::from_str("registerBye requires connect() first"));
        };
        net.say(msg, None);
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WasmSEA — Cryptographic operations
// ═══════════════════════════════════════════════════════════════════════

/// SEA cryptographic utilities exposed to JavaScript.
///
/// ```js
/// const sea = new WasmSEA();
/// const pair = sea.pair();
/// console.log(pair); // { pub, priv, epub, epriv }
/// ```
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmSEA;

#[cfg(target_arch = "wasm32")]
impl Default for WasmSEA {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmSEA {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self
    }

    /// Generate a new key pair.
    ///
    /// Returns JSON: `{ pub, priv, epub, epriv }`
    #[wasm_bindgen(js_name = "pair")]
    pub fn pair(&self) -> Result<JsValue, JsValue> {
        let kp = sea::pair().map_err(|e| JsValue::from_str(&e.to_string()))?;
        let json = serde_json::json!({
            "pub": kp.pub_key,
            "priv": kp.priv_key,
            "epub": kp.epub,
            "epriv": kp.epriv
        });
        Ok(JsValue::from_str(&json.to_string()))
    }

    /// Sign data with a private key.
    ///
    /// - `data`: JSON-encoded data to sign
    /// - `priv_key`: private signing key
    /// - `pub_key`: public signing key
    ///
    /// Returns: `SEA{...}` signed message string
    #[wasm_bindgen(js_name = "sign")]
    pub fn sign(
        &self,
        data: &str,
        priv_key: &str,
        pub_key: &str,
    ) -> Result<JsValue, JsValue> {
        let json_data: serde_json::Value =
            serde_json::from_str(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let signed = sea::sign(&json_data, priv_key, pub_key)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(JsValue::from_str(&signed))
    }

    /// Verify a signed message.
    ///
    /// - `message`: `SEA{...}` signed message from `sign()`
    /// - `pub_key`: public signing key
    ///
    /// Returns: the original JSON data, or throws on verification failure
    #[wasm_bindgen(js_name = "verify")]
    pub fn verify(&self, message: &str, pub_key: &str) -> Result<JsValue, JsValue> {
        let data =
            sea::verify(message, pub_key).map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(JsValue::from_str(&data.to_string()))
    }

    /// Encrypt data.
    ///
    /// - `data`: JSON-encoded data to encrypt
    /// - `key`: encryption key (epriv, shared secret, or passphrase)
    ///
    /// Returns: `SEA{...}` encrypted message string
    #[wasm_bindgen(js_name = "encrypt")]
    pub fn encrypt(&self, data: &str, key: &str) -> Result<JsValue, JsValue> {
        let json_data: serde_json::Value =
            serde_json::from_str(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let encrypted = sea::encrypt(&json_data, key)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(JsValue::from_str(&encrypted))
    }

    /// Decrypt data.
    ///
    /// - `message`: `SEA{...}` encrypted message from `encrypt()`
    /// - `key`: same key used to encrypt
    ///
    /// Returns: the original JSON data string
    #[wasm_bindgen(js_name = "decrypt")]
    pub fn decrypt(&self, message: &str, key: &str) -> Result<JsValue, JsValue> {
        let data =
            sea::decrypt(message, key).map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(JsValue::from_str(&data.to_string()))
    }

    /// Proof of Work / hash via PBKDF2.
    ///
    /// - `data`: data to hash
    /// - `salt`: optional salt (null for random)
    ///
    /// Returns: base64-encoded hash string
    #[wasm_bindgen(js_name = "work")]
    pub fn work(&self, data: &str, salt: Option<String>) -> Result<JsValue, JsValue> {
        let result = sea::work(data, salt.as_deref())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(JsValue::from_str(&result))
    }

    /// Derive shared secret via ECDH.
    ///
    /// - `their_epub`: other user's public encryption key
    /// - `my_epriv`: your private encryption key
    ///
    /// Returns: shared secret string (usable as encryption key)
    #[wasm_bindgen(js_name = "secret")]
    pub fn secret(&self, their_epub: &str, my_epriv: &str) -> Result<JsValue, JsValue> {
        let result = sea::secret(their_epub, my_epriv)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(JsValue::from_str(&result))
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WasmUser — Decentralized authentication
// ═══════════════════════════════════════════════════════════════════════

/// User authentication exposed to JavaScript.
///
/// ```js
/// const gun = new WasmGun();
/// const user = new WasmUser(gun);
///
/// // Create account
/// const result = JSON.parse(user.create("alice", "password123"));
/// if (result.ok !== undefined) {
///     console.log("Created! Public key:", result.pub);
/// }
///
/// // Auth
/// const auth = JSON.parse(user.auth("alice", "password123"));
///
/// // Write to user space
/// user.put("profile", '"Alice"');
///
/// // Check auth
/// const is = user.isAuthenticated(); // JSON or null
///
/// // Logout
/// user.leave();
/// ```
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmUser {
    inner: User,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmUser {
    /// Create a WasmUser tied to a WasmGun instance.
    #[wasm_bindgen(constructor)]
    pub fn new(gun: &WasmGun) -> Self {
        Self {
            inner: User::new(gun.inner.clone()),
        }
    }

    /// Create a new user account.
    ///
    /// Returns JSON: `{ ok: 0, pub: "..." }` or `{ err: "..." }`
    #[wasm_bindgen(js_name = "create")]
    pub fn create(&mut self, alias: &str, password: &str) -> JsValue {
        match self.inner.create(alias, password) {
            CreateResult::Ok { pub_key } => {
                let json = serde_json::json!({ "ok": 0, "pub": pub_key });
                JsValue::from_str(&json.to_string())
            }
            CreateResult::Err { err } => {
                let json = serde_json::json!({ "err": err });
                JsValue::from_str(&json.to_string())
            }
        }
    }

    /// Authenticate with alias and password.
    ///
    /// Returns JSON: `{ pub, epub, alias }` or `{ err: "..." }`
    #[wasm_bindgen(js_name = "auth")]
    pub fn auth(&mut self, alias: &str, password: &str) -> JsValue {
        match self.inner.auth_with_password(alias, password) {
            AuthResult::Ok(auth) => {
                let json = serde_json::json!({
                    "pub": auth.pub_key,
                    "epub": auth.epub,
                    "alias": auth.alias
                });
                JsValue::from_str(&json.to_string())
            }
            AuthResult::Err { err } => {
                let json = serde_json::json!({ "err": err });
                JsValue::from_str(&json.to_string())
            }
        }
    }

    /// Authenticate with a key pair (JSON: `{ pub, priv, epub, epriv }`).
    #[wasm_bindgen(js_name = "authPair")]
    pub fn auth_pair(&mut self, pair_json: &str) -> JsValue {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(pair_json);
        let parsed = match parsed {
            Ok(v) => v,
            Err(e) => {
                let json = serde_json::json!({ "err": e.to_string() });
                return JsValue::from_str(&json.to_string());
            }
        };

        // M15: Validate all four fields are present and non-empty
        let pub_key = parsed["pub"].as_str().unwrap_or("").to_string();
        let priv_key = parsed["priv"].as_str().unwrap_or("").to_string();
        let epub = parsed["epub"].as_str().unwrap_or("").to_string();
        let epriv = parsed["epriv"].as_str().unwrap_or("").to_string();

        if pub_key.is_empty() || priv_key.is_empty() || epub.is_empty() || epriv.is_empty() {
            let json = serde_json::json!({ "err": "Missing required key pair fields (pub, priv, epub, epriv)" });
            return JsValue::from_str(&json.to_string());
        }

        let pair = sea::SEAPair { pub_key, priv_key, epub, epriv };

        match self.inner.auth_with_pair(pair) {
            AuthResult::Ok(auth) => {
                let json = serde_json::json!({
                    "pub": auth.pub_key,
                    "epub": auth.epub,
                    "alias": auth.alias
                });
                JsValue::from_str(&json.to_string())
            }
            AuthResult::Err { err } => {
                let json = serde_json::json!({ "err": err });
                JsValue::from_str(&json.to_string())
            }
        }
    }

    /// The authenticated key pair as `{pub, priv, epub, epriv}` JSON
    /// (the `authPair` input shape), or null when not authenticated.
    /// This is how sessions restore without re-entering the password —
    /// the caller owns where it persists (and the risk of where).
    #[wasm_bindgen(js_name = "pairJson")]
    pub fn pair_json(&self) -> JsValue {
        match self.inner.is_authenticated() {
            Some(auth) => {
                let json = serde_json::json!({
                    "pub": auth.pair.pub_key,
                    "priv": auth.pair.priv_key,
                    "epub": auth.pair.epub,
                    "epriv": auth.pair.epriv,
                });
                JsValue::from_str(&json.to_string())
            }
            None => JsValue::NULL,
        }
    }

    /// Check if authenticated. Returns JSON `{ pub, epub, alias }` or null.
    #[wasm_bindgen(js_name = "isAuthenticated")]
    pub fn is_authenticated(&self) -> JsValue {
        match self.inner.is_authenticated() {
            Some(auth) => {
                let json = serde_json::json!({
                    "pub": auth.pub_key,
                    "epub": auth.epub,
                    "alias": auth.alias
                });
                JsValue::from_str(&json.to_string())
            }
            None => JsValue::NULL,
        }
    }

    /// Log out.
    #[wasm_bindgen(js_name = "leave")]
    pub fn leave(&mut self) {
        self.inner.leave();
    }

    /// Write to the user's namespace. Key is the property name.
    /// Value is a JSON-encoded string.
    #[wasm_bindgen(js_name = "put")]
    pub fn put(&self, key: &str, json_value: &str) -> Result<(), JsValue> {
        let chain = self
            .inner
            .get(key)
            .ok_or_else(|| JsValue::from_str("Not authenticated"))?;

        let parsed: serde_json::Value = serde_json::from_str(json_value)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        match wire::json_to_value(&parsed) {
            Some(val) => {
                chain.put_value(val);
                Ok(())
            }
            None => Err(JsValue::from_str("Invalid GUN value")),
        }
    }

    /// Read from the user's namespace.
    #[wasm_bindgen(js_name = "get")]
    pub fn get(&self, key: &str) -> JsValue {
        match self.inner.get(key) {
            Some(chain) => match chain.val() {
                Some(val) => JsValue::from_str(&wire::value_to_json(&val).to_string()),
                None => JsValue::NULL,
            },
            None => JsValue::NULL,
        }
    }

    /// Get the user's public key (or null if not authenticated).
    #[wasm_bindgen(js_name = "pubKey")]
    pub fn pub_key(&self) -> JsValue {
        match self.inner.is_authenticated() {
            Some(auth) => JsValue::from_str(&auth.pub_key),
            None => JsValue::NULL,
        }
    }

    /// Write a signed value to the user's namespace.
    ///
    /// The value is automatically signed with the user's private key.
    /// Metadata keys (pub, epub, alias, auth) are stored unsigned.
    #[wasm_bindgen(js_name = "putSigned")]
    pub fn put_signed(&self, key: &str, json_value: &str) -> Result<(), JsValue> {
        let signed_chain = self
            .inner
            .get_signed(key)
            .ok_or_else(|| JsValue::from_str("Not authenticated"))?;

        let parsed: serde_json::Value = serde_json::from_str(json_value)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        match wire::json_to_value(&parsed) {
            Some(val) => {
                signed_chain.put_value(val);
                Ok(())
            }
            None => Err(JsValue::from_str("Invalid GUN value")),
        }
    }

    /// Read and verify a signed value from the user's namespace.
    ///
    /// Returns the verified value (with signature stripped), or null if
    /// the value doesn't exist or verification fails.
    #[wasm_bindgen(js_name = "getSigned")]
    pub fn get_signed(&self, key: &str) -> JsValue {
        match self.inner.get_signed(key) {
            Some(signed_chain) => match signed_chain.val() {
                Some(val) => JsValue::from_str(&wire::value_to_json(&val).to_string()),
                None => JsValue::NULL,
            },
            None => JsValue::NULL,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WasmCert — Certificate management
// ═══════════════════════════════════════════════════════════════════════

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmCert;

#[cfg(target_arch = "wasm32")]
impl Default for WasmCert {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmCert {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self
    }

    /// Create a certificate granting write access.
    ///
    /// - `who`: public key of grantee, or `"*"` for anyone
    /// - `what`: path (exact), `"path/*"` (prefix), or `"*"` (all)
    /// - `expiry`: ms since epoch, or null/0 for no expiry
    /// - `issuer_pub`: issuer's public signing key
    /// - `issuer_priv`: issuer's private signing key
    ///
    /// Returns JSON: `{ who, what, expiry, issuer, signature }`
    #[wasm_bindgen(js_name = "create")]
    pub fn create(
        &self,
        who: &str,
        what: &str,
        expiry: Option<f64>,
        issuer_pub: &str,
        issuer_priv: &str,
    ) -> Result<JsValue, JsValue> {
        let cert_who = if who == "*" {
            CertWho::Anyone
        } else {
            CertWho::PubKey(who.to_string())
        };

        let cert_what = if what == "*" {
            CertWhat::All
        } else if what.ends_with('*') {
            CertWhat::Prefix(what.trim_end_matches('*').to_string())
        } else {
            CertWhat::Exact(what.to_string())
        };

        let exp = expiry.filter(|&e| e > 0.0);

        let cert = Certificate::create(cert_who, cert_what, exp, issuer_pub, issuer_priv)
            .map_err(|e| JsValue::from_str(&e))?;

        let json = serde_json::json!({
            "who": match &cert.who {
                CertWho::PubKey(pk) => pk.as_str(),
                CertWho::Anyone => "*",
            },
            "what": match &cert.what {
                CertWhat::Exact(p) => p.clone(),
                CertWhat::Prefix(p) => format!("{}*", p),
                CertWhat::All => "*".to_string(),
            },
            "expiry": cert.expiry,
            "issuer": cert.issuer,
            "signature": cert.signature,
        });
        Ok(JsValue::from_str(&json.to_string()))
    }

    /// Verify a certificate's signature.
    ///
    /// Takes the JSON returned by `create()`.
    /// Returns `true` if valid, throws on error.
    #[wasm_bindgen(js_name = "verify")]
    pub fn verify(&self, cert_json: &str) -> Result<bool, JsValue> {
        let parsed: serde_json::Value = serde_json::from_str(cert_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let who_str = parsed["who"].as_str().unwrap_or("");
        let what_str = parsed["what"].as_str().unwrap_or("");
        let issuer = parsed["issuer"].as_str().unwrap_or("");
        let signature = parsed["signature"].as_str().unwrap_or("");

        let who = if who_str == "*" {
            CertWho::Anyone
        } else {
            CertWho::PubKey(who_str.to_string())
        };

        let what = if what_str == "*" {
            CertWhat::All
        } else if what_str.ends_with('*') {
            CertWhat::Prefix(what_str.trim_end_matches('*').to_string())
        } else {
            CertWhat::Exact(what_str.to_string())
        };

        let expiry = parsed["expiry"].as_f64();

        let cert = Certificate {
            who,
            what,
            expiry,
            issuer: issuer.to_string(),
            signature: signature.to_string(),
        };

        cert.verify().map_err(|e| JsValue::from_str(&e))
    }

    /// Full read-side check: does this certificate (signature-valid,
    /// unexpired) grant `writer_pub` write access to `path`? This is
    /// what consumers run before trusting a cert-carrying write.
    #[wasm_bindgen(js_name = "grantsAccess")]
    pub fn grants_access(
        &self,
        cert_json: &str,
        writer_pub: &str,
        path: &str,
    ) -> Result<bool, JsValue> {
        let parsed: serde_json::Value = serde_json::from_str(cert_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let who_str = parsed["who"].as_str().unwrap_or("");
        let what_str = parsed["what"].as_str().unwrap_or("");

        let who = if who_str == "*" {
            CertWho::Anyone
        } else {
            CertWho::PubKey(who_str.to_string())
        };
        let what = if what_str == "*" {
            CertWhat::All
        } else if what_str.ends_with('*') {
            CertWhat::Prefix(what_str.trim_end_matches('*').to_string())
        } else {
            CertWhat::Exact(what_str.to_string())
        };

        let cert = Certificate {
            who,
            what,
            expiry: parsed["expiry"].as_f64(),
            issuer: parsed["issuer"].as_str().unwrap_or("").to_string(),
            signature: parsed["signature"].as_str().unwrap_or("").to_string(),
        };

        if !cert.verify().map_err(|e| JsValue::from_str(&e))? {
            return Ok(false);
        }
        Ok(cert.grants_access(writer_pub, path, crate::state::now_ms()))
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WasmUUID — Time-sortable UUID generation
// ═══════════════════════════════════════════════════════════════════════

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = "generateUUID")]
pub fn wasm_generate_uuid() -> String {
    crate::uuid::generate_uuid()
}

// ═══════════════════════════════════════════════════════════════════════
// WasmEvictionConfig — Graph eviction configuration
// ═══════════════════════════════════════════════════════════════════════

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmEvictionConfig {
    max_nodes: usize,
    max_keys: usize,
    eviction_fraction: f64,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmEvictionConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(max_nodes: usize, max_keys: usize, eviction_fraction: f64) -> Self {
        Self {
            max_nodes,
            max_keys,
            eviction_fraction,
        }
    }

    #[wasm_bindgen(js_name = "default")]
    pub fn default_config() -> Self {
        let d = crate::graph::EvictionConfig::default();
        Self {
            max_nodes: d.max_nodes,
            max_keys: d.max_keys,
            eviction_fraction: d.eviction_fraction,
        }
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> JsValue {
        let json = serde_json::json!({
            "maxNodes": self.max_nodes,
            "maxKeys": self.max_keys,
            "evictionFraction": self.eviction_fraction,
        });
        JsValue::from_str(&json.to_string())
    }
}
