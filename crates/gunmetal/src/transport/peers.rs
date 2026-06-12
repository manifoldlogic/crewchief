//! Peer registry — discovery, health tracking, and connection state.
//!
//! Manages the set of known peers, their connection state, and health metrics.
//! Used by both native and WASM WebSocket transports.

use std::collections::HashMap;
use std::time::Duration;

use crate::state::now_ms;

/// Connection state for a peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerState {
    /// Known but not connected.
    Disconnected,
    /// Connection in progress.
    Connecting,
    /// Actively connected and communicating.
    Connected,
    /// Connection failed, waiting for reconnect.
    Backoff,
}

/// Health metrics for a single peer.
#[derive(Debug, Clone)]
pub struct PeerHealth {
    /// Total messages sent to this peer.
    pub messages_sent: u64,
    /// Total messages received from this peer.
    pub messages_received: u64,
    /// Timestamp of last successful message (ms since epoch).
    pub last_seen: f64,
    /// Number of consecutive connection failures.
    pub consecutive_failures: u32,
    /// Average round-trip time in ms (if measured).
    pub avg_rtt_ms: Option<f64>,
}

impl Default for PeerHealth {
    fn default() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            last_seen: 0.0,
            consecutive_failures: 0,
            avg_rtt_ms: None,
        }
    }
}

/// Information about a known peer.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Unique identifier for this peer.
    pub id: String,
    /// WebSocket URL for this peer.
    pub url: String,
    /// Current connection state.
    pub state: PeerState,
    /// Health metrics.
    pub health: PeerHealth,
    /// When this peer was first discovered (ms since epoch).
    pub discovered_at: f64,
}

impl PeerInfo {
    /// Create a new peer info entry.
    pub fn new(id: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            url: url.into(),
            state: PeerState::Disconnected,
            health: PeerHealth::default(),
            discovered_at: now_ms(),
        }
    }

    /// Whether this peer is currently connected.
    pub fn is_connected(&self) -> bool {
        self.state == PeerState::Connected
    }

    /// Record a successful message send.
    pub fn record_sent(&mut self) {
        self.health.messages_sent += 1;
    }

    /// Record a received message.
    pub fn record_received(&mut self) {
        self.health.messages_received += 1;
        self.health.last_seen = now_ms();
        self.health.consecutive_failures = 0;
    }

    /// Record a connection failure.
    pub fn record_failure(&mut self) {
        self.health.consecutive_failures += 1;
        self.state = PeerState::Backoff;
    }

    /// Record a successful connection.
    pub fn record_connected(&mut self) {
        self.state = PeerState::Connected;
        self.health.consecutive_failures = 0;
        self.health.last_seen = now_ms();
    }

    /// Check if the peer appears dead (no messages for a given duration).
    pub fn is_stale(&self, timeout: Duration) -> bool {
        if self.health.last_seen == 0.0 {
            return false; // never seen, not stale
        }
        let elapsed = now_ms() - self.health.last_seen;
        elapsed > timeout.as_millis() as f64
    }
}

/// Registry of known peers.
///
/// Tracks all peers the node has communicated with, their connection state,
/// and health metrics. Provides methods for peer discovery and selection.
pub struct PeerRegistry {
    peers: HashMap<String, PeerInfo>,
}

impl PeerRegistry {
    /// Create a new empty peer registry.
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    /// Add or update a peer in the registry.
    ///
    /// If the peer already exists, returns false. Otherwise adds it and returns true.
    pub fn add_peer(&mut self, id: impl Into<String>, url: impl Into<String>) -> bool {
        let id = id.into();
        if self.peers.contains_key(&id) {
            return false;
        }
        let url = url.into();
        self.peers.insert(id.clone(), PeerInfo::new(id, url));
        true
    }

    /// Remove a peer from the registry.
    pub fn remove_peer(&mut self, id: &str) -> Option<PeerInfo> {
        self.peers.remove(id)
    }

    /// Get a peer by ID.
    pub fn get(&self, id: &str) -> Option<&PeerInfo> {
        self.peers.get(id)
    }

    /// Get a mutable reference to a peer by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut PeerInfo> {
        self.peers.get_mut(id)
    }

    /// Get all connected peer IDs.
    pub fn connected_peers(&self) -> Vec<&str> {
        self.peers
            .values()
            .filter(|p| p.is_connected())
            .map(|p| p.id.as_str())
            .collect()
    }

    /// Get all peer IDs regardless of state.
    pub fn all_peers(&self) -> Vec<&str> {
        self.peers.keys().map(|k| k.as_str()).collect()
    }

    /// Number of connected peers.
    pub fn connected_count(&self) -> usize {
        self.peers.values().filter(|p| p.is_connected()).count()
    }

    /// Total number of known peers.
    pub fn total_count(&self) -> usize {
        self.peers.len()
    }

    /// Get peers that are disconnected and ready for reconnection.
    pub fn disconnected_peers(&self) -> Vec<&PeerInfo> {
        self.peers
            .values()
            .filter(|p| matches!(p.state, PeerState::Disconnected | PeerState::Backoff))
            .collect()
    }

    /// Remove stale peers that haven't been seen in the given timeout.
    pub fn remove_stale(&mut self, timeout: Duration) -> Vec<PeerInfo> {
        let stale_ids: Vec<String> = self
            .peers
            .values()
            .filter(|p| p.is_stale(timeout))
            .map(|p| p.id.clone())
            .collect();

        stale_ids
            .into_iter()
            .filter_map(|id| self.peers.remove(&id))
            .collect()
    }

    /// Iterate over all peers.
    pub fn iter(&self) -> impl Iterator<Item = &PeerInfo> {
        self.peers.values()
    }
}

impl Default for PeerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_get_peer() {
        let mut reg = PeerRegistry::new();
        assert!(reg.add_peer("p1", "ws://localhost:8080"));

        let peer = reg.get("p1").unwrap();
        assert_eq!(peer.id, "p1");
        assert_eq!(peer.url, "ws://localhost:8080");
        assert_eq!(peer.state, PeerState::Disconnected);
    }

    #[test]
    fn add_duplicate_returns_false() {
        let mut reg = PeerRegistry::new();
        assert!(reg.add_peer("p1", "ws://a"));
        assert!(!reg.add_peer("p1", "ws://b")); // duplicate
        assert_eq!(reg.total_count(), 1);
    }

    #[test]
    fn remove_peer() {
        let mut reg = PeerRegistry::new();
        reg.add_peer("p1", "ws://a");
        let removed = reg.remove_peer("p1");
        assert!(removed.is_some());
        assert_eq!(reg.total_count(), 0);
    }

    #[test]
    fn connected_peers_filter() {
        let mut reg = PeerRegistry::new();
        reg.add_peer("p1", "ws://a");
        reg.add_peer("p2", "ws://b");
        reg.add_peer("p3", "ws://c");

        reg.get_mut("p1").unwrap().record_connected();
        reg.get_mut("p3").unwrap().record_connected();

        let connected = reg.connected_peers();
        assert_eq!(connected.len(), 2);
        assert!(connected.contains(&"p1"));
        assert!(connected.contains(&"p3"));
        assert_eq!(reg.connected_count(), 2);
    }

    #[test]
    fn peer_health_tracking() {
        let mut peer = PeerInfo::new("p1", "ws://localhost");

        peer.record_connected();
        assert!(peer.is_connected());
        assert_eq!(peer.health.consecutive_failures, 0);

        peer.record_sent();
        peer.record_sent();
        assert_eq!(peer.health.messages_sent, 2);

        peer.record_received();
        assert_eq!(peer.health.messages_received, 1);
        assert!(peer.health.last_seen > 0.0);
    }

    #[test]
    fn peer_failure_tracking() {
        let mut peer = PeerInfo::new("p1", "ws://localhost");

        peer.record_failure();
        assert_eq!(peer.health.consecutive_failures, 1);
        assert_eq!(peer.state, PeerState::Backoff);

        peer.record_failure();
        assert_eq!(peer.health.consecutive_failures, 2);

        peer.record_connected();
        assert_eq!(peer.health.consecutive_failures, 0);
        assert!(peer.is_connected());
    }

    #[test]
    fn disconnected_peers_list() {
        let mut reg = PeerRegistry::new();
        reg.add_peer("p1", "ws://a");
        reg.add_peer("p2", "ws://b");
        reg.add_peer("p3", "ws://c");

        reg.get_mut("p2").unwrap().record_connected();
        reg.get_mut("p3").unwrap().record_failure();

        let disconnected = reg.disconnected_peers();
        // p1 = Disconnected, p3 = Backoff — both should be in the list
        assert_eq!(disconnected.len(), 2);
    }

    #[test]
    fn peer_info_new_defaults() {
        let peer = PeerInfo::new("test", "ws://localhost:8080");
        assert_eq!(peer.state, PeerState::Disconnected);
        assert_eq!(peer.health.messages_sent, 0);
        assert_eq!(peer.health.messages_received, 0);
        assert_eq!(peer.health.consecutive_failures, 0);
        assert!(peer.discovered_at > 0.0);
    }
}
