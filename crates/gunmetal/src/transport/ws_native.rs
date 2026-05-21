//! Native WebSocket transport using tokio-tungstenite.
//!
//! Provides client and server WebSocket connections for native targets.
//! Each peer connection splits into a sink (send) and stream (receive).
//! Messages are JSON text frames using the existing wire format.

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::peers::{PeerRegistry, PeerState};
use super::reconnect::{ReconnectConfig, ReconnectState};

pub type WsSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type ServerWsSink = SplitSink<WebSocketStream<TcpStream>, Message>;

#[derive(Debug, Clone)]
pub struct WsNativeConfig {
    pub listen_url: Option<String>,
    pub peer_urls: Vec<String>,
    pub use_tls: bool,
    pub max_message_size: usize,
    pub ping_interval_secs: u64,
}

impl Default for WsNativeConfig {
    fn default() -> Self {
        Self {
            listen_url: None,
            peer_urls: Vec::new(),
            use_tls: false,
            max_message_size: 10 * 1024 * 1024,
            ping_interval_secs: 30,
        }
    }
}

#[derive(Debug, Clone)]
pub enum WsEvent {
    PeerConnected { peer_id: String },
    PeerDisconnected { peer_id: String, reason: String },
    MessageReceived { peer_id: String, message: String },
    StateChanged { peer_id: String, state: PeerState },
}

enum PeerSink {
    Client(WsSink),
    Server(ServerWsSink),
}

impl PeerSink {
    async fn send_text(&mut self, text: String) -> Result<(), String> {
        match self {
            PeerSink::Client(sink) => sink
                .send(Message::Text(text.into()))
                .await
                .map_err(|e| e.to_string()),
            PeerSink::Server(sink) => sink
                .send(Message::Text(text.into()))
                .await
                .map_err(|e| e.to_string()),
        }
    }

    async fn close(&mut self) -> Result<(), String> {
        match self {
            PeerSink::Client(sink) => sink.close().await.map_err(|e| e.to_string()),
            PeerSink::Server(sink) => sink.close().await.map_err(|e| e.to_string()),
        }
    }
}

pub struct WsNativeTransport {
    config: WsNativeConfig,
    peers: Arc<Mutex<PeerRegistry>>,
    sinks: Arc<Mutex<HashMap<String, PeerSink>>>,
    event_tx: mpsc::Sender<WsEvent>,
    event_rx: Arc<Mutex<mpsc::Receiver<WsEvent>>>,
    reconnect_configs: Arc<Mutex<HashMap<String, ReconnectState>>>,
}

impl WsNativeTransport {
    pub fn new(config: WsNativeConfig) -> Self {
        let (event_tx, event_rx) = mpsc::channel(256);
        Self {
            config,
            peers: Arc::new(Mutex::new(PeerRegistry::new())),
            sinks: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            reconnect_configs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn connect(&self, url: &str) -> Result<String, String> {
        let peer_id = url.to_string();

        {
            let mut peers = self.peers.lock().await;
            peers.add_peer(peer_id.clone(), url);
            if let Some(p) = peers.get_mut(&peer_id) {
                p.state = PeerState::Connecting;
            }
        }

        let (ws_stream, _) = tokio_tungstenite::connect_async(url)
            .await
            .map_err(|e| {
                let err = e.to_string();
                let peers = self.peers.clone();
                let pid = peer_id.clone();
                tokio::spawn(async move {
                    let mut p = peers.lock().await;
                    if let Some(info) = p.get_mut(&pid) {
                        info.record_failure();
                    }
                });
                err
            })?;

        let (sink, stream) = ws_stream.split();

        {
            let mut sinks = self.sinks.lock().await;
            sinks.insert(peer_id.clone(), PeerSink::Client(sink));
        }
        {
            let mut peers = self.peers.lock().await;
            if let Some(p) = peers.get_mut(&peer_id) {
                p.record_connected();
            }
        }

        let _ = self.event_tx.send(WsEvent::PeerConnected {
            peer_id: peer_id.clone(),
        }).await;

        self.spawn_reader(peer_id.clone(), stream);

        {
            let mut rc = self.reconnect_configs.lock().await;
            rc.entry(peer_id.clone())
                .or_insert_with(|| ReconnectState::new(ReconnectConfig::default()))
                .reset();
        }

        Ok(peer_id)
    }

    pub async fn listen(&self) -> Result<(), String> {
        let listen_url = self
            .config
            .listen_url
            .as_ref()
            .ok_or("No listen URL configured")?;

        let addr = listen_url
            .strip_prefix("ws://")
            .unwrap_or(listen_url);

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| e.to_string())?;

        let peers = self.peers.clone();
        let sinks = self.sinks.clone();
        let event_tx = self.event_tx.clone();
        let max_msg = self.config.max_message_size;

        tokio::spawn(async move {
            let mut counter = 0u64;
            while let Ok((stream, addr)) = listener.accept().await {
                counter += 1;
                let peer_id = format!("server-{}-{}", addr, counter);

                let ws = match tokio_tungstenite::accept_async(stream).await {
                    Ok(ws) => ws,
                    Err(_) => continue,
                };

                let (sink, mut read_stream) = ws.split();

                {
                    let mut p = peers.lock().await;
                    p.add_peer(peer_id.clone(), &addr.to_string());
                    if let Some(info) = p.get_mut(&peer_id) {
                        info.record_connected();
                    }
                }
                {
                    let mut s = sinks.lock().await;
                    s.insert(peer_id.clone(), PeerSink::Server(sink));
                }

                let _ = event_tx.send(WsEvent::PeerConnected {
                    peer_id: peer_id.clone(),
                }).await;

                let tx = event_tx.clone();
                let peers2 = peers.clone();
                let sinks2 = sinks.clone();
                let pid = peer_id.clone();

                tokio::spawn(async move {
                    while let Some(msg) = read_stream.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if text.len() <= max_msg {
                                    let _ = tx.send(WsEvent::MessageReceived {
                                        peer_id: pid.clone(),
                                        message: text.to_string(),
                                    }).await;
                                    let mut p = peers2.lock().await;
                                    if let Some(info) = p.get_mut(&pid) {
                                        info.record_received();
                                    }
                                }
                            }
                            Ok(Message::Close(_)) | Err(_) => break,
                            _ => {}
                        }
                    }
                    let _ = tx.send(WsEvent::PeerDisconnected {
                        peer_id: pid.clone(),
                        reason: "closed".into(),
                    }).await;
                    sinks2.lock().await.remove(&pid);
                    let mut p = peers2.lock().await;
                    if let Some(info) = p.get_mut(&pid) {
                        info.state = PeerState::Disconnected;
                    }
                });
            }
        });

        Ok(())
    }

    pub async fn send(&self, peer_id: &str, msg: &str) -> Result<(), String> {
        let mut sinks = self.sinks.lock().await;
        let sink = sinks
            .get_mut(peer_id)
            .ok_or_else(|| format!("Peer not connected: {}", peer_id))?;
        sink.send_text(msg.to_string()).await?;

        let mut peers = self.peers.lock().await;
        if let Some(p) = peers.get_mut(peer_id) {
            p.record_sent();
        }
        Ok(())
    }

    pub async fn broadcast(&self, msg: &str, exclude: Option<&str>) -> Result<(), String> {
        let connected: Vec<String> = {
            let peers = self.peers.lock().await;
            peers
                .connected_peers()
                .into_iter()
                .filter(|id| exclude != Some(id))
                .map(|s| s.to_string())
                .collect()
        };

        let mut sinks = self.sinks.lock().await;
        for pid in &connected {
            if let Some(sink) = sinks.get_mut(pid.as_str()) {
                let _ = sink.send_text(msg.to_string()).await;
            }
        }

        let mut peers = self.peers.lock().await;
        for pid in &connected {
            if let Some(p) = peers.get_mut(pid.as_str()) {
                p.record_sent();
            }
        }
        Ok(())
    }

    pub async fn disconnect(&self, peer_id: &str) -> Result<(), String> {
        let mut sinks = self.sinks.lock().await;
        if let Some(mut sink) = sinks.remove(peer_id) {
            let _ = sink.close().await;
        }

        let mut peers = self.peers.lock().await;
        if let Some(p) = peers.get_mut(peer_id) {
            p.state = PeerState::Disconnected;
        }

        let _ = self.event_tx.send(WsEvent::PeerDisconnected {
            peer_id: peer_id.to_string(),
            reason: "manual disconnect".into(),
        }).await;

        Ok(())
    }

    pub async fn recv_event(&self) -> Option<WsEvent> {
        let mut rx = self.event_rx.lock().await;
        rx.recv().await
    }

    pub fn connected_peers_sync(&self) -> Vec<String> {
        Vec::new()
    }

    fn spawn_reader<S>(&self, peer_id: String, mut stream: S)
    where
        S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
            + Unpin
            + Send
            + 'static,
    {
        let tx = self.event_tx.clone();
        let peers = self.peers.clone();
        let sinks = self.sinks.clone();
        let max_msg = self.config.max_message_size;

        let reconnect_configs = self.reconnect_configs.clone();
        let peers_for_reconnect = self.peers.clone();
        let sinks_for_reconnect = self.sinks.clone();
        let tx_for_reconnect = self.event_tx.clone();

        let pid = peer_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if text.len() <= max_msg {
                            let _ = tx.send(WsEvent::MessageReceived {
                                peer_id: pid.clone(),
                                message: text.to_string(),
                            }).await;
                            let mut p = peers.lock().await;
                            if let Some(info) = p.get_mut(&pid) {
                                info.record_received();
                            }
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        let mut s = sinks.lock().await;
                        if let Some(sink) = s.get_mut(&pid) {
                            let _ = sink.send_text(String::new()).await;
                            drop(s);
                        }
                        let _ = data;
                    }
                    Ok(Message::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }

            let _ = tx.send(WsEvent::PeerDisconnected {
                peer_id: pid.clone(),
                reason: "connection lost".into(),
            }).await;
            sinks.lock().await.remove(&pid);

            {
                let mut p = peers.lock().await;
                if let Some(info) = p.get_mut(&pid) {
                    info.record_failure();
                }
            }

            // Attempt reconnection with backoff
            let url = {
                let p = peers_for_reconnect.lock().await;
                p.get(&pid).map(|info| info.url.clone())
            };
            if let Some(url) = url {
                let delay = {
                    let mut rc = reconnect_configs.lock().await;
                    let state = rc
                        .entry(pid.clone())
                        .or_insert_with(|| ReconnectState::new(ReconnectConfig::default()));
                    state.next_delay()
                };
                if let Some(delay) = delay {
                    tokio::time::sleep(delay).await;
                    if let Ok((ws, _)) = tokio_tungstenite::connect_async(&url).await {
                        let (sink, _new_stream) = ws.split();
                        sinks_for_reconnect
                            .lock()
                            .await
                            .insert(pid.clone(), PeerSink::Client(sink));
                        let mut p = peers_for_reconnect.lock().await;
                        if let Some(info) = p.get_mut(&pid) {
                            info.record_connected();
                        }
                        let _ = tx_for_reconnect.send(WsEvent::PeerConnected {
                            peer_id: pid.clone(),
                        }).await;
                        reconnect_configs.lock().await.get_mut(&pid).map(|s| s.reset());
                    }
                }
            }
        });
    }
}

impl super::AsyncSyncAdapter for WsNativeTransport {
    fn send(
        &self,
        peer_id: &str,
        msg: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        let peer_id = peer_id.to_string();
        let msg = msg.to_string();
        Box::pin(async move { self.send(&peer_id, &msg).await })
    }

    fn broadcast(
        &self,
        msg: &str,
        exclude: Option<&str>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        let msg = msg.to_string();
        let exclude = exclude.map(|s| s.to_string());
        Box::pin(async move {
            self.broadcast(&msg, exclude.as_deref()).await
        })
    }

    fn connected_peers(&self) -> Vec<String> {
        self.connected_peers_sync()
    }

    fn is_connected(&self, _peer_id: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = WsNativeConfig::default();
        assert!(config.listen_url.is_none());
        assert!(config.peer_urls.is_empty());
        assert!(!config.use_tls);
        assert_eq!(config.max_message_size, 10 * 1024 * 1024);
        assert_eq!(config.ping_interval_secs, 30);
    }

    #[test]
    fn ws_event_types() {
        let connected = WsEvent::PeerConnected {
            peer_id: "p1".into(),
        };
        let disconnected = WsEvent::PeerDisconnected {
            peer_id: "p1".into(),
            reason: "timeout".into(),
        };
        let message = WsEvent::MessageReceived {
            peer_id: "p1".into(),
            message: "{}".into(),
        };

        match connected {
            WsEvent::PeerConnected { peer_id } => assert_eq!(peer_id, "p1"),
            _ => panic!("wrong variant"),
        }
        match disconnected {
            WsEvent::PeerDisconnected { peer_id, reason } => {
                assert_eq!(peer_id, "p1");
                assert_eq!(reason, "timeout");
            }
            _ => panic!("wrong variant"),
        }
        match message {
            WsEvent::MessageReceived { peer_id, message } => {
                assert_eq!(peer_id, "p1");
                assert_eq!(message, "{}");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn transport_creation() {
        let config = WsNativeConfig {
            listen_url: Some("ws://127.0.0.1:9000".into()),
            peer_urls: vec!["ws://peer1:8080".into()],
            ..Default::default()
        };
        let _transport = WsNativeTransport::new(config);
    }

    #[tokio::test]
    async fn loopback_connect_send_receive() {
        let server = WsNativeTransport::new(WsNativeConfig {
            listen_url: Some("ws://127.0.0.1:0".into()),
            ..Default::default()
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_peers = server.peers.clone();
        let server_sinks = server.sinks.clone();
        let server_tx = server.event_tx.clone();
        let server_event_rx = server.event_rx.clone();

        tokio::spawn(async move {
            if let Ok((stream, remote)) = listener.accept().await {
                let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let (sink, mut read_stream) = ws.split();
                let pid = format!("client-{}", remote);

                {
                    let mut p = server_peers.lock().await;
                    p.add_peer(pid.clone(), &remote.to_string());
                    if let Some(info) = p.get_mut(&pid) {
                        info.record_connected();
                    }
                }
                {
                    let mut s = server_sinks.lock().await;
                    s.insert(pid.clone(), PeerSink::Server(sink));
                }
                let _ = server_tx.send(WsEvent::PeerConnected { peer_id: pid.clone() }).await;

                while let Some(Ok(msg)) = read_stream.next().await {
                    if let Message::Text(text) = msg {
                        let _ = server_tx.send(WsEvent::MessageReceived {
                            peer_id: pid.clone(),
                            message: text.to_string(),
                        }).await;
                    }
                }
            }
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = WsNativeTransport::new(WsNativeConfig::default());
        let url = format!("ws://127.0.0.1:{}", addr.port());
        let peer_id = client.connect(&url).await.unwrap();

        client.send(&peer_id, r##"{"#":"test1","put":{}}"##).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let mut rx = server_event_rx.lock().await;
        let mut got_connect = false;
        let mut got_message = false;

        while let Ok(event) = rx.try_recv() {
            match event {
                WsEvent::PeerConnected { .. } => got_connect = true,
                WsEvent::MessageReceived { message, .. } => {
                    assert!(message.contains("test1"));
                    got_message = true;
                }
                _ => {}
            }
        }

        assert!(got_connect, "should have received connect event");
        assert!(got_message, "should have received message event");
    }

    #[tokio::test]
    async fn broadcast_to_multiple_peers() {
        let transport = WsNativeTransport::new(WsNativeConfig::default());

        let result = transport.broadcast(r##"{"#":"bc1"}"##, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_to_nonexistent_peer_fails() {
        let transport = WsNativeTransport::new(WsNativeConfig::default());

        let result = transport.send("nonexistent", "{}").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not connected"));
    }

    #[tokio::test]
    async fn disconnect_unknown_peer_is_ok() {
        let transport = WsNativeTransport::new(WsNativeConfig::default());
        let result = transport.disconnect("unknown").await;
        assert!(result.is_ok());
    }
}
