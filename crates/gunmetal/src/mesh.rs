//! Mesh — DAM (Daisy-chain Ad-hoc Mesh) message routing.
//!
//! Rust equivalent of `gun/src/mesh.js`. All wire traffic — PUTs, GETs and
//! ACKs — flows through the mesh, which provides:
//!
//! - **Deduplication**: message-ID (`#`) and content-hash (`@`+`##`) dedup
//!   via [`Dup`], preventing infinite relay loops.
//! - **Peer exclusion**: the `><` field lists peers that already saw a
//!   message; relays skip them and *replace* the field with their own list.
//! - **ACK routing**: ACKs (`@`) are traced through the dedup table back to
//!   the original sender instead of broadcast.
//! - **Batching**: outgoing messages are buffered per peer and flushed as
//!   JSON arrays after a configurable `gap` window.
//! - **DAM protocol**: `dam`-tagged messages (`?` handshake, `!` error,
//!   `mob` rebalancing, `opt` introduction) are consumed locally and never
//!   relayed.
//!
//! # Transport integration
//!
//! Transports register peers with [`Mesh::hi`], deliver inbound frames to
//! [`Mesh::hear`] and remove peers with [`Mesh::bye`]. The mesh pushes
//! outbound frames through the `PeerSender` closure supplied at `hi` time.
//! Senders MUST NOT call back into the mesh synchronously.
//!
//! Heartbeats are empty arrays (`[]`), sent by the transport layer every
//! [`HEARTBEAT_INTERVAL`] and ignored by [`Mesh::hear`].

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use serde_json::Value;

use crate::concurrency::{lock_mut, new_shared_mut, SharedMut};
use crate::dup::{Dup, DupConfig};
use crate::events::Event;
use crate::instance::Gun;
use crate::state::now_ms;
use crate::uuid::random_message_id;
use crate::wire::{self, WireMessage};

/// Outbound send closure registered per peer.
///
/// `Arc`/`Rc` so frames can be pushed without holding the mesh lock.
#[cfg(not(target_arch = "wasm32"))]
pub type PeerSender = std::sync::Arc<dyn Fn(&str) + Send + Sync>;
#[cfg(target_arch = "wasm32")]
pub type PeerSender = std::rc::Rc<dyn Fn(&str)>;

/// A serialized frame shared across per-peer queues/batches without
/// copying the bytes per peer.
#[cfg(not(target_arch = "wasm32"))]
type SharedFrame = std::sync::Arc<str>;
#[cfg(target_arch = "wasm32")]
type SharedFrame = std::rc::Rc<str>;

/// The wire form of a heartbeat: an empty batch.
pub const HEARTBEAT_RAW: &str = "[]";

/// How often transports send heartbeats on WebSocket connections.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);

/// Maximum disconnect-write registrations kept per peer.
const MAX_BYE_WRITES_PER_PEER: usize = 100;

/// Maximum total serialized bytes of disconnect-write payloads retained
/// per peer. Bounds bye-write memory in bytes alongside the count cap:
/// 100 registrations of near-wire-cap graphs would otherwise hold ~1 GB.
const MAX_BYE_BYTES_PER_PEER: usize = 1_048_576;

/// Maximum distinct keys retained per subscribed soul. PUT routing only
/// consults souls (`contains_key`), so excess keys are dropped without
/// affecting delivery — this bounds the second dimension of the table.
const MAX_KEYS_PER_SOUL: usize = 100;

/// Maximum souls a single peer may hold AXE subscriptions for. Beyond
/// this, further GETs are still answered but no longer recorded (the
/// peer simply misses pushed updates for the extra souls).
const MAX_SUBSCRIPTIONS_PER_PEER: usize = 10_000;

/// Mesh configuration. Mirrors DAM's `opt.*` knobs.
#[derive(Debug, Clone)]
pub struct MeshConfig {
    /// Process ID exchanged in the `?` handshake. Random if not set.
    pub pid: String,
    /// Batching window. `0` (default) sends frames immediately.
    pub gap: Duration,
    /// Messages processed per tick before yielding in
    /// [`Mesh::hear_async`] (GUN's `opt.puff`). Default 9.
    pub puff: usize,
    /// Maximum inbound frame size in bytes (GUN's `opt.max`).
    pub max_message_bytes: usize,
    /// Max peer IDs listed in the outgoing `><` field. Default 7.
    pub seen_by_max_peers: usize,
    /// Max characters of the outgoing `><` field. Default 99.
    pub seen_by_max_chars: usize,
    /// Acknowledge stored PUTs with `{@, ok}`. Default true.
    pub ack_puts: bool,
    /// AXE subscription routing: when enabled (relay peers), PUT relays
    /// are sent only to peers with a matching subscription (recorded from
    /// their GETs) instead of broadcast. Default false (DAM broadcast).
    pub axe: bool,
    /// Dedup table tuning.
    pub dup: DupConfig,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            pid: random_message_id(9),
            gap: Duration::ZERO,
            puff: 9,
            max_message_bytes: wire::MAX_MESSAGE_BYTES,
            seen_by_max_peers: 7,
            seen_by_max_chars: 99,
            ack_puts: true,
            axe: false,
            dup: DupConfig::default(),
        }
    }
}

impl MeshConfig {
    /// Derive a mesh configuration from GUN constructor options
    /// (`pid`, `gap`, `axe`).
    pub fn from_options(options: &crate::instance::GunOptions) -> Self {
        Self {
            pid: options
                .pid
                .clone()
                .unwrap_or_else(|| random_message_id(9)),
            gap: options.gap,
            axe: options.axe,
            ..Default::default()
        }
    }
}

/// A registered peer.
struct MeshPeer {
    id: String,
    /// Process ID learned from the `?` handshake.
    pid: Option<String>,
    /// Peer URL, when dialed out. Outbound connections (`Some`) matter
    /// for duplicate-connection resolution.
    url: Option<String>,
    /// Outbound transport. `None` while the connection is opening.
    sender: Option<PeerSender>,
    /// Frames queued while the connection is opening.
    queue: Vec<SharedFrame>,
    /// Frames buffered inside the current `gap` window.
    batch: Vec<SharedFrame>,
    /// When the current batch was opened (ms since epoch).
    batch_started_ms: f64,
    /// Whether an async flush has been scheduled for the current batch.
    flush_scheduled: bool,
    /// AXE subscription table: soul → properties this peer asked for
    /// (empty string = the whole node).
    subscriptions: HashMap<String, HashSet<String>>,
}

impl MeshPeer {
    fn new(id: String, url: Option<String>, sender: Option<PeerSender>) -> Self {
        Self {
            id,
            pid: None,
            url,
            sender,
            queue: Vec::new(),
            batch: Vec::new(),
            batch_started_ms: 0.0,
            flush_scheduled: false,
            subscriptions: HashMap::new(),
        }
    }
}

struct MeshInner {
    config: MeshConfig,
    dup: Dup,
    peers: HashMap<String, MeshPeer>,
    /// Disconnect writes registered via `bye` wire messages, per peer.
    bye_writes: HashMap<String, Vec<Value>>,
    /// Cumulative serialized bytes of registered bye writes, per peer.
    bye_write_bytes: HashMap<String, usize>,
    /// Last `{dam:'!'}` error received.
    last_error: Option<String>,
    /// Last `{dam:'mob'}` redirect: alternative peer URLs to dial instead.
    mob_redirect: Option<Vec<String>>,
    /// Peer URLs suggested via `{dam:'opt'}` introductions.
    suggested_peers: Vec<String>,
}

/// A frame ready to push to a transport (collected under the mesh lock,
/// sent after releasing it).
struct OutFrame {
    sender: PeerSender,
    raw: SharedFrame,
}

/// The DAM mesh router. Cheaply cloneable; clones share state.
#[derive(Clone)]
pub struct Mesh {
    gun: Gun,
    inner: SharedMut<MeshInner>,
}

impl Mesh {
    /// Create a mesh routing messages in and out of `gun`.
    ///
    /// Local writes (e.g. `gun.get("a").put_kv(...)`) are broadcast to
    /// connected peers automatically — the GUN `put → out` flow. Events
    /// originating from the network carry the wire message ID and are NOT
    /// re-wrapped (the relay path in [`hear`](Self::hear) already forwards
    /// them with their original ID for loop-free dedup).
    pub fn new(gun: Gun, config: MeshConfig) -> Self {
        let dup = Dup::with_config(config.dup.clone());
        let mesh = Self {
            gun: gun.clone(),
            inner: new_shared_mut(MeshInner {
                config,
                dup,
                peers: HashMap::new(),
                bye_writes: HashMap::new(),
                bye_write_bytes: HashMap::new(),
                last_error: None,
                mob_redirect: None,
                suggested_peers: Vec::new(),
            }),
        };

        let mesh_for_writes = mesh.clone();
        lock_mut(&gun.events).on("put", move |event| {
            if event.msg_id.is_some() {
                return; // network/storage-originated — already routed
            }
            let (Some(value), Some(key)) = (&event.value, &event.key) else {
                return;
            };
            let mut node = crate::types::Node::new(&event.soul);
            node.put(key, value.clone(), event.state);
            let msg = wire::put_message(&random_message_id(9), &[&node]);
            mesh_for_writes.say(msg, None);
        });

        mesh
    }

    /// This mesh's process ID.
    pub fn pid(&self) -> String {
        lock_mut(&self.inner).config.pid.clone()
    }

    /// Number of currently connected peers (GUN's `mesh.near`).
    pub fn near(&self) -> usize {
        lock_mut(&self.inner)
            .peers
            .values()
            .filter(|p| p.sender.is_some())
            .count()
    }

    /// Connected peer IDs.
    pub fn connected_peer_ids(&self) -> Vec<String> {
        lock_mut(&self.inner)
            .peers
            .values()
            .filter(|p| p.sender.is_some())
            .map(|p| p.id.clone())
            .collect()
    }

    /// The `pid` a peer reported in its handshake, if any.
    pub fn peer_pid(&self, peer_id: &str) -> Option<String> {
        lock_mut(&self.inner)
            .peers
            .get(peer_id)
            .and_then(|p| p.pid.clone())
    }

    /// Last `{dam:'!'}` error received from any peer.
    pub fn last_error(&self) -> Option<String> {
        lock_mut(&self.inner).last_error.clone()
    }

    /// Take the most recent `{dam:'mob'}` redirect (alternative peer URLs).
    /// Transports should dial one of these after being shed by a full relay.
    pub fn take_mob_redirect(&self) -> Option<Vec<String>> {
        lock_mut(&self.inner).mob_redirect.take()
    }

    /// Drain peer URLs suggested via `{dam:'opt'}` introductions.
    pub fn take_suggested_peers(&self) -> Vec<String> {
        std::mem::take(&mut lock_mut(&self.inner).suggested_peers)
    }

    /// Whether a peer is currently registered (connected or connecting).
    pub fn is_peer(&self, peer_id: &str) -> bool {
        lock_mut(&self.inner).peers.contains_key(peer_id)
    }

    /// The souls a peer has subscribed to (via GETs), AXE-style.
    pub fn peer_subscriptions(&self, peer_id: &str) -> Vec<String> {
        lock_mut(&self.inner)
            .peers
            .get(peer_id)
            .map(|p| p.subscriptions.keys().cloned().collect())
            .unwrap_or_default()
    }

    // ── hi / bye ────────────────────────────────────────────────────

    /// Register (or reconnect) a peer and send the DAM `?` handshake.
    ///
    /// `sender` pushes serialized frames to the transport; pass `None`
    /// while the connection is still opening and call `hi` again with the
    /// sender once open (queued frames flush automatically).
    pub fn hi(&self, peer_id: &str, url: Option<String>, sender: Option<PeerSender>) {
        let mut frames = Vec::new();
        {
            let mut inner = lock_mut(&self.inner);
            let peer = inner
                .peers
                .entry(peer_id.to_string())
                .or_insert_with(|| MeshPeer::new(peer_id.to_string(), url.clone(), None));
            if sender.is_some() {
                peer.sender = sender;
            }

            if let Some(ref s) = peer.sender {
                // Flush anything queued while connecting.
                for raw in peer.queue.drain(..) {
                    frames.push(OutFrame {
                        sender: s.clone(),
                        raw,
                    });
                }
            }
        }
        self.push_frames(frames);

        // Handshake: exchange process IDs.
        let handshake = WireMessage {
            id: Some(random_message_id(9)),
            dam: Some("?".into()),
            pid: Some(self.pid()),
            ..Default::default()
        };
        self.say(handshake, Some(peer_id));

        lock_mut(&self.gun.events).emit("hi", &Event::node(peer_id));
    }

    /// Disconnect a peer: apply registered `bye` writes, drop state, emit
    /// the `bye` event.
    pub fn bye(&self, peer_id: &str) {
        let bye_graphs = {
            let mut inner = lock_mut(&self.inner);
            inner.peers.remove(peer_id);
            inner.bye_write_bytes.remove(peer_id);
            inner.bye_writes.remove(peer_id).unwrap_or_default()
        };

        // Apply disconnect writes (gun/lib/bye.js server side): each graph
        // is {soul: {key: value}}; values get fresh state timestamps.
        for graph in bye_graphs {
            self.apply_bye_graph(&graph);
        }

        lock_mut(&self.gun.events).emit("bye", &Event::node(peer_id));
    }

    fn apply_bye_graph(&self, graph: &Value) {
        let Some(souls) = graph.as_object() else {
            return;
        };
        for (soul, node) in souls {
            // Bye writes are registered by any peer without signatures, so
            // they must not reach user namespaces — a regular PUT to
            // `~pubKey/...` goes through signature verification
            // (instance::verify_user_node); a bye write would bypass it.
            if soul.starts_with('~') {
                continue;
            }
            let Some(entries) = node.as_object() else {
                continue;
            };
            for (key, value) in entries {
                if let Some(gun_value) = wire::json_to_value(value) {
                    self.gun.get(soul).put_kv(key, gun_value);
                }
            }
        }
    }

    // ── hear ────────────────────────────────────────────────────────

    /// Process an inbound frame: a single message (`{…}`), a batch
    /// (`[…]`), or a heartbeat (`[]`, ignored).
    pub fn hear(&self, raw: &str, from_peer: &str) {
        for msg in self.parse_frame(raw) {
            self.hear_one(msg, from_peer);
        }
    }

    /// Async variant of [`hear`](Self::hear) that yields to the runtime
    /// every `puff` messages while draining large batches.
    pub async fn hear_async(&self, raw: &str, from_peer: &str) {
        let puff = lock_mut(&self.inner).config.puff.max(1);
        for (i, msg) in self.parse_frame(raw).into_iter().enumerate() {
            if i > 0 && i % puff == 0 {
                crate::runtime::sleep_async(Duration::ZERO).await;
            }
            self.hear_one(msg, from_peer);
        }
    }

    /// Parse a raw frame into individual wire messages. Oversized frames,
    /// heartbeats and malformed JSON yield nothing.
    fn parse_frame(&self, raw: &str) -> Vec<WireMessage> {
        let max = lock_mut(&self.inner).config.max_message_bytes;
        if raw.len() > max {
            return Vec::new();
        }
        let trimmed = raw.trim_start();
        if trimmed.starts_with('[') {
            // Empty array == heartbeat == no items; parse errors yield none.
            // Elements decode individually: one malformed message must not
            // discard the legitimate messages batched alongside it.
            serde_json::from_str::<Vec<serde_json::Value>>(trimmed)
                .map(|elems| {
                    elems
                        .into_iter()
                        .filter_map(|e| serde_json::from_value::<WireMessage>(e).ok())
                        .collect()
                })
                .unwrap_or_default()
        } else if trimmed.starts_with('{') {
            match serde_json::from_str::<WireMessage>(trimmed) {
                Ok(msg) => vec![msg],
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        }
    }

    fn hear_one(&self, msg: WireMessage, from_peer: &str) {
        // DAM protocol messages are consumed even without a message ID.
        let Some(id) = msg.id.clone() else {
            if msg.dam.is_some() {
                self.handle_dam(&msg, from_peer);
            }
            return;
        };

        // Dedup by message ID, then by ack+content-hash combo. The hash
        // used for the combo is recomputed locally from the `put` payload
        // rather than trusting the `##` field: a peer lying about the
        // hash could otherwise suppress another peer's genuine answer
        // (dedup poisoning). With a local hash, two answers only dedup
        // when their payloads actually hash the same.
        {
            let mut inner = lock_mut(&self.inner);
            if inner.dup.check(&id) {
                return;
            }
            if msg.hash.is_some()
                && let (Some(ack), Some(put)) = (&msg.ack, &msg.put)
                && let Ok(json) = serde_json::to_string(put)
            {
                let combo = format!("{}{}", ack, gun_string_hash(&json));
                if inner.dup.check(&combo) {
                    inner.dup.track_from(id, Some(from_peer.to_string()));
                    return;
                }
                inner.dup.track(combo);
            }
            inner.dup.track_from(id.clone(), Some(from_peer.to_string()));
        }

        // DAM protocol dispatch — consumed, never relayed.
        if msg.dam.is_some() {
            self.handle_dam(&msg, from_peer);
            return;
        }

        // Peers that already saw this message (incoming `><`).
        let mut exclusions: HashSet<String> = msg
            .seen_by
            .as_deref()
            .map(parse_seen_by)
            .unwrap_or_default();
        exclusions.insert(from_peer.to_string());

        // Disconnect-write registration rides on data messages. Capped
        // per peer so a client cannot grow the registry without bound.
        if let Some(ref bye_graph) = msg.bye {
            // Byte-account with the serialized size; an unserializable
            // graph counts as oversized and is rejected.
            let graph_bytes = serde_json::to_string(bye_graph)
                .map(|s| s.len())
                .unwrap_or(usize::MAX);
            let mut inner = lock_mut(&self.inner);
            let used = inner
                .bye_write_bytes
                .get(from_peer)
                .copied()
                .unwrap_or_default();
            let writes = inner.bye_writes.entry(from_peer.to_string()).or_default();
            if writes.len() < MAX_BYE_WRITES_PER_PEER
                && graph_bytes.saturating_add(used) <= MAX_BYE_BYTES_PER_PEER
            {
                writes.push(bye_graph.clone());
                *inner.bye_write_bytes.entry(from_peer.to_string()).or_default() += graph_bytes;
            }
        }

        let is_put = msg.put.is_some();
        let is_get = msg.get.is_some();

        // AXE: a GET doubles as a subscription — remember what this peer
        // asked for so PUT updates can be routed instead of broadcast.
        if let Some(ref get) = msg.get {
            let mut inner = lock_mut(&self.inner);
            if let Some(peer) = inner.peers.get_mut(from_peer)
                && (peer.subscriptions.len() < MAX_SUBSCRIPTIONS_PER_PEER
                    || peer.subscriptions.contains_key(&get.soul))
            {
                let keys = peer.subscriptions.entry(get.soul.clone()).or_default();
                if keys.len() < MAX_KEYS_PER_SOUL {
                    keys.insert(get.key.clone().unwrap_or_default());
                }
            }
        }

        // Dispatch into the GUN core. GET requests yield a PUT response.
        let response = self.gun.receive(&msg);

        match response {
            Some(mut resp) => {
                resp.id = Some(random_message_id(9));
                // ACK routing in say() traces resp.ack (== id) back to
                // from_peer and sends directly.
                self.say(resp, None);
            }
            None if is_get => {
                // Not found locally: empty ACK (GUN's "not found").
                let nack = WireMessage {
                    id: Some(random_message_id(9)),
                    ack: Some(id.clone()),
                    ..Default::default()
                };
                self.say(nack, None);
            }
            None => {}
        }

        // Acknowledge stored PUTs (but never ack an ACK — a PUT carrying
        // `@` is a GET response, not a write request).
        let ack_puts = lock_mut(&self.inner).config.ack_puts;
        if is_put && msg.ack.is_none() && ack_puts {
            let mut ok = serde_json::Map::new();
            ok.insert("@".into(), Value::Number(serde_json::Number::from(1)));
            ok.insert(
                "/".into(),
                Value::Number(serde_json::Number::from(self.near() as u64)),
            );
            let ack = WireMessage {
                id: Some(random_message_id(9)),
                ack: Some(id.clone()),
                ok: Some(Value::Object(ok)),
                ..Default::default()
            };
            self.say(ack, None);
        }

        // Daisy-chain: relay data messages to the rest of the mesh.
        if is_put || is_get {
            self.say_with_exclusions(msg, None, &exclusions);
        }
    }

    fn handle_dam(&self, msg: &WireMessage, from_peer: &str) {
        match msg.dam.as_deref() {
            Some("?") => {
                let their_pid = msg.pid.clone();
                let mut self_connection = false;
                {
                    let mut inner = lock_mut(&self.inner);
                    if let Some(ref pid) = their_pid {
                        self_connection = *pid == inner.config.pid;
                        if let Some(peer) = inner.peers.get_mut(from_peer) {
                            peer.pid = their_pid.clone();
                        }
                    }
                }
                if self_connection {
                    // AXE: connected to ourselves — drop the connection.
                    self.bye(from_peer);
                    return;
                }
                // AXE UP: duplicate connections to the same process are
                // resolved deterministically — the link dialed by the
                // higher-pid side survives, so both ends drop the same one.
                if let Some(their_pid) = their_pid.as_deref() {
                    let drop_id = {
                        let inner = lock_mut(&self.inner);
                        let new_outbound = inner
                            .peers
                            .get(from_peer)
                            .is_some_and(|p| p.url.is_some());
                        inner
                            .peers
                            .values()
                            .find(|p| p.id != from_peer && p.pid.as_deref() == Some(their_pid))
                            .map(|existing| {
                                let existing_outbound = existing.url.is_some();
                                if existing_outbound == new_outbound {
                                    // Same direction: keep the older link.
                                    from_peer.to_string()
                                } else {
                                    let keep_our_outbound =
                                        inner.config.pid.as_str() > their_pid;
                                    let drop_outbound = !keep_our_outbound;
                                    if existing_outbound == drop_outbound {
                                        existing.id.clone()
                                    } else {
                                        from_peer.to_string()
                                    }
                                }
                            })
                    };
                    if let Some(drop_id) = drop_id {
                        self.bye(&drop_id);
                        if drop_id == from_peer {
                            return;
                        }
                    }
                }
                // Reply with our pid unless this is already a reply.
                if msg.ack.is_none() {
                    let reply = WireMessage {
                        id: Some(random_message_id(9)),
                        ack: msg.id.clone(),
                        dam: Some("?".into()),
                        pid: Some(self.pid()),
                        ..Default::default()
                    };
                    self.say(reply, Some(from_peer));
                }
            }
            Some("!") => {
                lock_mut(&self.inner).last_error = msg.err.clone();
            }
            Some("mob") => {
                // Relay is full: remember alternative peers so the
                // transport can reconnect elsewhere.
                let urls = msg
                    .peers
                    .as_ref()
                    .map(extract_peer_urls)
                    .unwrap_or_default();
                lock_mut(&self.inner).mob_redirect = Some(urls);
            }
            Some("opt") => {
                // Peer introduction: collect suggested URLs.
                if let Some(ref opt) = msg.opt {
                    let urls = opt
                        .get("peers")
                        .map(extract_peer_urls)
                        .unwrap_or_default();
                    lock_mut(&self.inner).suggested_peers.extend(urls);
                }
            }
            _ => {} // unknown protocol messages are ignored (and not relayed)
        }
    }

    // ── say ─────────────────────────────────────────────────────────

    /// Send a message to a specific peer, or route it (ACK tracing /
    /// broadcast) when `to_peer` is `None`.
    pub fn say(&self, msg: WireMessage, to_peer: Option<&str>) {
        self.say_with_exclusions(msg, to_peer, &HashSet::new());
    }

    fn say_with_exclusions(
        &self,
        mut msg: WireMessage,
        to_peer: Option<&str>,
        exclusions: &HashSet<String>,
    ) {
        let frames = {
            let mut inner = lock_mut(&self.inner);

            // Every message gets an ID.
            let id = match msg.id.clone() {
                Some(id) => id,
                None => {
                    let id = random_message_id(9);
                    msg.id = Some(id.clone());
                    id
                }
            };

            // Content hash for ACKs carrying data (enables `@`+`##` dedup
            // downstream).
            if msg.put.is_some() && msg.ack.is_some() && msg.hash.is_none()
                && let Some(ref put) = msg.put
                    && let Ok(json) = serde_json::to_string(put) {
                        msg.hash = Some(Value::Number(serde_json::Number::from(
                            gun_string_hash(&json),
                        )));
                    }

            // Track our own sends so echoes are dropped on hear.
            inner.dup.track(id.clone());

            // Routing.
            let origin = inner.dup.via(&id).map(|s| s.to_string());
            let targets: Vec<String> = if let Some(peer) = to_peer {
                vec![peer.to_string()]
            } else if let Some(target) = msg
                .ack
                .as_deref()
                .and_then(|ack| inner.dup.via(ack))
                .map(|s| s.to_string())
            {
                // ACK routing: directly back toward the original sender.
                vec![target]
            } else {
                // Broadcast: replace `><` with our connected peers
                // (up to 7 IDs / 99 chars) before serializing.
                let connected: Vec<&str> = inner
                    .peers
                    .values()
                    .filter(|p| p.sender.is_some())
                    .map(|p| p.id.as_str())
                    .collect();
                msg.seen_by = build_seen_by(
                    &connected,
                    inner.config.seen_by_max_peers,
                    inner.config.seen_by_max_chars,
                );

                // AXE: PUT updates go only to peers subscribed to one of
                // the souls being written. GETs and everything else stay
                // broadcast (the subscription-miss fallback).
                let put_souls: Option<Vec<String>> = if inner.config.axe && msg.ack.is_none() {
                    msg.put.as_ref().and_then(|put| {
                        put.as_object()
                            .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
                    })
                } else {
                    None
                };

                inner
                    .peers
                    .values()
                    .filter(|p| match &put_souls {
                        Some(souls) => souls.iter().any(|s| p.subscriptions.contains_key(s)),
                        None => true,
                    })
                    .map(|p| p.id.clone())
                    .filter(|pid| {
                        // Self-send prevention: never send a message back to
                        // the peer it came from, nor to peers that already
                        // saw it.
                        Some(pid.as_str()) != origin.as_deref() && !exclusions.contains(pid)
                    })
                    .collect()
            };

            let raw: SharedFrame = match wire::serialize_message(&msg) {
                Ok(json) => SharedFrame::from(json),
                Err(_) => return,
            };

            let mut frames = Vec::new();
            let gap = inner.config.gap;
            let now = now_ms();
            for target in targets {
                let Some(peer) = inner.peers.get_mut(&target) else {
                    continue;
                };
                let Some(sender) = peer.sender.clone() else {
                    // Connection still opening: queue for the next hi().
                    peer.queue.push(raw.clone());
                    continue;
                };
                if gap.is_zero() {
                    frames.push(OutFrame {
                        sender,
                        raw: raw.clone(),
                    });
                } else {
                    if peer.batch.is_empty() {
                        peer.batch_started_ms = now;
                    }
                    peer.batch.push(raw.clone());
                    // flush_scheduled is reset by drain_batch(); if the
                    // deadline fallback below drains first, the already-
                    // scheduled timer becomes a harmless no-op flush and a
                    // new timer is scheduled on the next batched say().
                    if !peer.flush_scheduled {
                        peer.flush_scheduled = self.schedule_flush(peer.id.clone(), gap);
                    }
                    // Deadline fallback when no async runtime is available.
                    if now - peer.batch_started_ms >= gap.as_millis() as f64
                        && let Some(frame) = drain_batch(peer) {
                            frames.push(frame);
                        }
                }
            }
            frames
        };

        self.push_frames(frames);
    }

    // ── batching ────────────────────────────────────────────────────

    /// Flush all pending per-peer batches immediately.
    pub fn flush(&self) {
        let frames = {
            let mut inner = lock_mut(&self.inner);
            let mut frames = Vec::new();
            for peer in inner.peers.values_mut() {
                if let Some(frame) = drain_batch(peer) {
                    frames.push(frame);
                }
            }
            frames
        };
        self.push_frames(frames);
    }

    /// Flush one peer's pending batch.
    pub fn flush_peer(&self, peer_id: &str) {
        let frame = {
            let mut inner = lock_mut(&self.inner);
            inner.peers.get_mut(peer_id).and_then(drain_batch)
        };
        if let Some(frame) = frame {
            (frame.sender)(&frame.raw);
        }
    }

    /// Schedule an async flush after the gap window. Returns whether a
    /// timer was actually scheduled (requires a tokio runtime on native).
    #[cfg(not(target_arch = "wasm32"))]
    fn schedule_flush(&self, peer_id: String, gap: Duration) -> bool {
        if tokio::runtime::Handle::try_current().is_err() {
            return false;
        }
        let mesh = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(gap).await;
            mesh.flush_peer(&peer_id);
        });
        true
    }

    /// Schedule an async flush after the gap window (WASM: setTimeout).
    #[cfg(target_arch = "wasm32")]
    fn schedule_flush(&self, peer_id: String, gap: Duration) -> bool {
        let mesh = self.clone();
        crate::runtime::spawn_async(async move {
            crate::runtime::sleep_async(gap).await;
            mesh.flush_peer(&peer_id);
        });
        true
    }

    // ── heartbeat ───────────────────────────────────────────────────

    /// Send a heartbeat (`[]`) to every connected peer. Transports call
    /// this every [`HEARTBEAT_INTERVAL`].
    pub fn heartbeat_tick(&self) {
        let frames: Vec<OutFrame> = {
            let inner = lock_mut(&self.inner);
            inner
                .peers
                .values()
                .filter_map(|p| {
                    p.sender.as_ref().map(|s| OutFrame {
                        sender: s.clone(),
                        raw: SharedFrame::from(HEARTBEAT_RAW),
                    })
                })
                .collect()
        };
        self.push_frames(frames);
    }

    /// Spawn a background task sending heartbeats every
    /// [`HEARTBEAT_INTERVAL`]. Requires a tokio runtime; the task runs
    /// until the mesh is dropped elsewhere (it holds a clone).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_heartbeat(&self) {
        let mesh = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(HEARTBEAT_INTERVAL).await;
                mesh.heartbeat_tick();
            }
        });
    }

    // ── plumbing ────────────────────────────────────────────────────

    /// Invoke transport senders outside the mesh lock.
    fn push_frames(&self, frames: Vec<OutFrame>) {
        for frame in frames {
            (frame.sender)(&frame.raw);
        }
    }

    /// Access the underlying Gun instance.
    pub fn gun(&self) -> &Gun {
        &self.gun
    }
}

/// Drain a peer's batch into a single frame: one message is sent bare,
/// several are sent as a JSON array.
fn drain_batch(peer: &mut MeshPeer) -> Option<OutFrame> {
    peer.flush_scheduled = false;
    if peer.batch.is_empty() {
        return None;
    }
    let sender = peer.sender.clone()?;
    let raw: SharedFrame = if peer.batch.len() == 1 {
        peer.batch.drain(..).next()?
    } else {
        let joined = peer
            .batch
            .drain(..)
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(",");
        SharedFrame::from(format!("[{}]", joined))
    };
    Some(OutFrame { sender, raw })
}

/// Extract peer URLs from a DAM payload: either a single URL string
/// (`opt: {peers: "url"}`), a comma-separated list, or an object keyed by
/// URL (`peers: {"https://…/gun": 1}`).
fn extract_peer_urls(value: &Value) -> Vec<String> {
    match value {
        Value::String(s) => s
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        Value::Object(map) => map.keys().cloned().collect(),
        Value::Array(items) => items
            .iter()
            .filter_map(|v| v.as_str())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

/// Parse the `><` field into a set of peer IDs.
fn parse_seen_by(raw: &str) -> HashSet<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Build the outgoing `><` field: up to `max_peers` IDs, truncated to
/// `max_chars` (GUN limits: 7 peers / 99 chars).
fn build_seen_by(peer_ids: &[&str], max_peers: usize, max_chars: usize) -> Option<String> {
    if peer_ids.is_empty() {
        return None;
    }
    let mut out = String::new();
    for id in peer_ids.iter().take(max_peers) {
        let extra = if out.is_empty() { id.len() } else { id.len() + 1 };
        if out.len() + extra > max_chars {
            break;
        }
        if !out.is_empty() {
            out.push(',');
        }
        out.push_str(id);
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// GUN's `String.hash` (Java-style 31x rolling hash over UTF-16 code
/// units, iterated from the end, with int32 wrapping).
pub fn gun_string_hash(s: &str) -> i32 {
    let mut c: i32 = 0;
    for unit in s.encode_utf16().collect::<Vec<u16>>().into_iter().rev() {
        c = c.wrapping_shl(5).wrapping_sub(c).wrapping_add(unit as i32);
    }
    c
}

// ── AsyncSyncAdapter ────────────────────────────────────────────────

/// `Mesh` is a drop-in transport adapter: `send` routes to one peer,
/// `broadcast` routes through DAM (ACK tracing, exclusions, batching).
#[cfg(not(target_arch = "wasm32"))]
impl crate::transport::AsyncSyncAdapter for Mesh {
    fn send(
        &self,
        peer_id: &str,
        msg: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        let result = match wire::parse_message(msg) {
            Ok(parsed) => {
                self.say(parsed, Some(peer_id));
                Ok(())
            }
            Err(e) => Err(format!("invalid wire message: {}", e)),
        };
        Box::pin(async move { result })
    }

    fn broadcast(
        &self,
        msg: &str,
        exclude: Option<&str>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        let result = match wire::parse_message(msg) {
            Ok(parsed) => {
                let mut exclusions = HashSet::new();
                if let Some(peer) = exclude {
                    exclusions.insert(peer.to_string());
                }
                self.say_with_exclusions(parsed, None, &exclusions);
                Ok(())
            }
            Err(e) => Err(format!("invalid wire message: {}", e)),
        };
        Box::pin(async move { result })
    }

    fn connected_peers(&self) -> Vec<String> {
        self.connected_peer_ids()
    }

    fn is_connected(&self, peer_id: &str) -> bool {
        lock_mut(&self.inner)
            .peers
            .get(peer_id)
            .is_some_and(|p| p.sender.is_some())
    }
}

// ── tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::GunOptions;
    use crate::types::{GunValue, Node};
    use std::sync::{Arc, Mutex};

    type Outbox = Arc<Mutex<Vec<String>>>;

    fn new_mesh() -> Mesh {
        Mesh::new(Gun::new(GunOptions::default()), MeshConfig::default())
    }

    fn attach_peer(mesh: &Mesh, id: &str) -> Outbox {
        let outbox: Outbox = Arc::new(Mutex::new(Vec::new()));
        let o = outbox.clone();
        mesh.hi(
            id,
            None,
            Some(std::sync::Arc::new(move |raw: &str| {
                o.lock().unwrap().push(raw.to_string());
            })),
        );
        outbox
    }

    fn frames(outbox: &Outbox) -> Vec<String> {
        outbox.lock().unwrap().clone()
    }

    fn put_raw(id: &str, soul: &str, key: &str, value: GunValue, state: f64) -> String {
        let mut node = Node::new(soul);
        node.put(key, value, state);
        wire::serialize_message(&wire::put_message(id, &[&node])).unwrap()
    }

    #[test]
    fn hi_sends_handshake() {
        let mesh = new_mesh();
        let outbox = attach_peer(&mesh, "peerA");

        let sent = frames(&outbox);
        assert_eq!(sent.len(), 1);
        let msg = wire::parse_message(&sent[0]).unwrap();
        assert_eq!(msg.dam.as_deref(), Some("?"));
        assert_eq!(msg.pid.as_deref(), Some(mesh.pid().as_str()));
    }

    #[test]
    fn handshake_records_pid_and_replies() {
        let mesh = new_mesh();
        let outbox = attach_peer(&mesh, "peerA");
        outbox.lock().unwrap().clear();

        mesh.hear(r###"{"#":"h1","dam":"?","pid":"their-pid"}"###, "peerA");

        assert_eq!(mesh.peer_pid("peerA").as_deref(), Some("their-pid"));
        let sent = frames(&outbox);
        assert_eq!(sent.len(), 1, "handshake reply sent");
        let reply = wire::parse_message(&sent[0]).unwrap();
        assert_eq!(reply.dam.as_deref(), Some("?"));
        assert_eq!(reply.ack.as_deref(), Some("h1"));
    }

    #[test]
    fn handshake_reply_is_not_re_replied() {
        let mesh = new_mesh();
        let outbox = attach_peer(&mesh, "peerA");
        outbox.lock().unwrap().clear();

        // A reply (has @) must not trigger another reply — no ping-pong.
        mesh.hear(r###"{"#":"h2","@":"h1","dam":"?","pid":"their-pid"}"###, "peerA");
        assert!(frames(&outbox).is_empty());
    }

    #[test]
    fn self_connection_dropped_via_pid() {
        let mesh = new_mesh();
        attach_peer(&mesh, "loopback");
        assert_eq!(mesh.near(), 1);

        let own_pid = mesh.pid();
        mesh.hear(
            &format!(r###"{{"#":"h3","dam":"?","pid":"{}"}}"###, own_pid),
            "loopback",
        );
        assert_eq!(mesh.near(), 0, "self-connection must be dropped");
    }

    #[test]
    fn put_message_merges_into_graph() {
        let mesh = new_mesh();
        attach_peer(&mesh, "peerA");

        mesh.hear(
            &put_raw("m1", "mark", "name", GunValue::Text("Mark".into()), 100.0),
            "peerA",
        );
        assert_eq!(
            mesh.gun().get("mark").get("name").val(),
            Some(GunValue::Text("Mark".into()))
        );
    }

    #[test]
    fn duplicate_message_id_dropped() {
        let mesh = new_mesh();
        attach_peer(&mesh, "peerA");
        let b_out = attach_peer(&mesh, "peerB");
        b_out.lock().unwrap().clear();

        let raw = put_raw("dup1", "x", "v", GunValue::Number(1.0), 100.0);
        mesh.hear(&raw, "peerA");
        let after_first = frames(&b_out).len();
        mesh.hear(&raw, "peerA");
        assert_eq!(
            frames(&b_out).len(),
            after_first,
            "duplicate not re-relayed"
        );
    }

    #[test]
    fn hash_dedup_drops_redundant_acks() {
        let mesh = new_mesh();
        attach_peer(&mesh, "peerA");
        attach_peer(&mesh, "peerB");

        // Two peers answer the same GET (`@`: q1) with IDENTICAL payloads.
        // The dedup combo is built from a locally recomputed content hash,
        // so the second redundant answer is dropped before processing.
        let payload = r###""put":{"s":{"_":{"#":"s",">":{"k":100}},"k":"answer"}}"###;
        mesh.hear(
            &format!(r###"{{"#":"a1","@":"q1","##":1,{}}}"###, payload),
            "peerA",
        );
        // Track relay traffic to prove the duplicate is short-circuited.
        let c_out = attach_peer(&mesh, "peerC");
        c_out.lock().unwrap().clear();
        mesh.hear(
            &format!(r###"{{"#":"a2","@":"q1","##":1,{}}}"###, payload),
            "peerB",
        );
        assert!(
            !frames(&c_out).iter().any(|f| f.contains(r###""#":"a2""###)),
            "redundant identical answer is dropped"
        );
        assert_eq!(
            mesh.gun().get("s").get("k").val(),
            Some(GunValue::Text("answer".into()))
        );
    }

    #[test]
    fn hash_dedup_cannot_be_poisoned_by_fake_hashes() {
        let mesh = new_mesh();
        attach_peer(&mesh, "attacker");
        attach_peer(&mesh, "honest");

        // The attacker claims the same `##` as the genuine answer but with
        // garbage data. Because the dedup hash is recomputed locally from
        // the payload, the genuine answer (different payload, later state)
        // still lands.
        mesh.hear(
            r###"{"#":"evil1","@":"q9","##":777,"put":{"t":{"_":{"#":"t",">":{"k":100}},"k":"garbage"}}}"###,
            "attacker",
        );
        mesh.hear(
            r###"{"#":"real1","@":"q9","##":777,"put":{"t":{"_":{"#":"t",">":{"k":200}},"k":"genuine"}}}"###,
            "honest",
        );

        assert_eq!(
            mesh.gun().get("t").get("k").val(),
            Some(GunValue::Text("genuine".into())),
            "a forged ## must not suppress the genuine answer"
        );
    }

    #[test]
    fn heartbeat_ignored() {
        let mesh = new_mesh();
        let outbox = attach_peer(&mesh, "peerA");
        outbox.lock().unwrap().clear();

        mesh.hear(HEARTBEAT_RAW, "peerA");
        assert!(frames(&outbox).is_empty(), "heartbeat produces no traffic");
    }

    #[test]
    fn heartbeat_tick_sends_empty_arrays() {
        let mesh = new_mesh();
        let outbox = attach_peer(&mesh, "peerA");
        outbox.lock().unwrap().clear();

        mesh.heartbeat_tick();
        assert_eq!(frames(&outbox), vec![HEARTBEAT_RAW.to_string()]);
    }

    #[test]
    fn batch_array_processed_individually() {
        let mesh = new_mesh();
        attach_peer(&mesh, "peerA");

        let m1 = put_raw("b1", "a", "x", GunValue::Number(1.0), 100.0);
        let m2 = put_raw("b2", "b", "y", GunValue::Number(2.0), 100.0);
        mesh.hear(&format!("[{},{}]", m1, m2), "peerA");

        assert_eq!(
            mesh.gun().get("a").get("x").val(),
            Some(GunValue::Number(1.0))
        );
        assert_eq!(
            mesh.gun().get("b").get("y").val(),
            Some(GunValue::Number(2.0))
        );
    }

    #[test]
    fn relay_excludes_sender_and_seen_peers() {
        let mesh = new_mesh();
        let a_out = attach_peer(&mesh, "peerA");
        let b_out = attach_peer(&mesh, "peerB");
        let c_out = attach_peer(&mesh, "peerC");
        a_out.lock().unwrap().clear();
        b_out.lock().unwrap().clear();
        c_out.lock().unwrap().clear();

        // peerB is listed in >< — already saw the message.
        let mut node = Node::new("n");
        node.put("k", GunValue::Number(1.0), 100.0);
        let mut msg = wire::put_message("r1", &[&node]);
        msg.seen_by = Some("peerB".into());
        let raw = wire::serialize_message(&msg).unwrap();

        mesh.hear(&raw, "peerA");

        // peerA sent it (gets the PUT ack but not the relayed PUT),
        // peerB already saw it, peerC gets the relay.
        let relayed_to_c: Vec<_> = frames(&c_out)
            .iter()
            .filter(|f| f.contains(r###""put""###))
            .cloned()
            .collect();
        assert_eq!(relayed_to_c.len(), 1, "peerC receives the relay");

        assert!(
            !frames(&b_out).iter().any(|f| f.contains(r###""put""###)),
            "peerB already saw the message (><)"
        );
        assert!(
            !frames(&a_out)
                .iter()
                .any(|f| f.contains(r###""put""###) && f.contains(r###""#":"r1""###)),
            "message never echoes back to its sender"
        );
    }

    #[test]
    fn relay_replaces_seen_by_field() {
        let mesh = new_mesh();
        attach_peer(&mesh, "peerA");
        let c_out = attach_peer(&mesh, "peerC");
        c_out.lock().unwrap().clear();

        let mut node = Node::new("n2");
        node.put("k", GunValue::Number(1.0), 100.0);
        let mut msg = wire::put_message("r2", &[&node]);
        msg.seen_by = Some("upstream1,upstream2".into());
        mesh.hear(&wire::serialize_message(&msg).unwrap(), "peerA");

        let relayed = frames(&c_out)
            .into_iter()
            .find(|f| f.contains(r###""put""###))
            .expect("relay to peerC");
        let parsed = wire::parse_message(&relayed).unwrap();
        let seen = parsed.seen_by.expect("seen_by populated");
        assert!(
            !seen.contains("upstream1"),
            "relay must REPLACE ><, got {}",
            seen
        );
        assert!(seen.contains("peerA") || seen.contains("peerC"));
    }

    #[test]
    fn get_answered_from_graph_routed_to_requester() {
        let mesh = new_mesh();
        let a_out = attach_peer(&mesh, "peerA");
        let b_out = attach_peer(&mesh, "peerB");

        mesh.gun()
            .get("config")
            .put_kv("theme", GunValue::Text("dark".into()));
        a_out.lock().unwrap().clear();
        b_out.lock().unwrap().clear();

        mesh.hear(r###"{"#":"q9","get":{"#":"config"}}"###, "peerA");

        let answers: Vec<_> = frames(&a_out)
            .into_iter()
            .filter(|f| f.contains(r###""@":"q9""###) && f.contains("dark"))
            .collect();
        assert_eq!(answers.len(), 1, "answer goes to the requester");
        assert!(
            !frames(&b_out)
                .iter()
                .any(|f| f.contains(r###""@":"q9""###) && f.contains("dark")),
            "answer is not broadcast"
        );
    }

    #[test]
    fn get_not_found_sends_empty_ack() {
        let mesh = new_mesh();
        let a_out = attach_peer(&mesh, "peerA");
        a_out.lock().unwrap().clear();

        mesh.hear(r###"{"#":"q404","get":{"#":"missing"}}"###, "peerA");

        let nacks: Vec<_> = frames(&a_out)
            .into_iter()
            .filter(|f| f.contains(r###""@":"q404""###) && !f.contains(r###""put""###))
            .collect();
        assert_eq!(nacks.len(), 1, "not-found ack sent");
    }

    #[test]
    fn put_acknowledged() {
        let mesh = new_mesh();
        let a_out = attach_peer(&mesh, "peerA");
        a_out.lock().unwrap().clear();

        mesh.hear(
            &put_raw("p1", "ack-test", "k", GunValue::Number(1.0), 100.0),
            "peerA",
        );

        let acks: Vec<_> = frames(&a_out)
            .into_iter()
            .filter(|f| f.contains(r###""@":"p1""###) && f.contains(r###""ok""###))
            .collect();
        assert_eq!(acks.len(), 1, "PUT acked with ok metadata");
    }

    #[test]
    fn outgoing_broadcast_populates_seen_by_limits() {
        let mut ids: Vec<String> = Vec::new();
        for i in 0..10 {
            ids.push(format!("peer-with-a-rather-long-id-{}", i));
        }
        let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();

        let seen = build_seen_by(&id_refs, 7, 99).unwrap();
        assert!(seen.len() <= 99, "≤ 99 chars, got {}", seen.len());
        assert!(seen.split(',').count() <= 7, "≤ 7 peers");
    }

    #[test]
    fn dam_messages_not_relayed() {
        let mesh = new_mesh();
        attach_peer(&mesh, "peerA");
        let b_out = attach_peer(&mesh, "peerB");
        b_out.lock().unwrap().clear();

        mesh.hear(r###"{"#":"d1","dam":"?","pid":"x"}"###, "peerA");
        mesh.hear(r###"{"#":"d2","dam":"!","err":"boom"}"###, "peerA");

        assert!(
            !frames(&b_out)
                .iter()
                .any(|f| f.contains(r###""dam""###)),
            "protocol messages must not be relayed"
        );
        assert_eq!(mesh.last_error().as_deref(), Some("boom"));
    }

    #[test]
    fn mob_redirect_stored() {
        let mesh = new_mesh();
        attach_peer(&mesh, "relay");

        mesh.hear(
            r###"{"#":"m1","dam":"mob","mob":5000,"peers":{"https://other.example/gun":1}}"###,
            "relay",
        );
        let urls = mesh.take_mob_redirect().expect("mob stored");
        assert_eq!(urls, vec!["https://other.example/gun".to_string()]);
        assert!(mesh.take_mob_redirect().is_none(), "taken once");
    }

    #[test]
    fn opt_introduction_collected() {
        let mesh = new_mesh();
        attach_peer(&mesh, "relay");

        mesh.hear(
            r###"{"#":"o1","dam":"opt","opt":{"peers":"https://up.example/gun"}}"###,
            "relay",
        );
        assert_eq!(
            mesh.take_suggested_peers(),
            vec!["https://up.example/gun".to_string()]
        );
    }

    #[test]
    fn messages_queued_until_connected() {
        let mesh = new_mesh();
        // Register without a sender (connection opening).
        mesh.hi("slow", None, None);

        let mut node = Node::new("q");
        node.put("k", GunValue::Number(7.0), 100.0);
        mesh.say(wire::put_message("qm1", &[&node]), Some("slow"));

        // Now the connection opens.
        let outbox: Outbox = Arc::new(Mutex::new(Vec::new()));
        let o = outbox.clone();
        mesh.hi(
            "slow",
            None,
            Some(std::sync::Arc::new(move |raw: &str| {
                o.lock().unwrap().push(raw.to_string());
            })),
        );

        assert!(
            frames(&outbox).iter().any(|f| f.contains(r###""#":"qm1""###)),
            "queued message flushed on connect"
        );
    }

    #[test]
    fn gap_batches_into_array() {
        let config = MeshConfig {
            gap: Duration::from_secs(3600), // flush only explicitly
            ..Default::default()
        };
        let mesh = Mesh::new(Gun::new(GunOptions::default()), config);
        let outbox = attach_peer(&mesh, "peerA");
        mesh.flush(); // deliver the handshake so only test traffic batches
        outbox.lock().unwrap().clear();

        let mut n1 = Node::new("g1");
        n1.put("k", GunValue::Number(1.0), 100.0);
        let mut n2 = Node::new("g2");
        n2.put("k", GunValue::Number(2.0), 100.0);
        mesh.say(wire::put_message("gm1", &[&n1]), Some("peerA"));
        mesh.say(wire::put_message("gm2", &[&n2]), Some("peerA"));

        assert!(frames(&outbox).is_empty(), "held in gap window");
        mesh.flush();

        let sent = frames(&outbox);
        assert_eq!(sent.len(), 1, "single combined frame");
        assert!(sent[0].starts_with('['), "batch is a JSON array");
        let items: Vec<Value> = serde_json::from_str(&sent[0]).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn timed_gap_window_flushes_without_explicit_flush() {
        // §2.3: messages queued within the gap window are sent as one
        // array by the TIMER, not only by an explicit flush() call.
        let config = MeshConfig {
            gap: Duration::from_millis(20),
            ..Default::default()
        };
        let mesh = Mesh::new(Gun::new(GunOptions::default()), config);
        let outbox = attach_peer(&mesh, "peerA");
        mesh.flush(); // deliver the handshake so only test traffic batches
        outbox.lock().unwrap().clear();

        let mut n1 = Node::new("tg1");
        n1.put("k", GunValue::Number(1.0), 100.0);
        let mut n2 = Node::new("tg2");
        n2.put("k", GunValue::Number(2.0), 100.0);
        mesh.say(wire::put_message("tgm1", &[&n1]), Some("peerA"));
        mesh.say(wire::put_message("tgm2", &[&n2]), Some("peerA"));
        assert!(frames(&outbox).is_empty(), "held in gap window");

        tokio::time::sleep(Duration::from_millis(200)).await;

        let sent = frames(&outbox);
        assert_eq!(sent.len(), 1, "timer flushed one combined frame");
        let items: Vec<Value> = serde_json::from_str(&sent[0]).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn multi_hop_ack_routed_to_requester_via_dup_trace() {
        // §2.5: an answer arriving from a third peer is traced through
        // the dup table back to the original requester only — never
        // broadcast to bystanders.
        let mesh = Mesh::new(Gun::new(GunOptions::default()), MeshConfig::default());
        let requester = attach_peer(&mesh, "requester");
        let answerer = attach_peer(&mesh, "answerer");
        let bystander = attach_peer(&mesh, "bystander");
        for o in [&requester, &answerer, &bystander] {
            o.lock().unwrap().clear();
        }

        // Requester asks for a soul this mesh has no data for: the GET is
        // relayed to the other peers.
        mesh.hear(r###"{"#":"q1","get":{"#":"mh-soul"}}"###, "requester");
        assert!(
            frames(&answerer).iter().any(|f| f.contains("mh-soul")),
            "GET relayed to answerer"
        );
        requester.lock().unwrap().clear();
        bystander.lock().unwrap().clear();

        // Answerer responds with an ACK-carrying PUT referencing q1.
        mesh.hear(
            r###"{"#":"ans1","@":"q1","put":{"mh-soul":{"_":{"#":"mh-soul",">":{"k":100}},"k":42}}}"###,
            "answerer",
        );

        assert!(
            frames(&requester).iter().any(|f| f.contains("ans1")),
            "answer routed back to the requester"
        );
        assert!(
            !frames(&bystander).iter().any(|f| f.contains("ans1")),
            "answer not broadcast to bystanders"
        );
    }

    #[test]
    fn duplicate_same_direction_connection_keeps_older_link() {
        // §5.4: two links in the SAME direction to one process — the
        // older link survives, the newly-handshaking one is dropped.
        let mesh = Mesh::new(Gun::new(GunOptions::default()), MeshConfig::default());
        attach_peer(&mesh, "inbound-old");
        attach_peer(&mesh, "inbound-new");
        assert_eq!(mesh.near(), 2);

        mesh.hear(r###"{"#":"sd1","dam":"?","pid":"AAA"}"###, "inbound-old");
        mesh.hear(r###"{"#":"sd2","dam":"?","pid":"AAA"}"###, "inbound-new");

        assert_eq!(mesh.near(), 1, "duplicate link dropped");
        assert!(mesh.is_peer("inbound-old"), "older link kept");
        assert!(!mesh.is_peer("inbound-new"));
    }

    #[test]
    fn duplicate_connection_their_pid_higher_drops_our_outbound() {
        // §5.4: the link dialed by the higher-pid side survives. Their
        // pid is higher, so the link THEY dialed (our inbound) is kept
        // and our outbound dial is dropped.
        let config = MeshConfig {
            pid: "AAA".into(), // our pid is lower than theirs
            ..Default::default()
        };
        let mesh = Mesh::new(Gun::new(GunOptions::default()), config);
        let outbox: Outbox = Arc::new(Mutex::new(Vec::new()));
        let o = outbox.clone();
        mesh.hi(
            "outbound-link",
            Some("ws://them.example/gun".into()),
            Some(std::sync::Arc::new(move |raw: &str| {
                o.lock().unwrap().push(raw.to_string());
            })),
        );
        attach_peer(&mesh, "inbound-link");
        assert_eq!(mesh.near(), 2);

        mesh.hear(r###"{"#":"th1","dam":"?","pid":"ZZZ"}"###, "outbound-link");
        mesh.hear(r###"{"#":"th2","dam":"?","pid":"ZZZ"}"###, "inbound-link");

        assert_eq!(mesh.near(), 1, "duplicate link dropped");
        assert!(
            mesh.is_peer("inbound-link"),
            "their dial kept (their pid is higher)"
        );
        assert!(!mesh.is_peer("outbound-link"));
    }

    #[test]
    fn malformed_batch_element_does_not_drop_siblings() {
        // One type-invalid element in a batch must not discard the
        // legitimate messages batched alongside it.
        let mesh = Mesh::new(Gun::new(GunOptions::default()), MeshConfig::default());
        mesh.hear(
            r###"[{"get":{"bad":1}},{"#":"ok1","put":{"sib-soul":{"_":{"#":"sib-soul",">":{"k":100}},"k":7}}}]"###,
            "peerA",
        );
        assert_eq!(
            mesh.gun().get("sib-soul").get("k").val(),
            Some(GunValue::Number(7.0)),
            "valid sibling message landed"
        );
    }

    #[test]
    fn axe_subscription_keys_capped_per_soul() {
        // The subscription table is bounded in BOTH dimensions: souls per
        // peer and keys per soul. Routing only consults souls, so excess
        // keys are dropped without affecting delivery.
        let mesh = Mesh::new(Gun::new(GunOptions::default()), MeshConfig::default());
        attach_peer(&mesh, "subscriber");
        for i in 0..(MAX_KEYS_PER_SOUL + 50) {
            let frame = format!(
                r###"{{"#":"kc{i}","get":{{"#":"kc-soul",".":"key{i}"}}}}"###
            );
            mesh.hear(&frame, "subscriber");
        }
        let inner = lock_mut(&mesh.inner);
        let keys = inner
            .peers
            .get("subscriber")
            .and_then(|p| p.subscriptions.get("kc-soul"))
            .map(|s| s.len())
            .unwrap_or(0);
        assert!(keys <= MAX_KEYS_PER_SOUL, "keys bounded, got {}", keys);
        assert!(keys >= MAX_KEYS_PER_SOUL - 1, "cap not off-by-many: {}", keys);
    }

    #[test]
    fn bye_applies_registered_writes() {
        let mesh = new_mesh();
        attach_peer(&mesh, "browser");

        // Browser registers: on disconnect, set status=offline.
        mesh.hear(
            r###"{"#":"bye1","bye":{"users/alice":{"status":"offline"}}}"###,
            "browser",
        );
        assert_eq!(mesh.gun().get("users/alice").get("status").val(), None);

        mesh.bye("browser");
        assert_eq!(
            mesh.gun().get("users/alice").get("status").val(),
            Some(GunValue::Text("offline".into()))
        );
        assert_eq!(mesh.near(), 0);
    }

    #[test]
    fn axe_routes_puts_to_subscribers_only() {
        let config = MeshConfig {
            axe: true,
            ..Default::default()
        };
        let mesh = Mesh::new(Gun::new(GunOptions::default()), config);
        let writer = attach_peer(&mesh, "writer");
        let subscribed = attach_peer(&mesh, "subscribed");
        let bystander = attach_peer(&mesh, "bystander");

        // "subscribed" asks for the soul — that records a subscription.
        mesh.hear(r###"{"#":"sub-get","get":{"#":"feed"}}"###, "subscribed");
        assert_eq!(
            mesh.peer_subscriptions("subscribed"),
            vec!["feed".to_string()]
        );

        writer.lock().unwrap().clear();
        subscribed.lock().unwrap().clear();
        bystander.lock().unwrap().clear();

        mesh.hear(
            &put_raw("axe1", "feed", "post", GunValue::Text("hi".into()), 100.0),
            "writer",
        );

        assert!(
            frames(&subscribed)
                .iter()
                .any(|f| f.contains(r###""#":"axe1""###)),
            "subscribed peer receives the PUT"
        );
        assert!(
            !frames(&bystander)
                .iter()
                .any(|f| f.contains(r###""#":"axe1""###)),
            "unsubscribed peer is skipped under AXE"
        );

        // GETs still broadcast (the subscription-miss fallback).
        bystander.lock().unwrap().clear();
        mesh.hear(r###"{"#":"axe-get","get":{"#":"elsewhere"}}"###, "writer");
        assert!(
            frames(&bystander)
                .iter()
                .any(|f| f.contains(r###""#":"axe-get""###)),
            "GETs broadcast to all peers"
        );
    }

    #[test]
    fn axe_disabled_broadcasts_puts() {
        let mesh = new_mesh(); // axe: false
        attach_peer(&mesh, "writer");
        let bystander = attach_peer(&mesh, "bystander");
        bystander.lock().unwrap().clear();

        mesh.hear(
            &put_raw("noaxe1", "feed", "post", GunValue::Text("hi".into()), 100.0),
            "writer",
        );
        assert!(
            frames(&bystander)
                .iter()
                .any(|f| f.contains(r###""#":"noaxe1""###)),
            "DAM default is brute-force broadcast"
        );
    }

    #[test]
    fn duplicate_connection_resolved_deterministically() {
        let config = MeshConfig {
            pid: "ZZZ".into(), // our pid is higher than theirs
            ..Default::default()
        };
        let mesh = Mesh::new(Gun::new(GunOptions::default()), config);

        // Inbound link (no url) and outbound link (url) to the same process.
        attach_peer(&mesh, "inbound-link");
        let outbox: Outbox = Arc::new(Mutex::new(Vec::new()));
        let o = outbox.clone();
        mesh.hi(
            "outbound-link",
            Some("ws://them.example/gun".into()),
            Some(std::sync::Arc::new(move |raw: &str| {
                o.lock().unwrap().push(raw.to_string());
            })),
        );
        assert_eq!(mesh.near(), 2);

        // Both links report the same remote pid "AAA" (< ours): the
        // connection dialed by the higher-pid side — our outbound link —
        // survives; the inbound duplicate is dropped.
        mesh.hear(r###"{"#":"dc1","dam":"?","pid":"AAA"}"###, "outbound-link");
        mesh.hear(r###"{"#":"dc2","dam":"?","pid":"AAA"}"###, "inbound-link");

        assert_eq!(mesh.near(), 1, "duplicate link dropped");
        assert!(
            mesh.is_peer("outbound-link"),
            "outbound link kept (our pid is higher)"
        );
        assert!(!mesh.is_peer("inbound-link"));
    }

    #[test]
    fn mesh_config_from_options() {
        let opts = crate::instance::GunOptions {
            pid: Some("opt-pid".into()),
            gap: Duration::from_millis(25),
            axe: true,
            ..Default::default()
        };
        let config = MeshConfig::from_options(&opts);
        assert_eq!(config.pid, "opt-pid");
        assert_eq!(config.gap, Duration::from_millis(25));
        assert!(config.axe);

        let random = MeshConfig::from_options(&crate::instance::GunOptions {
            pid: None,
            ..Default::default()
        });
        assert_eq!(random.pid.len(), 9, "random pid generated");
    }

    #[test]
    fn bye_writes_rejected_for_user_namespaces() {
        let mesh = new_mesh();
        attach_peer(&mesh, "attacker");

        // Bye writes carry no signatures: letting them into `~pubKey/...`
        // souls would bypass verify_user_node. They must be dropped.
        mesh.hear(
            r###"{"#":"byeu1","bye":{"~victimpubkey/profile":{"name":"hacked"},"plain":{"k":"ok"}}}"###,
            "attacker",
        );
        mesh.bye("attacker");

        assert_eq!(
            mesh.gun().get("~victimpubkey/profile").get("name").val(),
            None,
            "user-namespace bye write must not apply"
        );
        assert_eq!(
            mesh.gun().get("plain").get("k").val(),
            Some(GunValue::Text("ok".into())),
            "plain-soul bye write still applies"
        );
    }

    #[test]
    fn bye_write_registrations_are_capped() {
        let mesh = new_mesh();
        attach_peer(&mesh, "flooder");

        for i in 0..300 {
            mesh.hear(
                &format!(
                    r###"{{"#":"byec{}","bye":{{"flood{}":{{"k":"v"}}}}}}"###,
                    i, i
                ),
                "flooder",
            );
        }
        mesh.bye("flooder");

        // Only the first MAX_BYE_WRITES_PER_PEER registrations applied.
        let applied = (0..300)
            .filter(|i| {
                mesh.gun()
                    .get(format!("flood{}", i))
                    .get("k")
                    .val()
                    .is_some()
            })
            .count();
        assert_eq!(applied, 100, "registry capped per peer");
    }

    #[test]
    fn bye_writes_capped_by_bytes_per_peer() {
        // The registry is bounded in bytes as well as count: a handful of
        // near-wire-cap graphs must not pin ~1 GB per peer until
        // disconnect.
        let mesh = Mesh::new(Gun::new(GunOptions::default()), MeshConfig::default());
        attach_peer(&mesh, "peerA");

        let small = r###"{"#":"bb1","put":{"x":{"_":{"#":"x",">":{"k":1}},"k":1}},"bye":{"bbs":{"k":"small"}}}"###;
        mesh.hear(small, "peerA");

        // ~2 MB payload blows the 1 MB per-peer byte budget — rejected.
        let blob = "B".repeat(2 * 1_048_576);
        let big = format!(
            r###"{{"#":"bb2","put":{{"y":{{"_":{{"#":"y",">":{{"k":1}}}},"k":1}}}},"bye":{{"bbl":{{"k":"{blob}"}}}}}}"###
        );
        mesh.hear(&big, "peerA");

        mesh.bye("peerA");
        assert!(
            mesh.gun().get("bbs").get("k").val().is_some(),
            "in-budget bye write applied"
        );
        assert!(
            mesh.gun().get("bbl").get("k").val().is_none(),
            "over-budget bye write rejected"
        );
    }

    #[test]
    fn gun_string_hash_matches_gun_js() {
        // Reference values computed with gun.js String.hash, which rolls
        // from the LAST character to the first:
        //   var i = s.length; while(i--){ c = ((c<<5)-c)+s.charCodeAt(i); c|=0 }
        // "" → 0; "a" → 97; "ab" → 'b' then 'a': 98*31+97 = 3135;
        // "hello" → 105835282 (reverse-order Java hash).
        assert_eq!(gun_string_hash(""), 0);
        assert_eq!(gun_string_hash("a"), 97);
        assert_eq!(gun_string_hash("ab"), 3135);
        assert_eq!(gun_string_hash("hello"), 105835282);
    }

    #[tokio::test]
    async fn mesh_works_as_async_sync_adapter() {
        use crate::transport::AsyncSyncAdapter;

        let mesh = new_mesh();
        let outbox = attach_peer(&mesh, "peerA");
        outbox.lock().unwrap().clear();

        let raw = put_raw("as1", "adapter", "k", GunValue::Number(3.0), 100.0);
        AsyncSyncAdapter::broadcast(&mesh, &raw, None).await.unwrap();

        assert!(frames(&outbox).iter().any(|f| f.contains(r###""#":"as1""###)));
        assert_eq!(mesh.connected_peers(), vec!["peerA".to_string()]);
        assert!(mesh.is_connected("peerA"));
        assert!(!mesh.is_connected("nobody"));
    }

    #[tokio::test]
    async fn hear_async_processes_large_batches() {
        let mesh = new_mesh();
        attach_peer(&mesh, "peerA");

        let msgs: Vec<String> = (0..25)
            .map(|i| {
                put_raw(
                    &format!("big{}", i),
                    &format!("soul{}", i),
                    "v",
                    GunValue::Number(i as f64),
                    100.0,
                )
            })
            .collect();
        let batch = format!("[{}]", msgs.join(","));

        mesh.hear_async(&batch, "peerA").await;

        for i in 0..25 {
            assert_eq!(
                mesh.gun().get(format!("soul{}", i)).get("v").val(),
                Some(GunValue::Number(i as f64)),
                "message {} processed",
                i
            );
        }
    }

    #[test]
    fn oversized_frame_dropped() {
        let config = MeshConfig {
            max_message_bytes: 64,
            ..Default::default()
        };
        let mesh = Mesh::new(Gun::new(GunOptions::default()), config);
        attach_peer(&mesh, "peerA");

        let raw = put_raw(
            "huge1",
            "big",
            "k",
            GunValue::Text("x".repeat(200)),
            100.0,
        );
        mesh.hear(&raw, "peerA");
        assert_eq!(mesh.gun().get("big").get("k").val(), None);
    }
}
