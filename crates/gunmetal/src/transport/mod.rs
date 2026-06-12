//! Transport — networking layer for GUN peer communication.
//!
//! Provides WebSocket-based transport for real-time peer-to-peer sync.
//! Two implementations behind the `AsyncSyncAdapter` trait:
//!
//! - **Native:** `tokio-tungstenite` — client+server, TLS support
//! - **WASM:** `web_sys::WebSocket` — event-driven browser WebSockets
//!
//! Shared logic:
//! - `reconnect` — exponential backoff with jitter (pure Rust)
//! - `peers` — peer registry, discovery, health tracking

pub mod peers;
pub mod reconnect;
#[cfg(not(target_arch = "wasm32"))]
pub mod ws_native;
#[cfg(target_arch = "wasm32")]
pub mod ws_wasm;

/// Trait for async sync transport adapters.
///
/// Unlike the sync `SyncAdapter` in `sync.rs`, this trait supports
/// async operations needed for real WebSocket connections.
pub trait AsyncSyncAdapter: Send + Sync {
    /// Send a message to a specific peer.
    fn send(
        &self,
        peer_id: &str,
        msg: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>>;

    /// Broadcast a message to all connected peers, optionally excluding one.
    fn broadcast(
        &self,
        msg: &str,
        exclude: Option<&str>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>>;

    /// Get a list of connected peer IDs.
    fn connected_peers(&self) -> Vec<String>;

    /// Check if a specific peer is connected.
    fn is_connected(&self, peer_id: &str) -> bool;
}
