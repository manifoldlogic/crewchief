//! Extended chain API — Rust equivalents of GUN's `gun/lib/*` plugins.
//!
//! Gated behind the `extended-api` feature (on by default; disable with
//! `default-features = false` for a lean core). Mirrors:
//!
//! | Method | GUN source | Behavior |
//! |--------|-----------|----------|
//! | [`GunChain::path`] | `gun/lib/path.js` | Dot-notation traversal |
//! | [`GunChain::open`] | `gun/lib/open.js` | Deep recursive document loading |
//! | [`GunChain::load`] | `gun/lib/load.js` | One-shot deep document loading |
//! | [`GunChain::not`] | `gun/lib/not.js` | Absence detection |
//! | [`GunChain::unset`] | `gun/lib/unset.js` | Remove a node from a set |
//! | [`GunChain::then`] | `gun/lib/then.js` | Future-based reads |
//! | [`GunChain::promise`] | `gun/lib/then.js` | Future resolving `{put, get, gun}` |
//! | [`GunChain::later`] | `gun/lib/later.js` | Delayed full-depth snapshot |
//! | [`GunChain::bye`] | `gun/lib/bye.js` | Disconnect-write registration |
//!
//! # Debounce semantics for `open()`
//!
//! GUN's `open()` debounces with `setTimeout`. Gunmetal's event emission is
//! synchronous, so `wait` acts as a *coalescing window*: the first change
//! fires immediately, changes arriving within `wait` ms of the last callback
//! are batched, and the batch is delivered on the first event after the
//! window closes (or via [`OpenHandle::poll`]). Use `wait: 0.0` to fire on
//! every change.
//!
//! # Callback re-entrancy
//!
//! Like core `.on()` callbacks, `open()` callbacks run while the event bus
//! lock is held: reading the graph is safe, writing from inside the callback
//! is not. `not()`, `load()`, `then()`, and `later()` callbacks run outside
//! event emission and may freely write.

use std::collections::{BTreeSet, HashSet};
use std::time::Duration;

use serde_json::{Map, Value};

use crate::concurrency::{lock_mut, new_shared_mut, read_lock, MaybeSend, SharedMut};
use crate::events::ListenerId;
use crate::graph::Graph;
use crate::instance::GunChain;
use crate::types::{GunValue, Soul};
use crate::wire;

// ── path() ──────────────────────────────────────────────────────────

impl GunChain {
    /// Traverse a dot-delimited path: `path("a.b.c")` ≡ `.get("a").get("b").get("c")`.
    ///
    /// A single segment with no dots behaves identically to `.get(key)`.
    /// An empty string returns the current chain unchanged.
    pub fn path(&self, path: &str) -> GunChain {
        self.path_with_separator(path, '.')
    }

    /// Like [`path`](Self::path) but with a custom separator
    /// (e.g. `path_with_separator("themes/active/color", '/')`).
    ///
    /// Empty segments (leading/trailing/double separators) are skipped.
    pub fn path_with_separator(&self, path: &str, separator: char) -> GunChain {
        let mut chain = self.clone();
        for segment in path.split(separator) {
            if segment.is_empty() {
                continue;
            }
            chain = chain.get(segment);
        }
        chain
    }

    /// Array-of-segments variant of [`path`](Self::path) for dynamic paths
    /// (segments may contain dots without being split).
    pub fn path_segments(&self, segments: &[&str]) -> GunChain {
        let mut chain = self.clone();
        for segment in segments {
            if segment.is_empty() {
                continue;
            }
            chain = chain.get(*segment);
        }
        chain
    }
}

// ── open() / load() ─────────────────────────────────────────────────

/// Options for [`GunChain::open`] and [`GunChain::load`].
#[derive(Debug, Clone)]
pub struct OpenOptions {
    /// Coalescing window in milliseconds. Changes within `wait` ms of the
    /// last callback are batched. Default: 9.0 (GUN's default). Use 0.0 to
    /// fire on every change.
    pub wait: f64,
    /// Maximum link-follow depth. `None` = unlimited. `Some(0)` leaves all
    /// links unresolved (`{"#": soul}` markers).
    pub depth: Option<usize>,
    /// Unsubscribe after the first callback (this is `.load()`).
    pub once: bool,
    /// Include GUN metadata (`_` with soul and state vector) in the output.
    pub meta: bool,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            wait: 9.0,
            depth: None,
            once: false,
            meta: false,
        }
    }
}

/// Handle to an active [`GunChain::open`] subscription.
pub struct OpenHandle {
    state: SharedMut<OpenState>,
    listener: Option<(GunChain, ListenerId)>,
}

struct OpenState {
    chain: GunChain,
    opts: OpenOptions,
    /// Souls reachable from the root in the last assembly. Events for souls
    /// outside this set are ignored.
    reachable: HashSet<Soul>,
    last_fire_ms: Option<f64>,
    dirty: bool,
    done: bool,
    #[cfg(not(target_arch = "wasm32"))]
    cb: Box<dyn FnMut(&Value) + Send>,
    #[cfg(target_arch = "wasm32")]
    cb: Box<dyn FnMut(&Value)>,
}

impl OpenState {
    /// Assemble the document and fire the callback. Must be called while
    /// holding the state lock via the wrapper below.
    fn fire(&mut self) {
        let mut reachable = HashSet::new();
        let doc = {
            let graph = read_lock(&self.chain.gun.graph);
            assemble_document(&graph, &self.chain, &self.opts, &mut reachable)
        };
        self.reachable = reachable;
        self.dirty = false;
        self.last_fire_ms = Some(crate::state::now_ms());
        if self.opts.once {
            self.done = true;
        }
        (self.cb)(&doc);
    }
}

impl OpenHandle {
    /// Deliver any coalesced changes that arrived inside the `wait` window
    /// but have not been followed by another event. No-op if nothing is
    /// pending or the subscription already completed.
    pub fn poll(&self) {
        let mut state = lock_mut(&self.state);
        if state.dirty && !state.done {
            state.fire();
        }
    }

    /// Unsubscribe. Equivalent to GUN's `off` on an open chain.
    pub fn off(&mut self) {
        {
            let mut state = lock_mut(&self.state);
            state.done = true;
        }
        if let Some((chain, id)) = self.listener.take() {
            crate::concurrency::lock_mut(&chain.gun.events).off("put", id);
        }
    }
}

impl Drop for OpenHandle {
    fn drop(&mut self) {
        self.off();
    }
}

impl GunChain {
    /// Recursively load the full document at this chain position and fire
    /// `cb` on every change at any depth.
    ///
    /// - Follows all [`GunValue::Link`] references, assembling a
    ///   `serde_json::Value` tree with graph metadata stripped (unless
    ///   `opts.meta` is set).
    /// - Circular references are detected per-assembly: a soul already
    ///   visited on the current path is emitted as a `{"#": soul}` marker.
    /// - Fires immediately with the current data, then on subsequent
    ///   changes per the debounce semantics described in the module docs.
    ///
    /// Returns an [`OpenHandle`]; dropping it (or calling
    /// [`OpenHandle::off`]) unsubscribes.
    pub fn open(
        &self,
        opts: OpenOptions,
        cb: impl FnMut(&Value) + MaybeSend + 'static,
    ) -> OpenHandle {
        let state = new_shared_mut(OpenState {
            chain: self.clone(),
            opts: opts.clone(),
            reachable: HashSet::new(),
            last_fire_ms: None,
            dirty: false,
            done: false,
            cb: Box::new(cb),
        });

        // Initial fire with current data.
        let done_after_initial = {
            let mut s = lock_mut(&state);
            s.fire();
            s.done
        };

        if done_after_initial {
            // once-mode: never subscribe.
            return OpenHandle {
                state,
                listener: None,
            };
        }

        // Single global "put" listener filtered by the reachable set.
        // Registering one listener (instead of per-soul tags) means no
        // subscription mutation is needed from inside event emission.
        let state_for_listener = state.clone();
        let listener_id = lock_mut(&self.gun.events).on("put", move |event| {
            let mut s = lock_mut(&state_for_listener);
            if s.done || !s.reachable.contains(&event.soul) {
                return;
            }
            s.dirty = true;
            let now = crate::state::now_ms();
            let due = match s.last_fire_ms {
                None => true,
                Some(last) => now - last >= s.opts.wait,
            };
            if due {
                s.fire();
            }
        });

        OpenHandle {
            state,
            listener: Some((self.clone(), listener_id)),
        }
    }

    /// One-shot deep document load: [`open`](Self::open) with `once`,
    /// firing `cb` exactly once with the full document tree.
    pub fn load(&self, opts: OpenOptions, cb: impl FnOnce(Value) + 'static) {
        let opts = OpenOptions { once: true, ..opts };
        let mut reachable = HashSet::new();
        let doc = {
            let graph = read_lock(&self.gun.graph);
            assemble_document(&graph, self, &opts, &mut reachable)
        };
        cb(doc);
    }
}

/// Assemble the full document for a chain position, following links.
///
/// `reachable` collects every soul visited so change events can be filtered.
fn assemble_document(
    graph: &Graph,
    chain: &GunChain,
    opts: &OpenOptions,
    reachable: &mut HashSet<Soul>,
) -> Value {
    reachable.insert(chain.soul.clone());
    match &chain.key {
        Some(key) => match graph.get(&chain.soul, key) {
            Some(GunValue::Link(target)) => {
                let mut visiting = BTreeSet::new();
                visiting.insert(chain.soul.clone());
                assemble_node(graph, target, opts, opts.depth, &mut visiting, reachable)
            }
            Some(value) => wire::value_to_json(value),
            None => Value::Null,
        },
        None => {
            let mut visiting = BTreeSet::new();
            assemble_node(graph, &chain.soul, opts, opts.depth, &mut visiting, reachable)
        }
    }
}

/// Recursively assemble a node into a JSON object.
///
/// `visiting` tracks the souls on the current recursion path for cycle
/// detection; `depth` counts remaining link-follow levels.
fn assemble_node(
    graph: &Graph,
    soul: &str,
    opts: &OpenOptions,
    depth: Option<usize>,
    visiting: &mut BTreeSet<Soul>,
    reachable: &mut HashSet<Soul>,
) -> Value {
    reachable.insert(soul.to_string());

    let Some(node) = graph.get_node(soul) else {
        return link_marker(soul);
    };

    if !visiting.insert(soul.to_string()) {
        // Circular reference — emit a soul marker instead of recursing.
        return link_marker(soul);
    }

    let mut obj = Map::new();

    if opts.meta {
        let mut meta = Map::new();
        meta.insert("#".to_string(), Value::String(soul.to_string()));
        let mut states = Map::new();
        for (key, _) in node.iter() {
            if let Some(n) = serde_json::Number::from_f64(node.state_of(key)) {
                states.insert(key.clone(), Value::Number(n));
            }
        }
        meta.insert(">".to_string(), Value::Object(states));
        obj.insert("_".to_string(), Value::Object(meta));
    }

    for (key, value) in node.iter() {
        let json = match value {
            GunValue::Link(target) => match depth {
                Some(0) => link_marker(target),
                _ => assemble_node(
                    graph,
                    target,
                    opts,
                    depth.map(|d| d - 1),
                    visiting,
                    reachable,
                ),
            },
            other => wire::value_to_json(other),
        };
        obj.insert(key.clone(), json);
    }

    visiting.remove(soul);
    Value::Object(obj)
}

fn link_marker(soul: &str) -> Value {
    let mut m = Map::new();
    m.insert("#".to_string(), Value::String(soul.to_string()));
    Value::Object(m)
}

// ── not() ───────────────────────────────────────────────────────────

impl GunChain {
    /// Fire `cb` with the key name if no data exists at this chain position
    /// in the local graph (missing or tombstoned with `Null`).
    ///
    /// **Caveat (per GUN):** absence cannot be guaranteed in a distributed
    /// system — data may exist on a peer we have not heard from. Treat this
    /// as a hint, not a proof. For a peer round-trip window, use
    /// [`not_within`](Self::not_within).
    ///
    /// Returns the chain unchanged so calls can continue: GUN's
    /// `.not(cb).put(...)` pattern becomes `chain.not(|_| ...);` followed by
    /// writes inside the callback (the callback runs outside event emission,
    /// so writing from it is safe).
    pub fn not(&self, cb: impl FnOnce(&str)) -> &Self {
        if self.is_absent() {
            cb(self.not_key());
        }
        self
    }

    /// Async variant of [`not`](Self::not) that waits `timeout` first,
    /// giving connected peers at least one round-trip to deliver the data,
    /// then re-checks the local graph.
    pub async fn not_within(&self, timeout: Duration, cb: impl FnOnce(&str)) {
        crate::runtime::sleep_async(timeout).await;
        if self.is_absent() {
            cb(self.not_key());
        }
    }

    fn is_absent(&self) -> bool {
        let graph = read_lock(&self.gun.graph);
        match &self.key {
            Some(key) => matches!(graph.get(&self.soul, key), None | Some(GunValue::Null)),
            None => graph.get_node(&self.soul).is_none_or(|n| n.is_empty()),
        }
    }

    fn not_key(&self) -> &str {
        self.key.as_deref().unwrap_or(&self.soul)
    }
}

// ── unset() ─────────────────────────────────────────────────────────

impl GunChain {
    /// Remove a previously [`set`](Self::set) node from this set by nulling
    /// out its link. The target node itself is NOT deleted — only the
    /// reference in this set.
    ///
    /// No-op if the item is not linked in this set.
    pub fn unset(&self, item: &GunChain) -> &Self {
        let item_soul = item.soul().to_string();
        let is_member = {
            let graph = read_lock(&self.gun.graph);
            matches!(graph.get(&self.soul, &item_soul), Some(GunValue::Link(_)))
        };
        if is_member {
            self.put_kv(item_soul, GunValue::Null);
        }
        self
    }
}

// ── then() / promise() ──────────────────────────────────────────────

/// Resolution context for [`GunChain::promise`] — mirrors GUN's
/// `.promise()` resolving `{put, get, gun}`.
pub struct ChainContext {
    /// The data at the chain position (same as [`GunChain::then`]).
    pub put: Option<GunValue>,
    /// The key (or soul, for soul-scoped chains) of the position.
    pub get: String,
    /// The chain itself, for continuing after resolution.
    pub chain: GunChain,
}

impl GunChain {
    /// Future-based read: resolves with the data at the current chain
    /// position using `.once()` semantics (a single local snapshot).
    ///
    /// Rust equivalent of GUN's `.then()` Promise API. On WASM, see
    /// `WasmGunChain::then` for the `js_sys::Promise` binding.
    pub async fn then(&self) -> Option<GunValue> {
        self.val()
    }

    /// Like [`then`](Self::then) but resolves with a richer
    /// [`ChainContext`] (`{put, get, gun}` in GUN terms).
    pub async fn promise(&self) -> ChainContext {
        ChainContext {
            put: self.val(),
            get: self
                .key()
                .map(|k| k.to_string())
                .unwrap_or_else(|| self.soul().to_string()),
            chain: self.clone(),
        }
    }
}

// ── later() ─────────────────────────────────────────────────────────

impl GunChain {
    /// Fire `cb` after `delay` with a full-depth snapshot of the data
    /// (assembled like [`open`](Self::open)) and the key.
    ///
    /// Uses `tokio::time::sleep` on native and `setTimeout` on WASM (via
    /// the runtime abstraction). Exact timing is not guaranteed, and the
    /// timer does not survive process restarts.
    pub async fn later(&self, delay: Duration, cb: impl FnOnce(Value, &str)) {
        crate::runtime::sleep_async(delay).await;
        let opts = OpenOptions::default();
        let mut reachable = HashSet::new();
        let doc = {
            let graph = read_lock(&self.gun.graph);
            assemble_document(&graph, self, &opts, &mut reachable)
        };
        cb(doc, self.not_key());
    }
}

// ── bye() ───────────────────────────────────────────────────────────

/// Registration context returned by [`GunChain::bye`]. Exposes only
/// [`put`](ByeBuilder::put), mirroring GUN's `bye().put(data)`.
pub struct ByeBuilder {
    soul: Soul,
    key: Option<String>,
}

impl ByeBuilder {
    /// Build the wire message registering `value` to be written when this
    /// peer disconnects. Send the returned message to a relay; the relay
    /// (see `relay` module) stores it and applies the write on disconnect.
    ///
    /// Experimental in GUN itself — requires relay support to execute.
    pub fn put(&self, value: GunValue) -> wire::WireMessage {
        let mut node_obj = Map::new();
        match &self.key {
            Some(key) => {
                node_obj.insert(key.clone(), wire::value_to_json(&value));
            }
            None => {
                // Soul-scoped bye with a primitive: store under "" like GUN's
                // val-position puts. Links/objects should use key-scoped chains.
                node_obj.insert(String::new(), wire::value_to_json(&value));
            }
        }
        let mut bye_graph = Map::new();
        bye_graph.insert(self.soul.clone(), Value::Object(node_obj));

        wire::WireMessage {
            id: Some(crate::uuid::generate_uuid()),
            bye: Some(Value::Object(bye_graph)),
            ..Default::default()
        }
    }
}

impl GunChain {
    /// Register data to be written when this peer disconnects from a relay.
    ///
    /// Returns a [`ByeBuilder`]; call `.put(value)` to obtain the wire
    /// message to send to the relay. Requires relay support (Phase 3).
    pub fn bye(&self) -> ByeBuilder {
        ByeBuilder {
            soul: self.soul.clone(),
            key: self.key.clone(),
        }
    }
}

// ── tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::{Gun, GunOptions};

    fn gun() -> Gun {
        Gun::new(GunOptions::default())
    }

    // ── path ──

    #[test]
    fn path_splits_on_dots() {
        let g = gun();
        g.get("settings")
            .get("themes")
            .put_kv("active", GunValue::Text("dark".into()));
        // settings.themes is a nested path on the same node in this model;
        // verify path() resolves to the same position as chained get()s.
        let via_path = g.get("settings").path("themes.active");
        let via_get = g.get("settings").get("themes").get("active");
        assert_eq!(via_path.soul(), via_get.soul());
        assert_eq!(via_path.key(), via_get.key());
    }

    #[test]
    fn path_follows_links_like_get() {
        let g = gun();
        g.get("a").put_kv("next", GunValue::Link("b".into()));
        g.get("b").put_kv("val", GunValue::Text("found".into()));

        assert_eq!(
            g.get("a").path("next.val").val(),
            Some(GunValue::Text("found".into()))
        );
    }

    #[test]
    fn path_single_segment_equals_get() {
        let g = gun();
        g.get("mark").put_kv("name", GunValue::Text("Mark".into()));
        assert_eq!(
            g.get("mark").path("name").val(),
            g.get("mark").get("name").val()
        );
    }

    #[test]
    fn path_empty_returns_chain_unchanged() {
        let g = gun();
        let chain = g.get("mark");
        let same = chain.path("");
        assert_eq!(same.soul(), chain.soul());
        assert_eq!(same.key(), chain.key());
    }

    #[test]
    fn path_skips_empty_segments() {
        let g = gun();
        let chain = g.get("a").path("..b..");
        assert_eq!(chain.key(), Some("b"));
    }

    #[test]
    fn path_custom_separator() {
        let g = gun();
        g.get("t").put_kv("color", GunValue::Text("red".into()));
        let chain = g.get("t").path_with_separator("/color", '/');
        assert_eq!(chain.val(), Some(GunValue::Text("red".into())));
    }

    #[test]
    fn path_segments_preserves_dots() {
        let g = gun();
        g.get("files").put_kv("a.txt", GunValue::Text("hello".into()));
        let chain = g.get("files").path_segments(&["a.txt"]);
        assert_eq!(chain.val(), Some(GunValue::Text("hello".into())));
    }

    // ── open / load ──

    #[test]
    fn open_assembles_nested_document() {
        let g = gun();
        g.get("alice").put_kv("name", GunValue::Text("Alice".into()));
        g.get("alice")
            .put_kv("address", GunValue::Link("alice_addr".into()));
        g.get("alice_addr")
            .put_kv("city", GunValue::Text("Wonderland".into()));

        let received = new_shared_mut(Vec::new());
        let r = received.clone();
        let _handle = g.get("alice").open(
            OpenOptions {
                wait: 0.0,
                ..Default::default()
            },
            move |doc| {
                lock_mut(&r).push(doc.clone());
            },
        );

        let docs = lock_mut(&received);
        assert_eq!(docs.len(), 1, "open fires immediately");
        let doc = &docs[0];
        assert_eq!(doc["name"], "Alice");
        assert_eq!(doc["address"]["city"], "Wonderland");
        assert!(doc.get("_").is_none(), "metadata stripped");
    }

    #[test]
    fn open_fires_on_deep_change() {
        let g = gun();
        g.get("root").put_kv("child", GunValue::Link("child1".into()));
        g.get("child1").put_kv("x", GunValue::Number(1.0));

        let received = new_shared_mut(Vec::new());
        let r = received.clone();
        let _handle = g.get("root").open(
            OpenOptions {
                wait: 0.0,
                ..Default::default()
            },
            move |doc| {
                lock_mut(&r).push(doc.clone());
            },
        );

        // Change at depth 1 — must re-fire with the updated tree.
        g.get("child1").put_kv("x", GunValue::Number(2.0));

        let docs = lock_mut(&received);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[1]["child"]["x"], 2.0);
    }

    #[test]
    fn open_discovers_newly_linked_nodes() {
        let g = gun();
        g.get("root").put_kv("a", GunValue::Number(1.0));

        let received = new_shared_mut(Vec::new());
        let r = received.clone();
        let _handle = g.get("root").open(
            OpenOptions {
                wait: 0.0,
                ..Default::default()
            },
            move |doc| {
                lock_mut(&r).push(doc.clone());
            },
        );

        // Link a new node, then update it — both must fire.
        g.get("root").put_kv("sub", GunValue::Link("newsub".into()));
        g.get("newsub").put_kv("y", GunValue::Number(9.0));

        let docs = lock_mut(&received);
        assert_eq!(docs.len(), 3);
        assert_eq!(docs[2]["sub"]["y"], 9.0);
    }

    #[test]
    fn open_handles_circular_references() {
        let g = gun();
        g.get("alice").put_kv("name", GunValue::Text("Alice".into()));
        g.get("bob").put_kv("name", GunValue::Text("Bob".into()));
        g.get("alice").put_kv("friend", GunValue::Link("bob".into()));
        g.get("bob").put_kv("friend", GunValue::Link("alice".into()));

        let received = new_shared_mut(Vec::new());
        let r = received.clone();
        let _handle = g.get("alice").open(
            OpenOptions {
                wait: 0.0,
                ..Default::default()
            },
            move |doc| {
                lock_mut(&r).push(doc.clone());
            },
        );

        let docs = lock_mut(&received);
        let doc = &docs[0];
        assert_eq!(doc["friend"]["name"], "Bob");
        // Cycle back to alice is a soul marker, not infinite recursion.
        assert_eq!(doc["friend"]["friend"]["#"], "alice");
    }

    #[test]
    fn open_respects_depth_limit() {
        let g = gun();
        g.get("l0").put_kv("next", GunValue::Link("l1".into()));
        g.get("l1").put_kv("next", GunValue::Link("l2".into()));
        g.get("l2").put_kv("val", GunValue::Number(42.0));

        let received = new_shared_mut(Vec::new());
        let r = received.clone();
        let _handle = g.get("l0").open(
            OpenOptions {
                wait: 0.0,
                depth: Some(1),
                ..Default::default()
            },
            move |doc| {
                lock_mut(&r).push(doc.clone());
            },
        );

        let docs = lock_mut(&received);
        let doc = &docs[0];
        // depth 1: l1 resolved, l2 left as a marker
        assert_eq!(doc["next"]["next"]["#"], "l2");
        assert!(doc["next"]["next"].get("val").is_none());
    }

    #[test]
    fn open_wait_coalesces_rapid_updates() {
        let g = gun();
        g.get("counter").put_kv("v", GunValue::Number(0.0));

        let received = new_shared_mut(Vec::new());
        let r = received.clone();
        let handle = g.get("counter").open(
            // Large window: everything after the initial fire coalesces.
            OpenOptions {
                wait: 10_000.0,
                ..Default::default()
            },
            move |doc| {
                lock_mut(&r).push(doc.clone());
            },
        );

        for i in 1..=5 {
            g.get("counter").put_kv("v", GunValue::Number(i as f64));
        }
        // Rapid updates inside the window: only the initial fire so far.
        assert_eq!(lock_mut(&received).len(), 1);

        // poll() delivers the coalesced batch.
        handle.poll();
        let docs = lock_mut(&received);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[1]["v"], 5.0);
    }

    #[test]
    fn open_meta_includes_metadata() {
        let g = gun();
        g.get("m").put_kv("x", GunValue::Number(1.0));

        let received = new_shared_mut(Vec::new());
        let r = received.clone();
        let _handle = g.get("m").open(
            OpenOptions {
                wait: 0.0,
                meta: true,
                ..Default::default()
            },
            move |doc| {
                lock_mut(&r).push(doc.clone());
            },
        );

        let docs = lock_mut(&received);
        assert_eq!(docs[0]["_"]["#"], "m");
        assert!(docs[0]["_"][">"]["x"].is_number());
    }

    #[test]
    fn open_off_unsubscribes() {
        let g = gun();
        g.get("s").put_kv("x", GunValue::Number(1.0));

        let received = new_shared_mut(Vec::new());
        let r = received.clone();
        let mut handle = g.get("s").open(
            OpenOptions {
                wait: 0.0,
                ..Default::default()
            },
            move |doc| {
                lock_mut(&r).push(doc.clone());
            },
        );

        handle.off();
        g.get("s").put_kv("x", GunValue::Number(2.0));
        assert_eq!(lock_mut(&received).len(), 1, "no fires after off()");
    }

    #[test]
    fn open_key_scoped_primitive() {
        let g = gun();
        g.get("mark").put_kv("name", GunValue::Text("Mark".into()));

        let received = new_shared_mut(Vec::new());
        let r = received.clone();
        let _handle = g.get("mark").get("name").open(
            OpenOptions {
                wait: 0.0,
                ..Default::default()
            },
            move |doc| {
                lock_mut(&r).push(doc.clone());
            },
        );

        assert_eq!(lock_mut(&received)[0], Value::String("Mark".into()));
    }

    #[test]
    fn load_fires_exactly_once() {
        let g = gun();
        g.get("doc").put_kv("v", GunValue::Number(0.0));

        let received = new_shared_mut(Vec::new());
        let r = received.clone();
        g.get("doc").load(OpenOptions::default(), move |doc| {
            lock_mut(&r).push(doc);
        });

        g.get("doc").put_kv("v", GunValue::Number(1.0));
        g.get("doc").put_kv("v", GunValue::Number(2.0));

        let docs = lock_mut(&received);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0]["v"], 0.0);
    }

    // ── not ──

    #[test]
    fn not_fires_for_missing_data() {
        let g = gun();
        let fired = new_shared_mut(None);
        let f = fired.clone();
        g.get("users").get("alice").not(move |key| {
            *lock_mut(&f) = Some(key.to_string());
        });
        assert_eq!(*lock_mut(&fired), Some("alice".to_string()));
    }

    #[test]
    fn not_silent_when_data_exists() {
        let g = gun();
        g.get("users").put_kv("alice", GunValue::Text("here".into()));
        let fired = new_shared_mut(false);
        let f = fired.clone();
        g.get("users").get("alice").not(move |_| {
            *lock_mut(&f) = true;
        });
        assert!(!*lock_mut(&fired));
    }

    #[test]
    fn not_fires_for_null_tombstone() {
        let g = gun();
        g.get("users").put_kv("bob", GunValue::Null);
        let fired = new_shared_mut(false);
        let f = fired.clone();
        g.get("users").get("bob").not(move |_| {
            *lock_mut(&f) = true;
        });
        assert!(*lock_mut(&fired), "null tombstone counts as absent");
    }

    #[test]
    fn not_can_create_data_in_callback() {
        let g = gun();
        let g2 = g.clone();
        g.get("posts").get("welcome").not(move |_key| {
            g2.get("posts")
                .put_kv("welcome", GunValue::Text("First post!".into()));
        });
        assert_eq!(
            g.get("posts").get("welcome").val(),
            Some(GunValue::Text("First post!".into()))
        );
    }

    #[tokio::test]
    async fn not_within_waits_then_checks() {
        let g = gun();
        let fired = new_shared_mut(false);
        let f = fired.clone();
        g.get("ghost")
            .get("data")
            .not_within(Duration::from_millis(10), move |_| {
                *lock_mut(&f) = true;
            })
            .await;
        assert!(*lock_mut(&fired));
    }

    // ── unset ──

    #[test]
    fn unset_removes_set_member() {
        let g = gun();
        let fluffy = g.get("fluffy");
        fluffy.put_kv("name", GunValue::Text("Fluffy".into()));
        let whiskers = g.get("whiskers");
        whiskers.put_kv("name", GunValue::Text("Whiskers".into()));

        let cats = g.get("cats");
        cats.set(fluffy.clone());
        cats.set(whiskers);

        cats.unset(&fluffy);

        // The link is tombstoned…
        assert_eq!(cats.get("fluffy").val(), Some(GunValue::Null));
        assert_eq!(
            cats.get("whiskers").val(),
            Some(GunValue::Link("whiskers".into()))
        );
        // …but the target node is untouched.
        assert_eq!(
            g.get("fluffy").get("name").val(),
            Some(GunValue::Text("Fluffy".into()))
        );
    }

    #[test]
    fn unset_noop_for_non_member() {
        let g = gun();
        let cats = g.get("cats");
        let stray = g.get("stray");
        cats.unset(&stray);
        assert_eq!(cats.get("stray").val(), None, "no tombstone created");
    }

    // ── then / promise ──

    #[tokio::test]
    async fn then_resolves_with_data() {
        let g = gun();
        g.get("greeting")
            .put_kv("message", GunValue::Text("Hello, World!".into()));
        let data = g.get("greeting").get("message").then().await;
        assert_eq!(data, Some(GunValue::Text("Hello, World!".into())));
    }

    #[tokio::test]
    async fn then_resolves_none_for_missing() {
        let g = gun();
        assert_eq!(g.get("nobody").get("nothing").then().await, None);
    }

    #[tokio::test]
    async fn promise_resolves_context() {
        let g = gun();
        g.get("settings").put_kv("theme", GunValue::Text("dark".into()));
        let ctx = g.get("settings").get("theme").promise().await;
        assert_eq!(ctx.put, Some(GunValue::Text("dark".into())));
        assert_eq!(ctx.get, "theme");
        // Chain reference still usable.
        ctx.chain.put_value(GunValue::Text("light".into()));
        assert_eq!(
            g.get("settings").get("theme").val(),
            Some(GunValue::Text("light".into()))
        );
    }

    // ── later ──

    #[tokio::test]
    async fn later_fires_with_snapshot() {
        let g = gun();
        g.get("session").put_kv("token", GunValue::Text("xyz".into()));

        let received = new_shared_mut(None);
        let r = received.clone();
        g.get("session")
            .later(Duration::from_millis(20), move |doc, key| {
                *lock_mut(&r) = Some((doc, key.to_string()));
            })
            .await;

        let got = lock_mut(&received).take().unwrap();
        assert_eq!(got.0["token"], "xyz");
        assert_eq!(got.1, "session");
    }

    // ── bye ──

    #[test]
    fn bye_builds_registration_message() {
        let g = gun();
        let msg = g
            .get("users")
            .get("alice")
            .get("status")
            .bye()
            .put(GunValue::Text("offline".into()));

        assert!(msg.id.is_some());
        let bye = msg.bye.as_ref().unwrap();
        // get("users").get("alice") follows no link (none exists), so the
        // chain is soul "users" nested… actually status is the final key.
        let (soul, node) = bye.as_object().unwrap().iter().next().unwrap();
        assert!(!soul.is_empty());
        let (key, value) = node.as_object().unwrap().iter().next().unwrap();
        assert_eq!(key, "status");
        assert_eq!(value, &Value::String("offline".into()));
    }
}
