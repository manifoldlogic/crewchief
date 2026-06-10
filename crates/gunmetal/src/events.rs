//! Event system — reactive pub-sub backbone for GUN.
//!
//! GUN's architecture is fundamentally event-driven. Data flows through
//! event listeners on named tags. This is modeled after the `onto` module
//! in the JS source, which implements a linked-list event emitter.
//!
//! In Rust, we implement this with a simpler callback registry pattern
//! using boxed closures keyed by listener ID.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::concurrency::MaybeSend;
use crate::types::GunValue;

/// Globally unique listener ID counter.
static NEXT_LISTENER_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    NEXT_LISTENER_ID.fetch_add(1, Ordering::Relaxed)
}

/// A handle returned when registering a listener, used to unsubscribe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListenerId(pub u64);

/// An event payload carrying data through the system.
#[derive(Debug, Clone)]
pub struct Event {
    /// The soul of the node this event pertains to.
    pub soul: String,
    /// The key that changed (if applicable).
    pub key: Option<String>,
    /// The value (if applicable).
    pub value: Option<GunValue>,
    /// The state timestamp of the change.
    pub state: f64,
    /// Wire message ID (for dedup/ack correlation).
    pub msg_id: Option<String>,
    /// Acknowledgment reference.
    pub ack_id: Option<String>,
}

impl Event {
    /// Create a data event for a key change.
    pub fn data(soul: impl Into<String>, key: impl Into<String>, value: GunValue, state: f64) -> Self {
        Self {
            soul: soul.into(),
            key: Some(key.into()),
            value: Some(value),
            state,
            msg_id: None,
            ack_id: None,
        }
    }

    /// Create a node-level event (no specific key).
    pub fn node(soul: impl Into<String>) -> Self {
        Self {
            soul: soul.into(),
            key: None,
            value: None,
            state: 0.0,
            msg_id: None,
            ack_id: None,
        }
    }
}

/// Type alias for event listener callbacks.
///
/// `Send` on native so `Gun` (and anything holding an `EventBus`) is
/// `Send + Sync` and can be shared across threads — required by the mesh
/// and relay layers. On single-threaded WASM there is no `Send` bound so
/// `JsValue`-capturing closures work. Use the `MaybeSend` bound from
/// `concurrency` when accepting callbacks destined for the bus.
#[cfg(not(target_arch = "wasm32"))]
type Listener = Box<dyn FnMut(&Event) + Send>;
#[cfg(target_arch = "wasm32")]
type Listener = Box<dyn FnMut(&Event)>;

/// An event emitter for a specific tag/channel.
///
/// Listeners are stored in insertion order and called sequentially.
/// Each listener has a unique `ListenerId` for removal.
struct TagEmitter {
    listeners: Vec<(ListenerId, Listener)>,
}

impl TagEmitter {
    fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }

    /// Maximum listeners per tag to prevent unbounded accumulation (H8 fix).
    const MAX_LISTENERS: usize = 1000;

    fn add(&mut self, cb: Listener) -> ListenerId {
        let id = ListenerId(next_id());
        // H8: cap listener count per tag to prevent memory exhaustion
        if self.listeners.len() < Self::MAX_LISTENERS {
            self.listeners.push((id, cb));
        }
        id
    }

    fn remove(&mut self, id: ListenerId) -> bool {
        let len = self.listeners.len();
        self.listeners.retain(|(lid, _)| *lid != id);
        self.listeners.len() < len
    }

    fn emit(&mut self, event: &Event) {
        for (_, cb) in &mut self.listeners {
            cb(event);
        }
    }

    fn is_empty(&self) -> bool {
        self.listeners.is_empty()
    }

    fn len(&self) -> usize {
        self.listeners.len()
    }
}

/// The main event bus.
///
/// Supports named event tags (like GUN's `'in'`, `'out'`, `'put'`, `'get'`,
/// `'hi'`, `'bye'`). Listeners register on tags and receive events when
/// that tag is emitted.
///
/// Also supports soul-specific listeners: `on("soul#key", cb)` for
/// subscribing to changes on a specific node or property.
pub struct EventBus {
    tags: HashMap<String, TagEmitter>,
}

impl EventBus {
    /// Create a new empty event bus.
    pub fn new() -> Self {
        Self {
            tags: HashMap::new(),
        }
    }

    /// Register a listener on a named tag.
    ///
    /// Returns a `ListenerId` that can be used to unsubscribe.
    ///
    /// # Tags
    ///
    /// GUN uses these internal tags:
    /// - `"in"` — incoming data events
    /// - `"out"` — outgoing data events (to peers/storage)
    /// - `"put"` — data write events (after HAM resolution)
    /// - `"get"` — data read events
    /// - `"hi"` — peer connected
    /// - `"bye"` — peer disconnected
    ///
    /// User-facing listeners (`.on()`, `.once()`) use soul-specific tags.
    pub fn on(
        &mut self,
        tag: impl Into<String>,
        cb: impl FnMut(&Event) + MaybeSend + 'static,
    ) -> ListenerId {
        let tag = tag.into();
        self.tags
            .entry(tag)
            .or_insert_with(TagEmitter::new)
            .add(Box::new(cb))
    }

    /// Remove a listener by its ID.
    ///
    /// Returns `true` if the listener was found and removed.
    pub fn off(&mut self, tag: &str, id: ListenerId) -> bool {
        if let Some(emitter) = self.tags.get_mut(tag) {
            let removed = emitter.remove(id);
            if emitter.is_empty() {
                self.tags.remove(tag);
            }
            removed
        } else {
            false
        }
    }

    /// Register a one-shot listener that fires once.
    ///
    /// After firing, the closure short-circuits on subsequent calls.
    /// Callers should use the returned ListenerId with `off()` to reclaim
    /// the closure memory when done (M10: documented behavior).
    pub fn once(
        &mut self,
        tag: impl Into<String>,
        cb: impl FnOnce(&Event) + MaybeSend + 'static,
    ) -> ListenerId {
        let tag_str: String = tag.into();
        let mut cb_opt = Some(cb);

        self.on(tag_str, move |event| {
            if let Some(cb) = cb_opt.take() {
                cb(event);
            }
        })
    }

    /// Emit an event to all listeners on a tag.
    pub fn emit(&mut self, tag: &str, event: &Event) {
        if let Some(emitter) = self.tags.get_mut(tag) {
            emitter.emit(event);
        }
    }

    /// Emit an event to multiple tags.
    pub fn emit_multi(&mut self, tags: &[&str], event: &Event) {
        for tag in tags {
            self.emit(tag, event);
        }
    }

    /// Check if any listeners exist for a tag.
    pub fn has_listeners(&self, tag: &str) -> bool {
        self.tags.get(tag).is_some_and(|e| !e.is_empty())
    }

    /// Get the number of listeners on a tag.
    pub fn listener_count(&self, tag: &str) -> usize {
        self.tags.get(tag).map_or(0, |e| e.len())
    }

    /// Remove all listeners on a tag.
    pub fn off_all(&mut self, tag: &str) {
        self.tags.remove(tag);
    }

    /// Remove all listeners on all tags.
    pub fn clear(&mut self) {
        self.tags.clear();
    }

    /// Helper: generate a soul-specific tag for property subscriptions.
    ///
    /// Used by the chain API to create tags like `"soul"` or `"soul.key"`.
    /// Generate a soul-specific tag for property subscriptions.
    ///
    /// Uses `\0` as separator (cannot appear in valid GUN souls/keys)
    /// to prevent collision between soul `"a.b"` and soul `"a"` + key `"b"` (M7 fix).
    pub fn soul_tag(soul: &str, key: Option<&str>) -> String {
        match key {
            Some(k) => format!("{}\0{}", soul, k),
            None => soul.to_string(),
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn on_and_emit() {
        let mut bus = EventBus::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let r = received.clone();

        bus.on("put", move |event| {
            r.lock().unwrap().push(event.soul.clone());
        });

        bus.emit("put", &Event::node("mark"));
        bus.emit("put", &Event::node("alice"));

        let data = received.lock().unwrap();
        assert_eq!(*data, vec!["mark", "alice"]);
    }

    #[test]
    fn off_removes_listener() {
        let mut bus = EventBus::new();
        let count = Arc::new(Mutex::new(0));
        let c = count.clone();

        let id = bus.on("put", move |_| {
            *c.lock().unwrap() += 1;
        });

        bus.emit("put", &Event::node("a"));
        assert_eq!(*count.lock().unwrap(), 1);

        bus.off("put", id);
        bus.emit("put", &Event::node("b"));
        assert_eq!(*count.lock().unwrap(), 1); // not called again
    }

    #[test]
    fn multiple_listeners_same_tag() {
        let mut bus = EventBus::new();
        let count = Arc::new(Mutex::new(0));
        let c1 = count.clone();
        let c2 = count.clone();

        bus.on("in", move |_| { *c1.lock().unwrap() += 1; });
        bus.on("in", move |_| { *c2.lock().unwrap() += 10; });

        bus.emit("in", &Event::node("x"));
        assert_eq!(*count.lock().unwrap(), 11);
    }

    #[test]
    fn once_fires_only_once() {
        let mut bus = EventBus::new();
        let count = Arc::new(Mutex::new(0));
        let c = count.clone();

        bus.once("get", move |_| {
            *c.lock().unwrap() += 1;
        });

        bus.emit("get", &Event::node("a"));
        bus.emit("get", &Event::node("b"));
        bus.emit("get", &Event::node("c"));

        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[test]
    fn different_tags_independent() {
        let mut bus = EventBus::new();
        let put_count = Arc::new(Mutex::new(0));
        let get_count = Arc::new(Mutex::new(0));
        let pc = put_count.clone();
        let gc = get_count.clone();

        bus.on("put", move |_| { *pc.lock().unwrap() += 1; });
        bus.on("get", move |_| { *gc.lock().unwrap() += 1; });

        bus.emit("put", &Event::node("a"));
        bus.emit("put", &Event::node("b"));
        bus.emit("get", &Event::node("c"));

        assert_eq!(*put_count.lock().unwrap(), 2);
        assert_eq!(*get_count.lock().unwrap(), 1);
    }

    #[test]
    fn has_listeners() {
        let mut bus = EventBus::new();
        assert!(!bus.has_listeners("put"));

        let id = bus.on("put", |_| {});
        assert!(bus.has_listeners("put"));
        assert_eq!(bus.listener_count("put"), 1);

        bus.off("put", id);
        assert!(!bus.has_listeners("put"));
    }

    #[test]
    fn off_all_clears_tag() {
        let mut bus = EventBus::new();
        bus.on("put", |_| {});
        bus.on("put", |_| {});
        assert_eq!(bus.listener_count("put"), 2);

        bus.off_all("put");
        assert!(!bus.has_listeners("put"));
    }

    #[test]
    fn event_data_fields() {
        let mut bus = EventBus::new();
        let captured = Arc::new(Mutex::new(None));
        let c = captured.clone();

        bus.on("put", move |event| {
            *c.lock().unwrap() = Some((
                event.soul.clone(),
                event.key.clone(),
                event.value.clone(),
                event.state,
            ));
        });

        let event = Event::data("mark", "name", GunValue::Text("Mark".into()), 100.0);
        bus.emit("put", &event);

        let data = captured.lock().unwrap();
        let (soul, key, value, state) = data.as_ref().unwrap();
        assert_eq!(soul, "mark");
        assert_eq!(key.as_deref(), Some("name"));
        assert_eq!(*value, Some(GunValue::Text("Mark".into())));
        assert_eq!(*state, 100.0);
    }

    #[test]
    fn soul_tag_helper() {
        assert_eq!(EventBus::soul_tag("mark", None), "mark");
        assert_eq!(EventBus::soul_tag("mark", Some("name")), "mark\0name");
        // M7: these should NOT collide
        assert_ne!(
            EventBus::soul_tag("mark.name", None),
            EventBus::soul_tag("mark", Some("name"))
        );
    }

    #[test]
    fn emit_to_nonexistent_tag_is_noop() {
        let mut bus = EventBus::new();
        // Should not panic
        bus.emit("nonexistent", &Event::node("x"));
    }

    #[test]
    fn clear_removes_everything() {
        let mut bus = EventBus::new();
        bus.on("a", |_| {});
        bus.on("b", |_| {});
        bus.on("c", |_| {});
        bus.clear();
        assert!(!bus.has_listeners("a"));
        assert!(!bus.has_listeners("b"));
        assert!(!bus.has_listeners("c"));
    }
}
