//! Core types for the GUN graph data model.
//!
//! GUN stores data as a graph of nodes. Each node has a soul (unique ID) and
//! contains key-value pairs. Values are either primitives (string, number,
//! boolean, null) or references to other nodes via their soul.
//!
//! Arrays are not natively supported — they require extension algorithms
//! to handle concurrency.

use std::collections::BTreeMap;
use std::fmt;

/// A soul is a unique identifier for a node in the graph.
///
/// Souls are strings, typically generated as `Gun.state().toString(36) + random(12)`.
/// User-created top-level nodes use the key as the soul (e.g., `gun.get("mark")`
/// creates a node with soul `"mark"`).
pub type Soul = String;

/// A valid GUN value.
///
/// From the source (`./valid`):
/// > Valid values are a subset of JSON: null, binary, number (!Infinity), text,
/// > or a soul relation. Arrays need special algorithms to handle concurrency,
/// > so they are not supported directly.
#[derive(Debug, Clone, PartialEq)]
pub enum GunValue {
    /// Null — used for "deletes" (tombstoning). In a CRDT you can't truly
    /// delete, only nullify.
    Null,

    /// A text string.
    Text(String),

    /// A boolean value.
    Bool(bool),

    /// A finite number (not NaN, not +/-Infinity).
    /// GUN uses JS numbers which are f64. NaN and Infinity are rejected
    /// because JSON does not support them.
    ///
    /// **L3 note:** Prefer `GunValue::number(n)` constructor which validates
    /// finiteness. Direct construction via `GunValue::Number(n)` is allowed
    /// for performance but callers must ensure `n.is_finite()`. Non-finite
    /// values will be serialized as `null` in the wire format.
    Number(f64),

    /// A reference to another node by its soul.
    /// In GUN's wire format this is `{"#": "soul_string"}`.
    Link(Soul),
}

impl GunValue {
    /// Validate that a number is a valid GUN number (finite, not NaN).
    /// Mirrors the JS check: `v != Infinity && v != -Infinity && v === v`
    pub fn number(n: f64) -> Option<Self> {
        if n.is_finite() {
            Some(GunValue::Number(n))
        } else {
            None
        }
    }

    /// Returns true if this value is Null (a tombstone/delete marker).
    pub fn is_null(&self) -> bool {
        matches!(self, GunValue::Null)
    }

    /// Returns true if this value is a link to another node.
    pub fn is_link(&self) -> bool {
        matches!(self, GunValue::Link(_))
    }

    /// If this is a Link, return the target soul.
    pub fn as_link(&self) -> Option<&str> {
        match self {
            GunValue::Link(s) => Some(s),
            _ => None,
        }
    }

    /// True if this value references a GUN node (it is a soul link).
    ///
    /// Rust equivalent of `Gun.node.is(value)` for the typed value model;
    /// for raw wire JSON objects use [`crate::wire::is_node`].
    pub fn is_node(&self) -> bool {
        self.is_link()
    }

    /// Extract the soul this value points at, if it references a node.
    ///
    /// Rust equivalent of `Gun.node.soul(value)`; for raw wire JSON use
    /// [`crate::wire::soul_of`].
    pub fn soul(&self) -> Option<&str> {
        self.as_link()
    }

    /// Returns the JSON-serialized length, used for tie-breaking in HAM.
    /// In the JS source this is `JSON.stringify(val).length`.
    pub fn json_len(&self) -> usize {
        match self {
            GunValue::Null => 4,                                // "null"
            GunValue::Bool(true) => 4,                          // "true"
            GunValue::Bool(false) => 5,                         // "false"
            GunValue::Number(n) => format!("{}", n).len(),      // numeric literal
            GunValue::Text(s) => s.len() + 2,                   // quotes
            GunValue::Link(s) => s.len() + 7,                   // {"#":"..."}
        }
    }
}

impl fmt::Display for GunValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GunValue::Null => write!(f, "null"),
            GunValue::Text(s) => write!(f, "\"{}\"", s),
            GunValue::Bool(b) => write!(f, "{}", b),
            GunValue::Number(n) => write!(f, "{}", n),
            GunValue::Link(s) => write!(f, "{{\"#\":\"{}\"}}", s),
        }
    }
}

/// Metadata for a node, stored under the `_` key in GUN's wire format.
///
/// Contains:
/// - `soul`: the node's unique ID (`_["#"]`)
/// - `states`: per-key state timestamps (`_[">"]`), used by HAM for conflict resolution
#[derive(Debug, Clone)]
pub struct NodeMeta {
    /// The node's soul (unique identifier).
    pub soul: Soul,

    /// State vector: maps each key to the timestamp when it was last written.
    /// HAM uses these to resolve conflicts — higher state wins.
    pub states: BTreeMap<String, f64>,
}

impl NodeMeta {
    pub fn new(soul: Soul) -> Self {
        Self {
            soul,
            states: BTreeMap::new(),
        }
    }
}

/// A node in the GUN graph.
///
/// Each node has:
/// - A soul (unique ID) stored in metadata
/// - A set of key-value pairs where keys are strings and values are `GunValue`
/// - A state vector tracking the last-write timestamp for each key
///
/// The `_` (metadata) key is never stored as a regular key-value pair.
#[derive(Debug, Clone)]
pub struct Node {
    /// Node metadata: soul and state vectors.
    pub meta: NodeMeta,

    /// The node's key-value data. Keys are property names, values are `GunValue`.
    /// This does NOT include the `_` metadata key.
    pub data: BTreeMap<String, GunValue>,
}

impl Node {
    /// Create a new empty node with the given soul.
    pub fn new(soul: impl Into<Soul>) -> Self {
        Self {
            meta: NodeMeta::new(soul.into()),
            data: BTreeMap::new(),
        }
    }

    /// Get the node's soul.
    pub fn soul(&self) -> &str {
        &self.meta.soul
    }

    /// Get the state timestamp for a key.
    ///
    /// Returns `-Infinity` if the key has no state, matching the JS behavior:
    /// `State.is` returns `NI` (-Infinity) if no state exists.
    pub fn state_of(&self, key: &str) -> f64 {
        self.meta.states.get(key).copied().unwrap_or(f64::NEG_INFINITY)
    }

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<&GunValue> {
        self.data.get(key)
    }

    /// Put a value with its state timestamp.
    ///
    /// Mirrors `State.ify(node, key, state, value, soul)` from the JS source.
    /// This is a low-level operation that does NOT check HAM — use `Graph::put`
    /// for conflict-resolved writes.
    pub fn put(&mut self, key: impl Into<String>, value: GunValue, state: f64) {
        let key = key.into();
        if key == "_" {
            return; // metadata key is never stored as data
        }
        self.meta.states.insert(key.clone(), state);
        self.data.insert(key, value);
    }

    /// Iterate over all key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &GunValue)> {
        self.data.iter()
    }

    /// Number of data keys (excluding metadata).
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the node has no data keys.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gun_value_number_rejects_nan() {
        assert!(GunValue::number(f64::NAN).is_none());
    }

    #[test]
    fn gun_value_number_rejects_infinity() {
        assert!(GunValue::number(f64::INFINITY).is_none());
        assert!(GunValue::number(f64::NEG_INFINITY).is_none());
    }

    #[test]
    fn gun_value_number_accepts_finite() {
        assert_eq!(GunValue::number(42.0), Some(GunValue::Number(42.0)));
        assert_eq!(GunValue::number(0.0), Some(GunValue::Number(0.0)));
        assert_eq!(GunValue::number(-3.14), Some(GunValue::Number(-3.14)));
    }

    #[test]
    fn gun_value_link() {
        let v = GunValue::Link("abc".into());
        assert!(v.is_link());
        assert_eq!(v.as_link(), Some("abc"));
    }

    #[test]
    fn node_put_get() {
        let mut node = Node::new("test");
        node.put("name", GunValue::Text("Alice".into()), 1.0);
        assert_eq!(node.get("name"), Some(&GunValue::Text("Alice".into())));
        assert_eq!(node.state_of("name"), 1.0);
    }

    #[test]
    fn node_ignores_underscore_key() {
        let mut node = Node::new("test");
        node.put("_", GunValue::Text("should not store".into()), 1.0);
        assert!(node.get("_").is_none());
        assert!(node.is_empty());
    }

    #[test]
    fn node_state_of_missing_key() {
        let node = Node::new("test");
        assert_eq!(node.state_of("missing"), f64::NEG_INFINITY);
    }

    #[test]
    fn gun_value_node_introspection() {
        let link = GunValue::Link("users/alice".into());
        assert!(link.is_node());
        assert_eq!(link.soul(), Some("users/alice"));

        assert!(!GunValue::Text("users/alice".into()).is_node());
        assert_eq!(GunValue::Number(1.0).soul(), None);
    }

    #[test]
    fn gun_value_json_len() {
        assert_eq!(GunValue::Null.json_len(), 4);
        assert_eq!(GunValue::Bool(true).json_len(), 4);
        assert_eq!(GunValue::Bool(false).json_len(), 5);
        assert_eq!(GunValue::Text("hello".into()).json_len(), 7); // "hello"
        assert_eq!(GunValue::Link("abc".into()).json_len(), 10); // {"#":"abc"}
    }
}
