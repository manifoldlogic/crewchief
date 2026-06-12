//! Wire format serialization/deserialization for the GUN protocol.
//!
//! GUN uses a JSON-based wire format for communication between peers.
//! This module handles conversion between Rust types and GUN's wire format.
//!
//! ## Wire Message Types
//!
//! **PUT** (write):
//! ```json
//! {
//!   "#": "msgId",
//!   "put": {
//!     "soul1": {
//!       "_": { "#": "soul1", ">": { "key1": 123456789.001 } },
//!       "key1": "value1",
//!       "key2": { "#": "other-soul" }
//!     }
//!   }
//! }
//! ```
//!
//! **GET** (read):
//! ```json
//! { "#": "msgId", "get": { "#": "soul", ".": "key" } }
//! ```
//!
//! **ACK** (acknowledgment):
//! ```json
//! { "@": "msgId", "ok": { "": 1 } }
//! { "@": "msgId", "err": "Error message" }
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::types::{GunValue, Node};

/// A GUN wire message.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WireMessage {
    /// Message ID.
    #[serde(rename = "#", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Acknowledgment reference (ID of message being acknowledged).
    #[serde(rename = "@", skip_serializing_if = "Option::is_none")]
    pub ack: Option<String>,

    /// PUT data — a graph of nodes keyed by soul.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<Value>,

    /// GET request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<GetRequest>,

    /// Success acknowledgment. DAM uses `{ "@": hops, "/": nearPeerCount }`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<Value>,

    /// Error message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err: Option<String>,

    /// Content hash of `put` data for hash-based deduplication (DAM).
    /// GUN emits this as a number (string hashes also appear in the wild).
    #[serde(rename = "##", skip_serializing_if = "Option::is_none")]
    pub hash: Option<Value>,

    /// Peer exclusion list: comma-separated peer IDs that have already
    /// seen this message (DAM). Each relay replaces this field.
    #[serde(rename = "><", skip_serializing_if = "Option::is_none")]
    pub seen_by: Option<String>,

    /// DAM protocol message type (`?`, `!`, `mob`, `opt`, …).
    /// Protocol messages are consumed by the mesh and never relayed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dam: Option<String>,

    /// Process ID, exchanged in the DAM `?` handshake.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<String>,

    /// Mob rebalancing payload: peer count (`{dam:'mob', mob:count, peers:…}`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mob: Option<Value>,

    /// Peer introduction payload (`{dam:'opt', opt:{peers:'url'}}`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opt: Option<Value>,

    /// Alternative peer URLs accompanying a `mob` message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peers: Option<Value>,

    /// Disconnect-write registration (`gun/lib/bye.js`): a graph of
    /// `{soul: {key: value}}` writes to apply when this peer disconnects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bye: Option<Value>,
}

/// A GET request specifying what to read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRequest {
    /// The soul of the node to read.
    #[serde(rename = "#")]
    pub soul: String,

    /// Optional key to read (if omitted, read the whole node).
    #[serde(rename = ".", skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

// ── Serialization (Rust → JSON) ─────────────────────────────────────────

/// Serialize a `GunValue` to a JSON `Value`.
pub fn value_to_json(val: &GunValue) -> Value {
    match val {
        GunValue::Null => Value::Null,
        GunValue::Text(s) => Value::String(s.clone()),
        GunValue::Bool(b) => Value::Bool(*b),
        GunValue::Number(n) => serde_json::Number::from_f64(*n)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        GunValue::Link(soul) => {
            let mut m = Map::new();
            m.insert("#".to_string(), Value::String(soul.clone()));
            Value::Object(m)
        }
    }
}

/// Serialize a `Node` to a JSON `Value` in GUN's wire format.
///
/// Output format:
/// ```json
/// {
///   "_": { "#": "soul", ">": { "key1": 123.0, "key2": 456.0 } },
///   "key1": "value1",
///   "key2": { "#": "other-soul" }
/// }
/// ```
pub fn node_to_json(node: &Node) -> Value {
    let mut obj = Map::new();

    // Metadata: _
    let mut meta = Map::new();
    meta.insert("#".to_string(), Value::String(node.soul().to_string()));

    let mut states = Map::new();
    for (key, &state) in &node.meta.states {
        if let Some(n) = serde_json::Number::from_f64(state) {
            states.insert(key.clone(), Value::Number(n));
        }
    }
    meta.insert(">".to_string(), Value::Object(states));
    obj.insert("_".to_string(), Value::Object(meta));

    // Data keys
    for (key, value) in node.iter() {
        obj.insert(key.clone(), value_to_json(value));
    }

    Value::Object(obj)
}

/// Serialize a graph (multiple nodes) to the PUT wire format.
///
/// Output: `{ "soul1": { ... }, "soul2": { ... } }`
pub fn graph_to_json(nodes: &[&Node]) -> Value {
    let mut obj = Map::new();
    for node in nodes {
        obj.insert(node.soul().to_string(), node_to_json(node));
    }
    Value::Object(obj)
}

/// Build a PUT wire message from nodes.
pub fn put_message(id: &str, nodes: &[&Node]) -> WireMessage {
    WireMessage {
        id: Some(id.to_string()),
        put: Some(graph_to_json(nodes)),
        ..Default::default()
    }
}

/// Build a GET wire message.
pub fn get_message(id: &str, soul: &str, key: Option<&str>) -> WireMessage {
    WireMessage {
        id: Some(id.to_string()),
        get: Some(GetRequest {
            soul: soul.to_string(),
            key: key.map(|k| k.to_string()),
        }),
        ..Default::default()
    }
}

/// Build an ACK (success) wire message.
pub fn ack_ok(ack_id: &str) -> WireMessage {
    let mut ok = Map::new();
    ok.insert(
        "".to_string(),
        Value::Number(serde_json::Number::from(1)),
    );
    WireMessage {
        ack: Some(ack_id.to_string()),
        ok: Some(Value::Object(ok)),
        ..Default::default()
    }
}

/// Build an ACK (error) wire message.
pub fn ack_err(ack_id: &str, error: &str) -> WireMessage {
    WireMessage {
        ack: Some(ack_id.to_string()),
        err: Some(error.to_string()),
        ..Default::default()
    }
}

// ── Node introspection ──────────────────────────────────────────────────

/// True if a JSON value is a GUN node object: an object carrying `_.#`
/// soul metadata. Mirrors `Gun.node.is(value)`.
pub fn is_node(value: &Value) -> bool {
    soul_of(value).is_some()
}

/// Extract the soul from a JSON value: either a full node object
/// (`{"_": {"#": soul, ...}, ...}`) or a bare link (`{"#": soul}`).
/// Mirrors `Gun.node.soul(value)`.
pub fn soul_of(value: &Value) -> Option<&str> {
    let obj = value.as_object()?;
    if let Some(meta) = obj.get("_").and_then(|m| m.as_object()) {
        return meta.get("#").and_then(|s| s.as_str());
    }
    if obj.len() == 1 {
        return obj.get("#").and_then(|s| s.as_str());
    }
    None
}

// ── Deserialization (JSON → Rust) ───────────────────────────────────────

/// Parse a JSON `Value` into a `GunValue`.
///
/// Follows the valid() check from gun.js:
/// - null → Null
/// - string → Text
/// - boolean → Bool
/// - finite number → Number
/// - `{"#": "soul"}` (single-key object) → Link
/// - anything else → None (invalid)
pub fn json_to_value(val: &Value) -> Option<GunValue> {
    match val {
        Value::Null => Some(GunValue::Null),
        Value::String(s) => Some(GunValue::Text(s.clone())),
        Value::Bool(b) => Some(GunValue::Bool(*b)),
        Value::Number(n) => n.as_f64().and_then(|f| {
            if f.is_finite() {
                Some(GunValue::Number(f))
            } else {
                None
            }
        }),
        Value::Object(obj) => {
            // A link is {"#": "soul"} — exactly one key "#" with a string value
            if obj.len() == 1
                && let Some(Value::String(soul)) = obj.get("#") {
                    return Some(GunValue::Link(soul.clone()));
                }
            None // non-link objects are not valid GunValues
        }
        Value::Array(_) => None, // arrays are not valid in GUN
    }
}

/// Parse a JSON node object into a `Node`.
///
/// Expects the format:
/// ```json
/// {
///   "_": { "#": "soul", ">": { "key": state } },
///   "key": value,
///   ...
/// }
/// ```
pub fn json_to_node(val: &Value) -> Option<Node> {
    let obj = val.as_object()?;

    // Extract metadata
    let meta = obj.get("_")?.as_object()?;
    let soul = meta.get("#")?.as_str()?;
    let states_obj = meta.get(">")?.as_object()?;

    let mut node = Node::new(soul);

    // Parse state vectors
    let mut states = BTreeMap::new();
    for (k, v) in states_obj {
        if let Some(s) = v.as_f64() {
            states.insert(k.clone(), s);
        }
    }

    // Parse data keys
    for (key, value) in obj {
        if key == "_" {
            continue;
        }
        if let Some(gun_val) = json_to_value(value) {
            let state = states.get(key).copied().unwrap_or(f64::NEG_INFINITY);
            node.put(key, gun_val, state);
        }
    }

    Some(node)
}

/// Maximum number of nodes in a single PUT message.
pub const MAX_NODES_PER_MESSAGE: usize = 1000;

/// Maximum number of keys per node.
pub const MAX_KEYS_PER_NODE: usize = 10_000;

/// Maximum wire message size in bytes.
pub const MAX_MESSAGE_BYTES: usize = 10 * 1024 * 1024; // 10 MB

/// Parse a PUT message's graph payload into a vec of nodes.
///
/// The payload is `{ "soul1": nodeObj, "soul2": nodeObj, ... }`.
/// Validates that the outer key matches the inner soul metadata.
/// Enforces limits on node count and keys per node to prevent DoS.
pub fn json_to_graph(val: &Value) -> Vec<Node> {
    let mut nodes = Vec::new();
    if let Some(obj) = val.as_object() {
        for (outer_soul, node_val) in obj {
            if nodes.len() >= MAX_NODES_PER_MESSAGE {
                break; // H3: cap node count
            }
            if let Some(node) = json_to_node(node_val) {
                // H6: validate outer key matches inner soul
                if node.soul() != outer_soul {
                    continue; // soul mismatch — skip this node
                }
                if node.len() > MAX_KEYS_PER_NODE {
                    continue; // too many keys — skip
                }
                nodes.push(node);
            }
        }
    }
    nodes
}

/// Parse a complete wire message from JSON.
///
/// Enforces a maximum message size to prevent memory exhaustion (H3).
pub fn parse_message(json: &str) -> Result<WireMessage, serde_json::Error> {
    if json.len() > MAX_MESSAGE_BYTES {
        return Err(serde::de::Error::custom(format!(
            "message exceeds size limit ({} > {} bytes)",
            json.len(),
            MAX_MESSAGE_BYTES
        )));
    }
    serde_json::from_str(json)
}

/// Serialize a wire message to JSON.
pub fn serialize_message(msg: &WireMessage) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_roundtrip_null() {
        let v = GunValue::Null;
        let json = value_to_json(&v);
        assert_eq!(json, Value::Null);
        assert_eq!(json_to_value(&json), Some(GunValue::Null));
    }

    #[test]
    fn value_roundtrip_text() {
        let v = GunValue::Text("hello".into());
        let json = value_to_json(&v);
        assert_eq!(json, Value::String("hello".into()));
        assert_eq!(json_to_value(&json), Some(v));
    }

    #[test]
    fn value_roundtrip_bool() {
        let json = value_to_json(&GunValue::Bool(true));
        assert_eq!(json, Value::Bool(true));
        assert_eq!(json_to_value(&json), Some(GunValue::Bool(true)));
    }

    #[test]
    fn value_roundtrip_number() {
        let v = GunValue::Number(42.5);
        let json = value_to_json(&v);
        assert_eq!(json_to_value(&json), Some(v));
    }

    #[test]
    fn value_roundtrip_link() {
        let v = GunValue::Link("abc123".into());
        let json = value_to_json(&v);
        // Should be {"#": "abc123"}
        let obj = json.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        assert_eq!(obj.get("#").unwrap().as_str().unwrap(), "abc123");
        assert_eq!(json_to_value(&json), Some(v));
    }

    #[test]
    fn invalid_json_values() {
        // Array is not valid
        assert_eq!(json_to_value(&Value::Array(vec![])), None);
        // Multi-key object is not a link
        let mut obj = Map::new();
        obj.insert("#".into(), Value::String("a".into()));
        obj.insert("x".into(), Value::String("b".into()));
        assert_eq!(json_to_value(&Value::Object(obj)), None);
    }

    #[test]
    fn node_roundtrip() {
        let mut node = Node::new("mark");
        node.put("name", GunValue::Text("Mark Nadal".into()), 1000.0);
        node.put("age", GunValue::Number(30.0), 1000.0);
        node.put("boss", GunValue::Link("fluffy".into()), 1001.0);

        let json = node_to_json(&node);
        let parsed = json_to_node(&json).unwrap();

        assert_eq!(parsed.soul(), "mark");
        assert_eq!(
            parsed.get("name"),
            Some(&GunValue::Text("Mark Nadal".into()))
        );
        assert_eq!(parsed.get("age"), Some(&GunValue::Number(30.0)));
        assert_eq!(
            parsed.get("boss"),
            Some(&GunValue::Link("fluffy".into()))
        );
        assert_eq!(parsed.state_of("name"), 1000.0);
        assert_eq!(parsed.state_of("boss"), 1001.0);
    }

    #[test]
    fn node_json_format() {
        let mut node = Node::new("test");
        node.put("x", GunValue::Number(1.0), 100.0);

        let json = node_to_json(&node);
        let obj = json.as_object().unwrap();

        // Must have _ metadata
        let meta = obj.get("_").unwrap().as_object().unwrap();
        assert_eq!(meta.get("#").unwrap().as_str().unwrap(), "test");
        let states = meta.get(">").unwrap().as_object().unwrap();
        assert_eq!(states.get("x").unwrap().as_f64().unwrap(), 100.0);

        // Must have data key
        assert_eq!(obj.get("x").unwrap().as_f64().unwrap(), 1.0);
    }

    #[test]
    fn graph_serialization() {
        let mut n1 = Node::new("a");
        n1.put("val", GunValue::Number(1.0), 100.0);
        let mut n2 = Node::new("b");
        n2.put("val", GunValue::Number(2.0), 100.0);

        let json = graph_to_json(&[&n1, &n2]);
        let obj = json.as_object().unwrap();
        assert!(obj.contains_key("a"));
        assert!(obj.contains_key("b"));

        let nodes = json_to_graph(&json);
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn put_message_format() {
        let mut node = Node::new("mark");
        node.put("name", GunValue::Text("Mark".into()), 100.0);

        let msg = put_message("abc123", &[&node]);
        let json_str = serialize_message(&msg).unwrap();
        let parsed: WireMessage = parse_message(&json_str).unwrap();

        assert_eq!(parsed.id.as_deref(), Some("abc123"));
        assert!(parsed.put.is_some());
        assert!(parsed.get.is_none());
    }

    #[test]
    fn get_message_format() {
        let msg = get_message("def456", "mark", Some("name"));
        let json_str = serialize_message(&msg).unwrap();
        let parsed: WireMessage = parse_message(&json_str).unwrap();

        assert_eq!(parsed.id.as_deref(), Some("def456"));
        let get = parsed.get.unwrap();
        assert_eq!(get.soul, "mark");
        assert_eq!(get.key.as_deref(), Some("name"));
    }

    #[test]
    fn ack_messages() {
        let ok = ack_ok("msg1");
        assert_eq!(ok.ack.as_deref(), Some("msg1"));
        assert!(ok.ok.is_some());
        assert!(ok.err.is_none());

        let err = ack_err("msg2", "not found");
        assert_eq!(err.ack.as_deref(), Some("msg2"));
        assert!(err.ok.is_none());
        assert_eq!(err.err.as_deref(), Some("not found"));
    }

    #[test]
    fn parse_external_put() {
        // Simulate a message from an actual GUN peer.
        // Build JSON programmatically to avoid raw string issues with "#" in Rust 2024.
        let mut mark_meta = Map::new();
        mark_meta.insert("#".into(), Value::String("mark".into()));
        let mut mark_states = Map::new();
        mark_states.insert("name".into(), serde_json::json!(1000));
        mark_states.insert("email".into(), serde_json::json!(1001));
        mark_meta.insert(">".into(), Value::Object(mark_states));

        let mut mark_node = Map::new();
        mark_node.insert("_".into(), Value::Object(mark_meta));
        mark_node.insert("name".into(), Value::String("Mark Nadal".into()));
        mark_node.insert("email".into(), Value::String("mark@gun.eco".into()));

        let mut put_graph = Map::new();
        put_graph.insert("mark".into(), Value::Object(mark_node));

        let mut msg_obj = Map::new();
        msg_obj.insert("#".into(), Value::String("xyz789".into()));
        msg_obj.insert("put".into(), Value::Object(put_graph));

        let json = serde_json::to_string(&Value::Object(msg_obj)).unwrap();
        let msg: WireMessage = parse_message(&json).unwrap();
        assert_eq!(msg.id.as_deref(), Some("xyz789"));
        let nodes = json_to_graph(msg.put.as_ref().unwrap());
        assert_eq!(nodes.len(), 1);
        let node = &nodes[0];
        assert_eq!(node.soul(), "mark");
        assert_eq!(
            node.get("name"),
            Some(&GunValue::Text("Mark Nadal".into()))
        );
        assert_eq!(
            node.get("email"),
            Some(&GunValue::Text("mark@gun.eco".into()))
        );
    }

    #[test]
    fn parse_external_get() {
        let mut get_inner = Map::new();
        get_inner.insert("#".into(), Value::String("users".into()));
        get_inner.insert(".".into(), Value::String("alice".into()));

        let mut msg_obj = Map::new();
        msg_obj.insert("#".into(), Value::String("abc".into()));
        msg_obj.insert("get".into(), Value::Object(get_inner));

        let json = serde_json::to_string(&Value::Object(msg_obj)).unwrap();
        let msg: WireMessage = parse_message(&json).unwrap();
        let get = msg.get.unwrap();
        assert_eq!(get.soul, "users");
        assert_eq!(get.key.as_deref(), Some("alice"));
    }

    #[test]
    fn node_introspection_helpers() {
        let mut node = Node::new("mark");
        node.put("name", GunValue::Text("Mark".into()), 1.0);
        let json = node_to_json(&node);
        assert!(is_node(&json));
        assert_eq!(soul_of(&json), Some("mark"));

        // A bare link also resolves to its soul.
        let link = value_to_json(&GunValue::Link("fluffy".into()));
        assert_eq!(soul_of(&link), Some("fluffy"));

        // Plain objects and primitives are not nodes.
        assert!(!is_node(&serde_json::json!({"a": 1})));
        assert!(!is_node(&serde_json::json!("text")));
        assert_eq!(soul_of(&serde_json::json!({"a": 1, "#": "x"})), None);
    }

    #[test]
    fn null_tombstone_roundtrip() {
        let mut node = Node::new("test");
        node.put("deleted", GunValue::Null, 200.0);

        let json = node_to_json(&node);
        let parsed = json_to_node(&json).unwrap();
        assert_eq!(parsed.get("deleted"), Some(&GunValue::Null));
        assert_eq!(parsed.state_of("deleted"), 200.0);
    }
}
