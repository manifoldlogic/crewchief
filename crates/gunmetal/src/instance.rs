//! GunInstance — the user-facing API orchestrating graph, events, dedup, and wire.
//!
//! This is the Rust equivalent of `var gun = Gun(options)` and the chain API
//! (`gun.get().put().on().once()`). The JS chain is notoriously complex; this
//! implementation provides an equivalent API with Rust idioms.
//!
//! # Architecture
//!
//! `Gun` is a cheaply-cloneable handle (via shared pointers) to split state.
//! Each component has its own lock for fine-grained concurrency:
//! - `graph`: `SharedRead` (read-many/write-one)
//! - `events`: `SharedMut` (exclusive — callbacks are `FnMut`)
//! - `dup`: `SharedMut` (exclusive)
//!
//! **Key invariant:** Never hold graph lock while emitting events.
//! This prevents deadlocks when callbacks read the graph.

use crate::concurrency::{
    lock_mut, new_shared_mut, new_shared_read, read_lock, write_lock, SharedMut, SharedRead,
};
use crate::dup::Dup;
use crate::events::{Event, EventBus, ListenerId};
use crate::graph::{Graph, PutResult};
use crate::lex::Lex;
use crate::types::{GunValue, Soul};
use crate::wire;

/// Configuration options for a GUN instance.
///
/// Mirrors `GunOptions` from the JS source.
#[derive(Debug, Clone, Default)]
pub struct GunOptions {
    /// Peer URLs to connect to.
    pub peers: Vec<String>,
    /// Whether to persist to local storage (browser). Default: true.
    pub local_storage: bool,
    /// Whether to persist to disk via RAD (Node.js). Default: true.
    pub radisk: bool,
    /// Custom file path for storage.
    pub file: Option<String>,
}

/// A GUN database instance.
///
/// This is the root entry point, equivalent to `var gun = Gun(options)`.
/// It's cheaply cloneable — all clones share the same underlying state.
///
/// # Example (conceptual)
/// ```
/// use gunmetal::instance::{Gun, GunOptions};
/// use gunmetal::types::GunValue;
///
/// let gun = Gun::new(GunOptions::default());
///
/// // Write data
/// gun.get("mark").put_kv("name", GunValue::Text("Mark".into()));
/// gun.get("mark").put_kv("email", GunValue::Text("mark@gun.eco".into()));
///
/// // Read data
/// let name = gun.get("mark").get("name").val();
/// assert_eq!(name, Some(GunValue::Text("Mark".into())));
/// ```
#[derive(Clone)]
pub struct Gun {
    graph: SharedRead<Graph>,
    events: SharedMut<EventBus>,
    dup: SharedMut<Dup>,
    #[allow(dead_code)]
    options: GunOptions,
}

impl Gun {
    /// Create a new GUN instance with the given options.
    pub fn new(options: GunOptions) -> Self {
        Self {
            graph: new_shared_read(Graph::new()),
            events: new_shared_mut(EventBus::new()),
            dup: new_shared_mut(Dup::new()),
            options,
        }
    }

    /// Create a GUN instance with default options.
    pub fn default_instance() -> Self {
        Self::new(GunOptions::default())
    }

    /// Navigate to a top-level node by soul.
    ///
    /// Equivalent to `gun.get('key')` in JS.
    /// Returns a `GunChain` scoped to that node.
    pub fn get(&self, soul: impl Into<String>) -> GunChain {
        GunChain {
            gun: self.clone(),
            soul: soul.into(),
            key: None,
        }
    }

    /// Get the current state timestamp from the clock.
    pub fn state(&self) -> f64 {
        read_lock(&self.graph).clock.now()
    }

    /// Register a listener on an internal event tag.
    ///
    /// Tags: "in", "out", "put", "get", "hi", "bye"
    pub fn on_event(
        &self,
        tag: &str,
        cb: impl FnMut(&Event) + 'static,
    ) -> ListenerId {
        lock_mut(&self.events).on(tag, cb)
    }

    /// Remove a listener.
    pub fn off_event(&self, tag: &str, id: ListenerId) {
        lock_mut(&self.events).off(tag, id);
    }

    /// Process an incoming wire message from a peer.
    ///
    /// This is how external data enters the system — a PUT message from
    /// a peer is parsed, deduped, HAM-resolved, and merged into the graph.
    /// Listeners are notified of accepted changes.
    ///
    /// If the message contains a GET request, returns a response PUT message
    /// with the requested data and an `@` ack field referencing the request.
    pub fn receive(&self, msg: &wire::WireMessage) -> Option<wire::WireMessage> {
        // Step 1: Dedup check (dup lock only)
        if let Some(ref id) = msg.id {
            if lock_mut(&self.dup).track(id.clone()) {
                return None; // already seen
            }
        }

        // Step 2: Graph merge (graph write lock only)
        let pending_events = {
            let mut graph = write_lock(&self.graph);
            let mut events = Vec::new();

            if let Some(ref put_data) = msg.put {
                let nodes = wire::json_to_graph(put_data);
                for node in &nodes {
                    let soul = node.soul().to_string();

                    // Verify signatures for user namespace writes (~pubKey/...)
                    if soul.starts_with('~') {
                        if let Some(verified_node) = Self::verify_user_node(node) {
                            let results = graph.merge_node(&verified_node);
                            for (key, result) in results {
                                if result == PutResult::Accepted {
                                    let value = verified_node
                                        .get(&key)
                                        .cloned()
                                        .unwrap_or(GunValue::Null);
                                    let state = verified_node.state_of(&key);
                                    events.push(Event::data(&soul, &key, value, state));
                                }
                            }
                        }
                        // If verification fails, silently drop the node
                        continue;
                    }

                    let results = graph.merge_node(node);
                    for (key, result) in results {
                        if result == PutResult::Accepted {
                            let value = node.get(&key).cloned().unwrap_or(GunValue::Null);
                            let state = node.state_of(&key);
                            events.push(Event::data(&soul, &key, value, state));
                        }
                    }
                }
            }

            events
        }; // graph lock released

        // Step 3: Emit events (events lock only, no graph lock held)
        self.emit_events(&pending_events);

        // Step 4: Handle GET requests — respond with data from graph
        if let Some(ref get_req) = msg.get {
            return self.handle_get(get_req, msg.id.as_deref());
        }

        None
    }

    /// Verify a node destined for a user namespace (`~pubKey/...`).
    ///
    /// For each key-value pair, if the value is a `SEA{...}` signed string,
    /// verify the signature against the pubkey from the soul. Metadata keys
    /// (`pub`, `epub`, `alias`, `auth`) are accepted unsigned.
    ///
    /// Returns the verified node (with only valid entries), or None if the
    /// soul format is invalid.
    fn verify_user_node(node: &crate::types::Node) -> Option<crate::types::Node> {
        let soul = node.soul();

        // Extract pubkey from soul (format: ~pubKey or ~pubKey/...)
        let pub_key = soul.strip_prefix('~')?;
        // Handle nested paths like ~pubKey/certs/id — pubkey is everything up to first /
        let pub_key = pub_key.split('/').next().unwrap_or(pub_key);

        if pub_key.is_empty() {
            return None;
        }

        let metadata_keys = ["pub", "epub", "alias", "auth"];
        let mut verified = crate::types::Node::new(soul);

        for (key, value) in node.iter() {
            let state = node.state_of(key);

            // Metadata keys are accepted unsigned
            if metadata_keys.contains(&key.as_str()) {
                verified.put(key, value.clone(), state);
                continue;
            }

            // For signed values, verify the signature
            if let GunValue::Text(text) = value {
                if text.starts_with("SEA{") {
                    match crate::sea::verify(text, pub_key) {
                        Ok(_) => {
                            // Signature valid — store the signed value as-is
                            verified.put(key, value.clone(), state);
                        }
                        Err(_) => {
                            // Signature invalid — skip this key
                            continue;
                        }
                    }
                    continue;
                }
            }

            // Non-signed, non-metadata values in user namespace:
            // Accept them (backward compatibility — not all data is signed yet)
            verified.put(key, value.clone(), state);
        }

        // Only return if we have at least one key
        if verified.len() > 0 {
            Some(verified)
        } else {
            None
        }
    }

    /// Handle a GET request by looking up data in the graph.
    ///
    /// Returns a PUT response with `@` ack field if data is found.
    fn handle_get(
        &self,
        get_req: &wire::GetRequest,
        request_id: Option<&str>,
    ) -> Option<wire::WireMessage> {
        let graph = read_lock(&self.graph);

        let node = graph.get_node(&get_req.soul)?;

        // Build response node with requested data
        let mut response_node = crate::types::Node::new(&get_req.soul);

        match &get_req.key {
            Some(key) => {
                // Specific key requested
                if let Some(value) = node.get(key) {
                    let state = node.state_of(key);
                    response_node.put(key, value.clone(), state);
                } else {
                    return None; // key not found
                }
            }
            None => {
                // Whole node requested
                for (key, value) in node.iter() {
                    let state = node.state_of(key);
                    response_node.put(key, value.clone(), state);
                }
            }
        }

        // Build PUT response with @ ack referencing the request
        let mut response = wire::put_message("get_response", &[&response_node]);
        response.ack = request_id.map(|id| id.to_string());

        Some(response)
    }

    /// Access the underlying graph (for inspection/testing).
    pub fn graph<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Graph) -> R,
    {
        let graph = read_lock(&self.graph);
        f(&graph)
    }

    /// Emit events without holding the graph lock.
    /// This is the key pattern that prevents re-entrant deadlocks:
    /// callbacks can safely read the graph since only the events lock is held
    /// during emission, and it's released between each emit call.
    fn emit_events(&self, events: &[Event]) {
        for event in events {
            // Lock per-emit to allow callback re-entrancy patterns
            lock_mut(&self.events).emit("put", event);

            let soul_tag = EventBus::soul_tag(&event.soul, None);
            lock_mut(&self.events).emit(&soul_tag, event);

            if let Some(ref key) = event.key {
                let key_tag = EventBus::soul_tag(&event.soul, Some(key));
                lock_mut(&self.events).emit(&key_tag, event);
            }
        }
    }
}

/// A chain reference scoped to a specific path in the graph.
///
/// Created by `gun.get("soul")` or `chain.get("key")`.
/// This is the Rust equivalent of GUN's chaining API.
///
/// A chain always has a `soul` (the top-level node ID) and optionally
/// a `key` (a property within that node).
#[derive(Clone)]
pub struct GunChain {
    gun: Gun,
    soul: Soul,
    key: Option<String>,
}

impl GunChain {
    /// Navigate deeper into the graph by key.
    ///
    /// - If this chain is at a soul level, `.get("key")` scopes to that property.
    /// - If this chain is already at a property and the property is a link,
    ///   `.get("key")` follows the link and scopes to the linked node's property.
    ///
    /// Equivalent to `gun.get('soul').get('key')` in JS.
    pub fn get(&self, key: impl Into<String>) -> GunChain {
        let key_str = key.into();

        if self.key.is_none() {
            // soul-level chain → scope to property
            return GunChain {
                gun: self.gun.clone(),
                soul: self.soul.clone(),
                key: Some(key_str),
            };
        }

        // Property-level chain → check if current value is a link
        {
            let graph = read_lock(&self.gun.graph);
            if let Some(current_key) = &self.key {
                if let Some(GunValue::Link(target)) = graph.get(&self.soul, current_key) {
                    // Follow the link to the target node
                    return GunChain {
                        gun: self.gun.clone(),
                        soul: target.clone(),
                        key: Some(key_str),
                    };
                }
            }
        } // graph lock released

        // Not a link — treat as nested path (soul/key)
        GunChain {
            gun: self.gun.clone(),
            soul: self.soul.clone(),
            key: Some(key_str),
        }
    }

    /// Put an object (multiple key-value pairs) at this node.
    ///
    /// Only valid on a soul-level chain (no key).
    /// Equivalent to `gun.get('soul').put({k1: v1, k2: v2})` in JS.
    pub fn put(&self, data: impl IntoIterator<Item = (String, GunValue)>) -> &Self {
        let pending_events = {
            let mut graph = write_lock(&self.gun.graph);
            let results = graph.put_node(&self.soul, data);
            let mut events = Vec::new();

            for (key, result) in &results {
                if *result == PutResult::Accepted {
                    if let Some(value) = graph.get(&self.soul, key) {
                        let state = graph
                            .get_node(&self.soul)
                            .map(|n| n.state_of(key))
                            .unwrap_or(0.0);
                        events.push(Event::data(&self.soul, key, value.clone(), state));
                    }
                }
            }
            events
        }; // graph lock released

        self.gun.emit_events(&pending_events);
        self
    }

    /// Put a single key-value pair.
    ///
    /// If this chain is scoped to a key, writes that key on the node.
    /// If scoped to a soul, writes the key-value on that node.
    ///
    /// Equivalent to `gun.get('soul').get('key').put(value)` or
    /// `gun.get('soul').put({key: value})`.
    pub fn put_kv(&self, key: impl Into<String>, value: GunValue) -> &Self {
        let key = key.into();
        self.put(std::iter::once((key, value)))
    }

    /// Put a value at the current chain position.
    ///
    /// Only valid on a key-scoped chain.
    /// Equivalent to `gun.get('soul').get('key').put(value)` in JS.
    pub fn put_value(&self, value: GunValue) -> &Self {
        if let Some(ref key) = self.key {
            let pending = {
                let mut graph = write_lock(&self.gun.graph);
                let result = graph.put(&self.soul, key, value.clone());

                if result == PutResult::Accepted {
                    let state = graph
                        .get_node(&self.soul)
                        .map(|n| n.state_of(key))
                        .unwrap_or(0.0);
                    vec![Event::data(&self.soul, key, value, state)]
                } else {
                    vec![]
                }
            }; // graph lock released
            self.gun.emit_events(&pending);
        }
        self
    }

    /// Read the current value synchronously.
    ///
    /// Returns `None` if the data hasn't been loaded or doesn't exist.
    /// This is a Rust convenience not directly in the JS API (which is async).
    pub fn val(&self) -> Option<GunValue> {
        let graph = read_lock(&self.gun.graph);
        match &self.key {
            Some(key) => graph.get(&self.soul, key).cloned(),
            None => {
                // Return a Link to self if node exists
                if graph.get_node(&self.soul).is_some() {
                    Some(GunValue::Link(self.soul.clone()))
                } else {
                    None
                }
            }
        }
    }

    /// Get the full node data as key-value pairs.
    ///
    /// Returns None if the node doesn't exist.
    pub fn node_data(&self) -> Option<Vec<(String, GunValue)>> {
        let graph = read_lock(&self.gun.graph);
        graph.get_node(&self.soul).map(|node| {
            node.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
    }

    /// Subscribe to realtime updates.
    ///
    /// Equivalent to `.on(callback)` in JS. The callback fires once
    /// immediately with current data (if any) and again on every change.
    ///
    /// Auto-pins the soul in the graph to prevent eviction while
    /// the subscription is active. Call `.off()` to unsubscribe and unpin.
    ///
    /// Returns a `ListenerId` for unsubscribing with `.off()`.
    pub fn on(&self, mut cb: impl FnMut(GunValue, String) + 'static) -> ListenerId {
        let tag = match &self.key {
            Some(key) => EventBus::soul_tag(&self.soul, Some(key)),
            None => EventBus::soul_tag(&self.soul, None),
        };

        // Auto-pin this soul to prevent eviction (graph write lock)
        {
            let mut graph = write_lock(&self.gun.graph);
            graph.pin(self.soul.clone());
        }

        // Fire immediately with current data (graph read lock)
        {
            let graph = read_lock(&self.gun.graph);
            match &self.key {
                Some(key) => {
                    if let Some(value) = graph.get(&self.soul, key) {
                        cb(value.clone(), key.clone());
                    }
                }
                None => {
                    if let Some(node) = graph.get_node(&self.soul) {
                        for (k, v) in node.iter() {
                            cb(v.clone(), k.clone());
                        }
                    }
                }
            }
        } // graph lock released

        // Register for future updates (events lock)
        let key_filter = self.key.clone();
        lock_mut(&self.gun.events).on(tag, move |event| {
            if let (Some(value), Some(event_key)) = (&event.value, &event.key) {
                match &key_filter {
                    Some(_) => {
                        // Key-scoped: fire with the value
                        cb(value.clone(), event_key.clone());
                    }
                    None => {
                        // Soul-scoped: fire for each key change
                        cb(value.clone(), event_key.clone());
                    }
                }
            }
        })
    }

    /// Get the current data once without subscribing.
    ///
    /// Equivalent to `.once(callback)` in JS. Fires once with current data.
    pub fn once(&self, cb: impl FnOnce(Option<GunValue>, String) + 'static) {
        let graph = read_lock(&self.gun.graph);
        match &self.key {
            Some(key) => {
                let value = graph.get(&self.soul, key).cloned();
                cb(value, key.clone());
            }
            None => {
                let value = if graph.get_node(&self.soul).is_some() {
                    Some(GunValue::Link(self.soul.clone()))
                } else {
                    None
                };
                cb(value, self.soul.clone());
            }
        }
    }

    /// Unsubscribe a listener created by `.on()`.
    ///
    /// Unpins the soul from the graph, allowing it to be evicted
    /// if no other listeners are active on this soul.
    pub fn off(&self, id: ListenerId) {
        let tag = match &self.key {
            Some(key) => EventBus::soul_tag(&self.soul, Some(key)),
            None => EventBus::soul_tag(&self.soul, None),
        };
        lock_mut(&self.gun.events).off(&tag, id);

        // Unpin the soul now that this listener is removed.
        // Check if any other listeners exist for this soul before unpinning.
        let soul_tag = EventBus::soul_tag(&self.soul, None);
        let has_soul_listeners = lock_mut(&self.gun.events).has_listeners(&soul_tag);
        if !has_soul_listeners {
            // Also check if any key-specific listeners remain
            // (we can't enumerate all possible keys, but the soul-level
            // listener covers the common case)
            let mut graph = write_lock(&self.gun.graph);
            graph.unpin(&self.soul);
        }
    }

    /// Add a unique item to this node (treated as a set/collection).
    ///
    /// Equivalent to `gun.get('list').set(item)` in JS.
    /// Generates a unique soul for the item and links it.
    pub fn set(&self, item: GunChain) -> GunChain {
        let item_soul = item.soul.clone();
        // Link the item into this node using the item's soul as the key
        self.put_kv(&item_soul, GunValue::Link(item_soul.clone()));
        item
    }

    /// Add a value to a set/collection with a generated UUID key.
    ///
    /// Like `.set()` but for primitive values instead of linked nodes.
    /// Returns the generated UUID key.
    pub fn set_value(&self, value: GunValue) -> String {
        let uuid = crate::uuid::generate_uuid();
        self.put_kv(&uuid, value);
        uuid
    }

    /// Iterate over each property/item, subscribing to updates and new items.
    ///
    /// Equivalent to `.map().on(cb)` in JS. Calls the callback for each
    /// existing key-value pair and for any future changes.
    ///
    /// Auto-pins the soul to prevent eviction while subscribed.
    /// Optionally filters by a LEX expression.
    pub fn map(
        &self,
        lex: Option<&Lex>,
        mut cb: impl FnMut(GunValue, String) + 'static,
    ) -> ListenerId {
        // Auto-pin this soul to prevent eviction
        {
            let mut graph = write_lock(&self.gun.graph);
            graph.pin(self.soul.clone());
        }

        // Fire for existing data (graph read lock)
        {
            let graph = read_lock(&self.gun.graph);
            if let Some(node) = graph.get_node(&self.soul) {
                for (k, v) in node.iter() {
                    if let Some(lex) = lex {
                        if !lex.matches(k) {
                            continue;
                        }
                    }
                    cb(v.clone(), k.clone());
                }
            }
        } // graph lock released

        // Subscribe to future changes (events lock)
        let soul_tag = EventBus::soul_tag(&self.soul, None);
        let lex_owned = lex.cloned();
        lock_mut(&self.gun.events).on(soul_tag, move |event| {
            if let (Some(value), Some(key)) = (&event.value, &event.key) {
                if let Some(ref lex) = lex_owned {
                    if !lex.matches(key) {
                        return;
                    }
                }
                cb(value.clone(), key.clone());
            }
        })
    }

    /// Move up to the parent chain.
    ///
    /// If at a key, returns the soul-level chain.
    /// If at a soul, returns the root gun instance.
    pub fn back(&self) -> GunChain {
        if self.key.is_some() {
            GunChain {
                gun: self.gun.clone(),
                soul: self.soul.clone(),
                key: None,
            }
        } else {
            // At root level, return self
            self.clone()
        }
    }

    /// Get the soul this chain is scoped to.
    pub fn soul(&self) -> &str {
        &self.soul
    }

    /// Get the key this chain is scoped to (if any).
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concurrency::new_shared_mut;
    use crate::types::Node;

    fn gun() -> Gun {
        Gun::default_instance()
    }

    #[test]
    fn put_and_val() {
        let g = gun();
        g.get("mark").put_kv("name", GunValue::Text("Mark".into()));
        let val = g.get("mark").get("name").val();
        assert_eq!(val, Some(GunValue::Text("Mark".into())));
    }

    #[test]
    fn put_object() {
        let g = gun();
        g.get("alice").put(vec![
            ("name".into(), GunValue::Text("Alice".into())),
            ("age".into(), GunValue::Number(30.0)),
        ]);
        assert_eq!(
            g.get("alice").get("name").val(),
            Some(GunValue::Text("Alice".into()))
        );
        assert_eq!(
            g.get("alice").get("age").val(),
            Some(GunValue::Number(30.0))
        );
    }

    #[test]
    fn partial_merge() {
        let g = gun();
        g.get("mark").put_kv("name", GunValue::Text("Mark".into()));
        g.get("mark").put_kv("age", GunValue::Number(30.0));

        // Both fields should exist
        assert_eq!(
            g.get("mark").get("name").val(),
            Some(GunValue::Text("Mark".into()))
        );
        assert_eq!(
            g.get("mark").get("age").val(),
            Some(GunValue::Number(30.0))
        );
    }

    #[test]
    fn link_traversal() {
        let g = gun();
        g.get("mark").put_kv("name", GunValue::Text("Mark".into()));
        g.get("mark").put_kv("boss", GunValue::Link("fluffy".into()));
        g.get("fluffy").put_kv("name", GunValue::Text("Fluffy".into()));

        // Traverse the link
        let boss_name = g.get("mark").get("boss").get("name").val();
        assert_eq!(boss_name, Some(GunValue::Text("Fluffy".into())));
    }

    #[test]
    fn on_fires_immediately_and_on_update() {
        let g = gun();
        g.get("mark").put_kv("name", GunValue::Text("Mark".into()));

        let received = new_shared_mut(Vec::new());
        let r = received.clone();

        let _id = g.get("mark").get("name").on(move |val, _key| {
            lock_mut(&r).push(val);
        });

        // Should have fired immediately
        assert_eq!(lock_mut(&received).len(), 1);
        assert_eq!(
            lock_mut(&received)[0],
            GunValue::Text("Mark".into())
        );

        // Update — should fire again
        g.get("mark").put_kv("name", GunValue::Text("Mark Nadal".into()));
        assert_eq!(lock_mut(&received).len(), 2);
        assert_eq!(
            lock_mut(&received)[1],
            GunValue::Text("Mark Nadal".into())
        );
    }

    #[test]
    fn once_fires_once() {
        let g = gun();
        g.get("mark").put_kv("name", GunValue::Text("Mark".into()));

        let received = new_shared_mut(None);
        let r = received.clone();

        g.get("mark").get("name").once(move |val, _key| {
            *lock_mut(&r) = val;
        });

        assert_eq!(
            *lock_mut(&received),
            Some(GunValue::Text("Mark".into()))
        );
    }

    #[test]
    fn off_unsubscribes() {
        let g = gun();
        let count = new_shared_mut(0);
        let c = count.clone();

        let id = g.get("mark").get("name").on(move |_val, _key| {
            *lock_mut(&c) += 1;
        });

        g.get("mark").put_kv("name", GunValue::Text("v1".into()));
        let c1 = *lock_mut(&count);

        g.get("mark").get("name").off(id);
        g.get("mark").put_kv("name", GunValue::Text("v2".into()));
        let c2 = *lock_mut(&count);

        assert_eq!(c1, c2); // no new calls after off
    }

    #[test]
    fn set_adds_to_collection() {
        let g = gun();
        let alice = g.get("alice");
        alice.put_kv("name", GunValue::Text("Alice".into()));

        let bob = g.get("bob");
        bob.put_kv("name", GunValue::Text("Bob".into()));

        g.get("users").set(alice);
        g.get("users").set(bob);

        // Users node should have links
        let users = g.get("users").node_data().unwrap();
        assert_eq!(users.len(), 2);
        assert!(users.iter().any(|(_, v)| *v == GunValue::Link("alice".into())));
        assert!(users.iter().any(|(_, v)| *v == GunValue::Link("bob".into())));
    }

    #[test]
    fn set_value_adds_with_uuid() {
        let g = gun();
        let key1 = g.get("items").set_value(GunValue::Text("first".into()));
        let key2 = g.get("items").set_value(GunValue::Text("second".into()));

        // Keys should be unique UUIDs
        assert_ne!(key1, key2);
        assert!(!key1.is_empty());

        // Values should be retrievable
        assert_eq!(
            g.get("items").get(&key1).val(),
            Some(GunValue::Text("first".into()))
        );
        assert_eq!(
            g.get("items").get(&key2).val(),
            Some(GunValue::Text("second".into()))
        );
    }

    #[test]
    fn map_iterates_and_subscribes() {
        let g = gun();
        g.get("scores").put_kv("alice", GunValue::Number(95.0));
        g.get("scores").put_kv("bob", GunValue::Number(87.0));

        let received = new_shared_mut(Vec::new());
        let r = received.clone();

        let _id = g.get("scores").map(None, move |val, key| {
            lock_mut(&r).push((key, val));
        });

        // Should have received existing entries
        assert_eq!(lock_mut(&received).len(), 2);

        // Add new entry — should fire
        g.get("scores").put_kv("charlie", GunValue::Number(92.0));
        assert_eq!(lock_mut(&received).len(), 3);
    }

    #[test]
    fn map_with_lex_filter() {
        let g = gun();
        g.get("chat").put_kv("2024/01/01", GunValue::Text("hello".into()));
        g.get("chat").put_kv("2024/01/15", GunValue::Text("world".into()));
        g.get("chat").put_kv("2024/02/01", GunValue::Text("february".into()));

        let received = new_shared_mut(Vec::new());
        let r = received.clone();

        let lex = Lex::prefix("2024/01/");
        let _id = g.get("chat").map(Some(&lex), move |val, key| {
            lock_mut(&r).push((key, val));
        });

        // Only January entries
        assert_eq!(lock_mut(&received).len(), 2);
    }

    #[test]
    fn null_tombstone() {
        let g = gun();
        g.get("mark").put_kv("name", GunValue::Text("Mark".into()));
        g.get("mark").get("name").put_value(GunValue::Null);

        assert_eq!(g.get("mark").get("name").val(), Some(GunValue::Null));
    }

    #[test]
    fn receive_wire_message() {
        let g = gun();

        // Simulate incoming PUT from a peer
        let mut node = Node::new("peer_data");
        node.put("status", GunValue::Text("online".into()), 100.0);

        let msg = wire::put_message("msg1", &[&node]);
        g.receive(&msg);

        assert_eq!(
            g.get("peer_data").get("status").val(),
            Some(GunValue::Text("online".into()))
        );
    }

    #[test]
    fn receive_deduplicates() {
        let g = gun();

        let mut node = Node::new("test");
        node.put("x", GunValue::Number(1.0), 100.0);

        let msg = wire::put_message("same_id", &[&node]);

        let count = new_shared_mut(0);
        let c = count.clone();
        g.on_event("put", move |_| {
            *lock_mut(&c) += 1;
        });

        g.receive(&msg);
        g.receive(&msg); // same ID — should be deduped

        assert_eq!(*lock_mut(&count), 1);
    }

    #[test]
    fn back_navigation() {
        let g = gun();
        let chain = g.get("mark").get("name");
        assert_eq!(chain.soul(), "mark");
        assert_eq!(chain.key(), Some("name"));

        let parent = chain.back();
        assert_eq!(parent.soul(), "mark");
        assert_eq!(parent.key(), None);
    }

    #[test]
    fn node_data_returns_all_fields() {
        let g = gun();
        g.get("alice").put(vec![
            ("name".into(), GunValue::Text("Alice".into())),
            ("age".into(), GunValue::Number(25.0)),
            ("active".into(), GunValue::Bool(true)),
        ]);

        let data = g.get("alice").node_data().unwrap();
        assert_eq!(data.len(), 3);
    }

    #[test]
    fn nonexistent_val_is_none() {
        let g = gun();
        assert_eq!(g.get("nobody").get("nothing").val(), None);
    }

    #[test]
    fn state_returns_monotonic_timestamp() {
        let g = gun();
        let t1 = g.state();
        let t2 = g.state();
        assert!(t2 > t1);
    }

    #[test]
    fn clone_shares_state() {
        let g1 = gun();
        g1.get("shared").put_kv("x", GunValue::Number(42.0));

        let g2 = g1.clone();
        assert_eq!(
            g2.get("shared").get("x").val(),
            Some(GunValue::Number(42.0))
        );
    }

    #[test]
    fn callback_can_read_graph_during_emission() {
        // This test proves split locks work: callbacks can read the graph
        // during event emission without deadlocking. With the old single
        // Arc<Mutex<GunInner>>, this would deadlock.
        let g = gun();
        g.get("mark").put_kv("name", GunValue::Text("Mark".into()));

        let g2 = g.clone();
        let read_result = new_shared_mut(None);
        let r = read_result.clone();

        g.get("mark").get("age").on(move |_val, _key| {
            // Read the graph inside a callback — safe with split locks
            let name = g2.get("mark").get("name").val();
            *lock_mut(&r) = name;
        });

        g.get("mark").put_kv("age", GunValue::Number(30.0));

        // The callback should have read the name successfully
        assert_eq!(
            *lock_mut(&read_result),
            Some(GunValue::Text("Mark".into()))
        );
    }

    // ── GET handling tests ──────────────────────────────────────────

    #[test]
    fn get_returns_specific_key() {
        let g = gun();
        g.get("mark").put_kv("name", GunValue::Text("Mark".into()));
        g.get("mark").put_kv("age", GunValue::Number(30.0));

        let get_msg = wire::get_message("req1", "mark", Some("name"));
        let response = g.receive(&get_msg).unwrap();

        assert_eq!(response.ack.as_deref(), Some("req1"));
        assert!(response.put.is_some());

        // Parse the response and verify it contains only the requested key
        let nodes = wire::json_to_graph(response.put.as_ref().unwrap());
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].soul(), "mark");
        assert_eq!(
            nodes[0].get("name"),
            Some(&GunValue::Text("Mark".into()))
        );
        // Should NOT contain "age" since we only asked for "name"
        assert!(nodes[0].get("age").is_none());
    }

    #[test]
    fn get_returns_whole_node() {
        let g = gun();
        g.get("mark").put_kv("name", GunValue::Text("Mark".into()));
        g.get("mark").put_kv("age", GunValue::Number(30.0));

        let get_msg = wire::get_message("req2", "mark", None);
        let response = g.receive(&get_msg).unwrap();

        let nodes = wire::json_to_graph(response.put.as_ref().unwrap());
        assert_eq!(nodes.len(), 1);
        assert!(nodes[0].get("name").is_some());
        assert!(nodes[0].get("age").is_some());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let g = gun();
        let get_msg = wire::get_message("req3", "nobody", None);
        let response = g.receive(&get_msg);
        assert!(response.is_none());
    }

    #[test]
    fn get_nonexistent_key_returns_none() {
        let g = gun();
        g.get("mark").put_kv("name", GunValue::Text("Mark".into()));

        let get_msg = wire::get_message("req4", "mark", Some("missing"));
        let response = g.receive(&get_msg);
        assert!(response.is_none());
    }
}
