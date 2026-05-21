//! Sync — peer-to-peer replication for the GUN protocol.
//!
//! This module implements the sync layer that makes GUN a distributed database.
//! When data changes locally, outgoing messages are generated. When messages
//! arrive from peers, they're deduplicated, HAM-resolved, and merged.
//!
//! ## Architecture
//!
//! GUN's sync model from the JS source:
//! 1. Local write → HAM resolves → accepted changes emit on "put" event
//! 2. "put" event triggers "out" event → mesh broadcasts to all peers
//! 3. Peers receive → dedup check → HAM resolve → merge → emit "put"
//! 4. Peers forward to their peers (except sender) → mesh propagation
//!
//! In gunmetal, we use a `SyncAdapter` trait so transports (WebSocket,
//! WebRTC, in-memory) are pluggable.

use std::sync::{Arc, Mutex};

use crate::events::ListenerId;
use crate::instance::Gun;
use crate::types::Node;
use crate::wire;

/// Unique identifier for a peer connection.
pub type PeerId = String;

/// A message ready to be sent to peers.
#[derive(Debug, Clone)]
pub struct OutgoingMessage {
    /// The serialized JSON wire message.
    pub json: String,
    /// Peer to exclude from broadcast (the sender, to prevent echo).
    pub exclude: Option<PeerId>,
}

/// Trait for sync transport adapters.
///
/// Implement this to plug in WebSocket, WebRTC, or any transport.
pub trait SyncAdapter: 'static {
    /// Send a message to a specific peer.
    fn send(&mut self, peer: &str, msg: &str);

    /// Broadcast a message to all connected peers, optionally excluding one.
    fn broadcast(&mut self, msg: &str, exclude: Option<&str>);
}

/// An in-memory sync adapter for testing — directly connects two Gun instances.
pub struct MemorySync {
    peers: Vec<(PeerId, Gun)>,
    /// Local gun instance to receive GET responses routed back.
    local_gun: Option<Gun>,
}

impl MemorySync {
    pub fn new() -> Self {
        Self {
            peers: Vec::new(),
            local_gun: None,
        }
    }

    pub fn add_peer(&mut self, id: impl Into<PeerId>, gun: Gun) {
        self.peers.push((id.into(), gun));
    }

    /// Set the local Gun instance for receiving GET responses.
    pub fn set_local(&mut self, gun: Gun) {
        self.local_gun = Some(gun);
    }
}

impl SyncAdapter for MemorySync {
    fn send(&mut self, peer_id: &str, msg: &str) {
        for (id, gun) in &self.peers {
            if id == peer_id {
                if let Ok(wire_msg) = wire::parse_message(msg) {
                    gun.receive(&wire_msg);
                }
            }
        }
    }

    fn broadcast(&mut self, msg: &str, exclude: Option<&str>) {
        // Collect responses from GET handlers to send back
        let mut responses = Vec::new();

        for (id, gun) in &self.peers {
            if Some(id.as_str()) == exclude {
                continue;
            }
            if let Ok(wire_msg) = wire::parse_message(msg) {
                if let Some(response) = gun.receive(&wire_msg) {
                    // GET handler produced a response — route it back
                    if let Ok(json) = wire::serialize_message(&response) {
                        responses.push(json);
                    }
                }
            }
        }

        // Route GET responses back to the local gun instance
        if let Some(ref local) = self.local_gun {
            for json in &responses {
                if let Ok(wire_msg) = wire::parse_message(json) {
                    local.receive(&wire_msg);
                }
            }
        }
    }
}

impl Default for MemorySync {
    fn default() -> Self {
        Self::new()
    }
}

/// A sync manager that bridges a Gun instance to peers.
///
/// Listens for local data changes and queues outgoing messages.
/// Call `flush()` to send queued messages, or use `tick()` for
/// write-then-flush in one step.
pub struct SyncManager {
    gun: Gun,
    adapter: Box<dyn SyncAdapter>,
    outbox: Arc<Mutex<Vec<String>>>,
    _listener_id: Option<ListenerId>,
    msg_counter: Arc<Mutex<u64>>,
}

impl SyncManager {
    /// Create a new sync manager connecting a Gun instance to an adapter.
    ///
    /// Listens for local changes and queues outgoing messages.
    /// Call `flush()` after writes to send them to peers.
    pub fn new(gun: Gun, adapter: impl SyncAdapter) -> Self {
        let outbox: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let msg_counter = Arc::new(Mutex::new(0u64));

        // Listen for local "put" events and queue outgoing messages
        let outbox_clone = outbox.clone();
        let counter_clone = msg_counter.clone();
        let listener_id = gun.on_event("put", move |event| {
            if let (Some(value), Some(key)) = (&event.value, &event.key) {
                let mut counter = counter_clone.lock().unwrap_or_else(|e| e.into_inner());
                *counter += 1;
                let msg_id = format!("gm{}", *counter);

                let mut node = Node::new(&event.soul);
                node.put(key, value.clone(), event.state);

                let wire_msg = wire::put_message(&msg_id, &[&node]);
                if let Ok(json) = wire::serialize_message(&wire_msg) {
                    outbox_clone.lock().unwrap_or_else(|e| e.into_inner()).push(json);
                }
            }
        });

        Self {
            gun,
            adapter: Box::new(adapter),
            outbox,
            _listener_id: Some(listener_id),
            msg_counter,
        }
    }

    /// Send all queued outgoing messages to peers.
    ///
    /// This must be called after writes to actually sync data.
    /// It's separate from the write path to avoid deadlocks.
    pub fn flush(&mut self) {
        let messages: Vec<String> = {
            let mut outbox = self.outbox.lock().unwrap_or_else(|e| e.into_inner());
            outbox.drain(..).collect()
        };
        for json in messages {
            self.adapter.broadcast(&json, None);
        }
    }

    /// Get a reference to the Gun instance.
    pub fn gun(&self) -> &Gun {
        &self.gun
    }

    /// Manually send a GET request to peers for a specific soul.
    ///
    /// Broadcasts a GET message to all connected peers. Peers that have
    /// the requested data will respond with a PUT message containing the
    /// data, which gets merged into the local graph via the normal receive path.
    pub fn request(&mut self, soul: &str, key: Option<&str>) {
        let msg_id = {
            let mut counter = self.msg_counter.lock().unwrap_or_else(|e| e.into_inner());
            *counter += 1;
            format!("gm{}", *counter)
        };

        let msg = wire::get_message(&msg_id, soul, key);
        if let Ok(json) = wire::serialize_message(&msg) {
            self.adapter.broadcast(&json, None);
        }
    }
}

/// Connect two Gun instances for bidirectional sync.
///
/// Returns both SyncManagers. After writing data, call `flush()` on the
/// writer's SyncManager to send it to the other peer.
pub fn sync_pair(gun_a: Gun, gun_b: Gun) -> (SyncManager, SyncManager) {
    // A's adapter sends to B, routes GET responses back to A
    let mut adapter_a = MemorySync::new();
    adapter_a.add_peer("B", gun_b.clone());
    adapter_a.set_local(gun_a.clone());

    // B's adapter sends to A, routes GET responses back to B
    let mut adapter_b = MemorySync::new();
    adapter_b.add_peer("A", gun_a.clone());
    adapter_b.set_local(gun_b.clone());

    let sync_a = SyncManager::new(gun_a, adapter_a);
    let sync_b = SyncManager::new(gun_b, adapter_b);

    (sync_a, sync_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::GunOptions;
    use crate::types::GunValue;

    fn new_gun() -> Gun {
        Gun::new(GunOptions::default())
    }

    #[test]
    fn sync_pair_replicates_writes() {
        let gun_a = new_gun();
        let gun_b = new_gun();

        let (mut sa, _sb) = sync_pair(gun_a.clone(), gun_b.clone());

        gun_a.get("mark").put_kv("name", GunValue::Text("Mark".into()));
        sa.flush(); // send to B

        assert_eq!(
            gun_b.get("mark").get("name").val(),
            Some(GunValue::Text("Mark".into()))
        );
    }

    #[test]
    fn sync_pair_bidirectional() {
        let gun_a = new_gun();
        let gun_b = new_gun();

        let (mut sa, mut sb) = sync_pair(gun_a.clone(), gun_b.clone());

        gun_a.get("alice").put_kv("role", GunValue::Text("admin".into()));
        sa.flush();

        gun_b.get("bob").put_kv("role", GunValue::Text("user".into()));
        sb.flush();

        assert_eq!(
            gun_b.get("alice").get("role").val(),
            Some(GunValue::Text("admin".into()))
        );
        assert_eq!(
            gun_a.get("bob").get("role").val(),
            Some(GunValue::Text("user".into()))
        );
    }

    #[test]
    fn sync_multiple_keys() {
        let gun_a = new_gun();
        let gun_b = new_gun();

        let (mut sa, _sb) = sync_pair(gun_a.clone(), gun_b.clone());

        gun_a.get("user").put(vec![
            ("name".into(), GunValue::Text("Alice".into())),
            ("age".into(), GunValue::Number(30.0)),
            ("active".into(), GunValue::Bool(true)),
        ]);
        sa.flush();

        assert_eq!(
            gun_b.get("user").get("name").val(),
            Some(GunValue::Text("Alice".into()))
        );
        assert_eq!(
            gun_b.get("user").get("age").val(),
            Some(GunValue::Number(30.0))
        );
        assert_eq!(
            gun_b.get("user").get("active").val(),
            Some(GunValue::Bool(true))
        );
    }

    #[test]
    fn sync_conflict_resolution() {
        let gun_a = new_gun();
        let gun_b = new_gun();

        // Pre-seed B with old data using an explicit old state timestamp
        // Seed B with data at a deliberately old timestamp (1000.0)
        // Seed B by receiving a "peer" message with an old timestamp
        let mut old_node = Node::new("x");
        old_node.put("val", GunValue::Text("old".into()), 1000.0);
        let old_msg = wire::put_message("seed", &[&old_node]);
        gun_b.receive(&old_msg);

        let (mut sa, _sb) = sync_pair(gun_a.clone(), gun_b.clone());

        // A writes newer data (current clock >> 1000.0)
        gun_a.get("x").put_kv("val", GunValue::Text("new".into()));
        sa.flush();

        // B should accept A's newer value via HAM
        assert_eq!(
            gun_b.get("x").get("val").val(),
            Some(GunValue::Text("new".into()))
        );
    }

    #[test]
    fn sync_null_tombstone() {
        let gun_a = new_gun();
        let gun_b = new_gun();

        let (mut sa, _sb) = sync_pair(gun_a.clone(), gun_b.clone());

        gun_a.get("item").put_kv("status", GunValue::Text("active".into()));
        sa.flush();
        assert_eq!(
            gun_b.get("item").get("status").val(),
            Some(GunValue::Text("active".into()))
        );

        gun_a.get("item").put_kv("status", GunValue::Null);
        sa.flush();
        assert_eq!(
            gun_b.get("item").get("status").val(),
            Some(GunValue::Null)
        );
    }

    #[test]
    fn sync_links() {
        let gun_a = new_gun();
        let gun_b = new_gun();

        let (mut sa, _sb) = sync_pair(gun_a.clone(), gun_b.clone());

        gun_a.get("mark").put_kv("name", GunValue::Text("Mark".into()));
        gun_a.get("mark").put_kv("boss", GunValue::Link("fluffy".into()));
        gun_a.get("fluffy").put_kv("name", GunValue::Text("Fluffy".into()));
        sa.flush();

        let boss_link = gun_b.get("mark").get("boss").val();
        assert_eq!(boss_link, Some(GunValue::Link("fluffy".into())));

        let boss_name = gun_b.get("mark").get("boss").get("name").val();
        assert_eq!(boss_name, Some(GunValue::Text("Fluffy".into())));
    }

    #[test]
    fn sync_triggers_listeners_on_remote() {
        let gun_a = new_gun();
        let gun_b = new_gun();

        let (mut sa, _sb) = sync_pair(gun_a.clone(), gun_b.clone());

        let received = Arc::new(Mutex::new(Vec::new()));
        let r = received.clone();
        gun_b.get("chat").get("msg1").on(move |val, _key| {
            r.lock().unwrap().push(val);
        });

        gun_a.get("chat").put_kv("msg1", GunValue::Text("hello!".into()));
        sa.flush();

        let data = received.lock().unwrap();
        assert!(
            data.contains(&GunValue::Text("hello!".into())),
            "expected listener to fire, got {:?}",
            *data
        );
    }

    #[test]
    fn memory_sync_adapter() {
        let gun_b = new_gun();
        let mut adapter = MemorySync::new();
        adapter.add_peer("B", gun_b.clone());

        // Manually send a message
        let mut node = Node::new("test");
        node.put("x", GunValue::Number(42.0), 100.0);
        let msg = wire::put_message("test1", &[&node]);
        let json = wire::serialize_message(&msg).unwrap();

        adapter.broadcast(&json, None);

        assert_eq!(
            gun_b.get("test").get("x").val(),
            Some(GunValue::Number(42.0))
        );
    }

    #[test]
    fn get_request_fetches_remote_data() {
        let gun_a = new_gun();
        let gun_b = new_gun();

        // B has data
        gun_b.get("config").put_kv("theme", GunValue::Text("dark".into()));

        let (mut sa, _sb) = sync_pair(gun_a.clone(), gun_b.clone());

        // A doesn't have data yet
        assert!(gun_a.get("config").get("theme").val().is_none());

        // A sends GET request — B responds with PUT containing the data
        sa.request("config", Some("theme"));

        // After the request, A should have received the data via the GET response
        assert_eq!(
            gun_a.get("config").get("theme").val(),
            Some(GunValue::Text("dark".into()))
        );
    }

    #[test]
    fn get_request_whole_node() {
        let gun_a = new_gun();
        let gun_b = new_gun();

        gun_b.get("user").put(vec![
            ("name".into(), GunValue::Text("Alice".into())),
            ("role".into(), GunValue::Text("admin".into())),
        ]);

        let (mut sa, _sb) = sync_pair(gun_a.clone(), gun_b.clone());

        // Request whole node
        sa.request("user", None);

        assert_eq!(
            gun_a.get("user").get("name").val(),
            Some(GunValue::Text("Alice".into()))
        );
        assert_eq!(
            gun_a.get("user").get("role").val(),
            Some(GunValue::Text("admin".into()))
        );
    }
}
