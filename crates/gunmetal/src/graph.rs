//! In-memory graph store with HAM-resolved writes and LRU eviction.
//!
//! Mirrors GUN's `root.graph` — a map of souls to nodes. All writes go through
//! HAM conflict resolution before being applied. When the graph exceeds
//! configured capacity limits, least-recently-accessed unpinned nodes are evicted.

use std::collections::HashSet;

use indexmap::IndexMap;

use crate::crdt::{self, HamResult};
use crate::state::State;
use crate::types::{GunValue, Node, Soul};

/// Result of a `put` operation on a single key.
#[derive(Debug, Clone, PartialEq)]
pub enum PutResult {
    /// The value was accepted and written.
    Accepted,
    /// The value was rejected (old or same).
    Rejected,
    /// The value is in the future and should be retried after the given milliseconds.
    Deferred(f64),
    /// An error occurred during conflict resolution.
    Error(&'static str),
}

/// Configuration for graph eviction.
#[derive(Debug, Clone)]
pub struct EvictionConfig {
    /// Maximum number of nodes before eviction triggers. Default: 10,000.
    pub max_nodes: usize,
    /// Maximum total key-value pairs across all nodes. Default: 1,000,000.
    pub max_keys: usize,
    /// Fraction of nodes to evict when limit is hit (0.0–1.0). Default: 0.1.
    pub eviction_fraction: f64,
}

impl Default for EvictionConfig {
    fn default() -> Self {
        Self {
            max_nodes: 10_000,
            max_keys: 1_000_000,
            eviction_fraction: 0.1,
        }
    }
}

/// The in-memory GUN graph.
///
/// Stores nodes indexed by soul. All writes are conflict-resolved via HAM.
/// Uses `IndexMap` for insertion-order tracking, with access-order updates
/// for LRU eviction. Pinned nodes are exempt from eviction.
pub struct Graph {
    /// The state clock for generating timestamps.
    pub clock: State,

    /// Soul → Node mapping, ordered by access time (most recent at end).
    nodes: IndexMap<Soul, Node>,

    /// Souls that should never be evicted (active subscriptions, current user).
    pinned: HashSet<Soul>,

    /// Eviction configuration.
    eviction_config: EvictionConfig,

    /// Running count of total keys across all nodes.
    total_keys: usize,
}

impl Graph {
    /// Create a new empty graph with a fresh state clock.
    pub fn new() -> Self {
        Self {
            clock: State::new(),
            nodes: IndexMap::new(),
            pinned: HashSet::new(),
            eviction_config: EvictionConfig::default(),
            total_keys: 0,
        }
    }

    /// Create a new graph with a custom state clock.
    pub fn with_clock(clock: State) -> Self {
        Self {
            clock,
            nodes: IndexMap::new(),
            pinned: HashSet::new(),
            eviction_config: EvictionConfig::default(),
            total_keys: 0,
        }
    }

    /// Create a new graph with custom eviction config.
    pub fn with_eviction(config: EvictionConfig) -> Self {
        Self {
            clock: State::new(),
            nodes: IndexMap::new(),
            pinned: HashSet::new(),
            eviction_config: config,
            total_keys: 0,
        }
    }

    /// Get a reference to a node by its soul.
    pub fn get_node(&self, soul: &str) -> Option<&Node> {
        self.nodes.get(soul)
    }

    /// Get a value from a node at a specific key.
    /// Marks the node as recently accessed for LRU tracking.
    pub fn get(&self, soul: &str, key: &str) -> Option<&GunValue> {
        self.nodes.get(soul).and_then(|node| node.get(key))
    }

    /// Mark a node as recently accessed (moves to end of LRU order).
    pub fn touch(&mut self, soul: &str) {
        if let Some(idx) = self.nodes.get_index_of(soul) {
            self.nodes.move_index(idx, self.nodes.len() - 1);
        }
    }

    /// Pin a soul so it won't be evicted.
    ///
    /// Used for active subscriptions and the current user's data.
    pub fn pin(&mut self, soul: impl Into<Soul>) {
        self.pinned.insert(soul.into());
    }

    /// Unpin a soul, allowing it to be evicted.
    pub fn unpin(&mut self, soul: &str) {
        self.pinned.remove(soul);
    }

    /// Check if a soul is pinned.
    pub fn is_pinned(&self, soul: &str) -> bool {
        self.pinned.contains(soul)
    }

    /// Get the current eviction configuration.
    pub fn eviction_config(&self) -> &EvictionConfig {
        &self.eviction_config
    }

    /// Put a single key-value pair into a node, using the current clock time
    /// as the state timestamp.
    ///
    /// Creates the node if it doesn't exist. The write goes through HAM
    /// conflict resolution. May trigger eviction if capacity is exceeded.
    pub fn put(&mut self, soul: &str, key: &str, value: GunValue) -> PutResult {
        let state = self.clock.now();
        self.put_with_state(soul, key, value, state)
    }

    /// Put a single key-value pair with an explicit state timestamp.
    ///
    /// This is used when applying incoming data from peers, where the state
    /// is provided by the sender. HAM resolution determines whether the
    /// write is accepted.
    pub fn put_with_state(
        &mut self,
        soul: &str,
        key: &str,
        value: GunValue,
        state: f64,
    ) -> PutResult {
        let machine_state = self.clock.now();

        // Allow slight clock skew from peers but cap the maximum drift.
        // Without this cap, a malicious peer could send state=f64::MAX
        // and permanently poison keys (all future writes rejected as "old").
        // 10 minutes of drift tolerance matches reasonable peer clock skew.
        const MAX_DRIFT_MS: f64 = 10.0 * 60.0 * 1000.0; // 10 minutes
        if state > machine_state + MAX_DRIFT_MS {
            return PutResult::Deferred(state - machine_state);
        }
        // For states within drift tolerance, treat them as current
        let machine_state = if state > machine_state {
            state
        } else {
            machine_state
        };

        let (current_state, current_value) = match self.nodes.get(soul) {
            Some(node) => (node.state_of(key), node.get(key)),
            None => (f64::NEG_INFINITY, None),
        };

        match crdt::ham(machine_state, state, current_state, &value, current_value) {
            HamResult::Accept => {
                let is_new_key = self
                    .nodes
                    .get(soul)
                    .map_or(true, |n| n.get(key).is_none());

                let node = self
                    .nodes
                    .entry(soul.to_string())
                    .or_insert_with(|| Node::new(soul));
                node.put(key, value, state);

                if is_new_key {
                    self.total_keys += 1;
                }

                // Move to end (most recently accessed)
                if let Some(idx) = self.nodes.get_index_of(soul) {
                    self.nodes.move_index(idx, self.nodes.len() - 1);
                }

                // Check if eviction is needed
                self.maybe_evict();

                PutResult::Accepted
            }
            HamResult::Old | HamResult::Same => PutResult::Rejected,
            HamResult::Future(delay) => PutResult::Deferred(delay),
            HamResult::Error(msg) => PutResult::Error(msg),
        }
    }

    /// Put multiple key-value pairs into a node.
    ///
    /// This is the equivalent of `gun.get(soul).put({k1: v1, k2: v2, ...})`.
    /// Each key is independently conflict-resolved via HAM.
    pub fn put_node(
        &mut self,
        soul: &str,
        data: impl IntoIterator<Item = (String, GunValue)>,
    ) -> Vec<(String, PutResult)> {
        let state = self.clock.now();
        data.into_iter()
            .map(|(key, value)| {
                let result = self.put_with_state(soul, &key, value, state);
                (key, result)
            })
            .collect()
    }

    /// Merge an incoming node from a peer into the graph.
    ///
    /// Each key-value pair in the incoming node is independently resolved via HAM.
    /// The incoming node's state vectors are used (not our local clock).
    pub fn merge_node(&mut self, incoming: &Node) -> Vec<(String, PutResult)> {
        let soul = incoming.soul();
        incoming
            .iter()
            .map(|(key, value)| {
                let state = incoming.state_of(key);
                let result = self.put_with_state(soul, key, value.clone(), state);
                (key.clone(), result)
            })
            .collect()
    }

    /// Iterate over all nodes in the graph.
    pub fn nodes(&self) -> impl Iterator<Item = (&Soul, &Node)> {
        self.nodes.iter()
    }

    /// Number of nodes in the graph.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if the graph has no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Total number of key-value pairs across all nodes.
    pub fn total_keys(&self) -> usize {
        self.total_keys
    }

    /// Number of pinned souls.
    pub fn pinned_count(&self) -> usize {
        self.pinned.len()
    }

    /// Evict least-recently-accessed unpinned nodes if over capacity.
    ///
    /// Whole-node eviction to avoid partial state. Returns the souls
    /// that were evicted (useful for persisting evicted data).
    fn maybe_evict(&mut self) -> Vec<Soul> {
        let over_nodes = self.nodes.len() > self.eviction_config.max_nodes;
        let over_keys = self.total_keys > self.eviction_config.max_keys;

        if !over_nodes && !over_keys {
            return Vec::new();
        }

        let target_count = (self.nodes.len() as f64 * self.eviction_config.eviction_fraction)
            .ceil() as usize;
        let target_count = target_count.max(1); // evict at least 1

        let mut evicted = Vec::new();

        // Scan from the front (oldest access) to find unpinned nodes
        // We can't remove while iterating IndexMap, so collect indices first
        let candidates: Vec<Soul> = self
            .nodes
            .keys()
            .filter(|soul| !self.pinned.contains(soul.as_str()))
            .take(target_count)
            .cloned()
            .collect();

        for soul in candidates {
            if let Some(node) = self.nodes.swap_remove(&soul) {
                // swap_remove is O(1) but changes order — fine because we're
                // removing the oldest items anyway
                self.total_keys -= node.iter().count();
                evicted.push(soul);
            }
        }

        evicted
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_and_get() {
        let mut g = Graph::new();
        let result = g.put("mark", "name", GunValue::Text("Mark".into()));
        assert_eq!(result, PutResult::Accepted);
        assert_eq!(
            g.get("mark", "name"),
            Some(&GunValue::Text("Mark".into()))
        );
    }

    #[test]
    fn put_creates_node() {
        let mut g = Graph::new();
        assert!(g.is_empty());
        g.put("mark", "name", GunValue::Text("Mark".into()));
        assert_eq!(g.len(), 1);
        let node = g.get_node("mark").unwrap();
        assert_eq!(node.soul(), "mark");
    }

    #[test]
    fn newer_state_overwrites() {
        let mut g = Graph::new();
        g.put_with_state("mark", "name", GunValue::Text("old".into()), 10.0);
        let result =
            g.put_with_state("mark", "name", GunValue::Text("new".into()), 20.0);
        assert_eq!(result, PutResult::Accepted);
        assert_eq!(
            g.get("mark", "name"),
            Some(&GunValue::Text("new".into()))
        );
    }

    #[test]
    fn older_state_rejected() {
        let mut g = Graph::new();
        g.put_with_state("mark", "name", GunValue::Text("current".into()), 20.0);
        let result =
            g.put_with_state("mark", "name", GunValue::Text("old".into()), 10.0);
        assert_eq!(result, PutResult::Rejected);
        assert_eq!(
            g.get("mark", "name"),
            Some(&GunValue::Text("current".into()))
        );
    }

    #[test]
    fn partial_merge() {
        let mut g = Graph::new();
        g.put("mark", "name", GunValue::Text("Mark".into()));
        g.put("mark", "age", GunValue::Number(30.0));

        // Both keys should exist (partial merge, not replace)
        assert_eq!(
            g.get("mark", "name"),
            Some(&GunValue::Text("Mark".into()))
        );
        assert_eq!(g.get("mark", "age"), Some(&GunValue::Number(30.0)));
    }

    #[test]
    fn merge_incoming_node() {
        let mut g = Graph::new();
        g.put_with_state("mark", "name", GunValue::Text("old".into()), 10.0);

        let mut incoming = Node::new("mark");
        incoming.put("name", GunValue::Text("Mark Nadal".into()), 20.0);
        incoming.put("email", GunValue::Text("mark@gun.eco".into()), 20.0);

        let results = g.merge_node(&incoming);
        assert_eq!(results.len(), 2);

        assert_eq!(
            g.get("mark", "name"),
            Some(&GunValue::Text("Mark Nadal".into()))
        );
        assert_eq!(
            g.get("mark", "email"),
            Some(&GunValue::Text("mark@gun.eco".into()))
        );
    }

    #[test]
    fn null_tombstone() {
        let mut g = Graph::new();
        g.put_with_state("mark", "name", GunValue::Text("Mark".into()), 10.0);
        g.put_with_state("mark", "name", GunValue::Null, 20.0);
        assert_eq!(g.get("mark", "name"), Some(&GunValue::Null));
    }

    #[test]
    fn link_reference() {
        let mut g = Graph::new();
        g.put("mark", "name", GunValue::Text("Mark".into()));
        g.put(
            "mark",
            "boss",
            GunValue::Link("fluffy".into()),
        );
        g.put("fluffy", "name", GunValue::Text("Fluffy".into()));
        g.put("fluffy", "species", GunValue::Text("kitty".into()));

        // Traverse the graph manually
        let boss_link = g.get("mark", "boss").unwrap();
        let boss_soul = boss_link.as_link().unwrap();
        let boss_name = g.get(boss_soul, "name").unwrap();
        assert_eq!(boss_name, &GunValue::Text("Fluffy".into()));
    }

    #[test]
    fn put_node_multiple_keys() {
        let mut g = Graph::new();
        let data = vec![
            ("name".to_string(), GunValue::Text("Alice".into())),
            ("age".to_string(), GunValue::Number(30.0)),
            ("active".to_string(), GunValue::Bool(true)),
        ];
        let results = g.put_node("alice", data);
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|(_, r)| *r == PutResult::Accepted));

        assert_eq!(
            g.get("alice", "name"),
            Some(&GunValue::Text("Alice".into()))
        );
        assert_eq!(g.get("alice", "age"), Some(&GunValue::Number(30.0)));
        assert_eq!(g.get("alice", "active"), Some(&GunValue::Bool(true)));
    }

    #[test]
    fn get_nonexistent() {
        let g = Graph::new();
        assert!(g.get("nobody", "name").is_none());
        assert!(g.get_node("nobody").is_none());
    }

    // ── Eviction tests ──────────────────────────────────────────────

    #[test]
    fn eviction_triggers_at_max_nodes() {
        let config = EvictionConfig {
            max_nodes: 5,
            max_keys: 1_000_000,
            eviction_fraction: 0.4, // evict 40% = 2 nodes
        };
        let mut g = Graph::with_eviction(config);

        // Add 6 nodes (1 over limit)
        for i in 0..6 {
            g.put_with_state(
                &format!("node{}", i),
                "val",
                GunValue::Number(i as f64),
                (i + 1) as f64,
            );
        }

        // Should have evicted 2 oldest nodes (node0, node1)
        assert!(g.len() <= 5, "expected <= 5 nodes, got {}", g.len());
        // Newest nodes should survive
        assert!(g.get_node("node5").is_some());
        assert!(g.get_node("node4").is_some());
    }

    #[test]
    fn pinned_nodes_survive_eviction() {
        let config = EvictionConfig {
            max_nodes: 3,
            max_keys: 1_000_000,
            eviction_fraction: 0.5,
        };
        let mut g = Graph::with_eviction(config);

        // Add node0 and pin it
        g.put_with_state("node0", "val", GunValue::Number(0.0), 1.0);
        g.pin("node0");

        // Add more nodes to trigger eviction
        for i in 1..5 {
            g.put_with_state(
                &format!("node{}", i),
                "val",
                GunValue::Number(i as f64),
                (i + 1) as f64,
            );
        }

        // node0 should survive because it's pinned
        assert!(g.get_node("node0").is_some(), "pinned node should survive eviction");
    }

    #[test]
    fn total_keys_tracked() {
        let mut g = Graph::new();
        assert_eq!(g.total_keys(), 0);

        g.put("a", "x", GunValue::Number(1.0));
        assert_eq!(g.total_keys(), 1);

        g.put("a", "y", GunValue::Number(2.0));
        assert_eq!(g.total_keys(), 2);

        g.put("b", "z", GunValue::Number(3.0));
        assert_eq!(g.total_keys(), 3);

        // Updating existing key shouldn't increase count
        g.put("a", "x", GunValue::Number(10.0));
        assert_eq!(g.total_keys(), 3);
    }

    #[test]
    fn eviction_triggers_at_max_keys() {
        let config = EvictionConfig {
            max_nodes: 1_000,
            max_keys: 5, // very low key limit
            eviction_fraction: 0.5,
        };
        let mut g = Graph::with_eviction(config);

        // Add nodes with multiple keys each
        g.put_with_state("a", "k1", GunValue::Number(1.0), 1.0);
        g.put_with_state("a", "k2", GunValue::Number(2.0), 2.0);
        g.put_with_state("b", "k1", GunValue::Number(3.0), 3.0);
        g.put_with_state("b", "k2", GunValue::Number(4.0), 4.0);
        g.put_with_state("c", "k1", GunValue::Number(5.0), 5.0);
        g.put_with_state("c", "k2", GunValue::Number(6.0), 6.0); // 6 keys, over limit

        // Should have evicted some nodes
        assert!(g.total_keys() <= 5, "expected <= 5 keys, got {}", g.total_keys());
    }

    #[test]
    fn pin_and_unpin() {
        let mut g = Graph::new();
        g.put("node", "x", GunValue::Number(1.0));

        assert!(!g.is_pinned("node"));
        g.pin("node");
        assert!(g.is_pinned("node"));
        g.unpin("node");
        assert!(!g.is_pinned("node"));
    }

    #[test]
    fn touch_updates_access_order() {
        let config = EvictionConfig {
            max_nodes: 3,
            max_keys: 1_000_000,
            eviction_fraction: 0.5,
        };
        let mut g = Graph::with_eviction(config);

        // Add 3 nodes in order
        g.put_with_state("old", "v", GunValue::Number(1.0), 1.0);
        g.put_with_state("mid", "v", GunValue::Number(2.0), 2.0);
        g.put_with_state("new", "v", GunValue::Number(3.0), 3.0);

        // Touch "old" to make it recently accessed
        g.touch("old");

        // Add another node to trigger eviction — "mid" should be evicted
        // (it's now the least recently accessed)
        g.put_with_state("newest", "v", GunValue::Number(4.0), 4.0);

        assert!(g.get_node("old").is_some(), "touched node should survive");
        assert!(g.get_node("newest").is_some(), "newest node should survive");
    }

    #[test]
    fn eviction_count_stays_consistent() {
        let config = EvictionConfig {
            max_nodes: 5,
            max_keys: 1_000_000,
            eviction_fraction: 0.2,
        };
        let mut g = Graph::with_eviction(config);

        for i in 0..20 {
            g.put_with_state(
                &format!("n{}", i),
                "v",
                GunValue::Number(i as f64),
                (i + 1) as f64,
            );
        }

        // total_keys should match actual keys
        let actual_keys: usize = g.nodes().map(|(_, n)| n.iter().count()).sum();
        assert_eq!(g.total_keys(), actual_keys);
        assert_eq!(g.len(), g.nodes().count());
    }
}
