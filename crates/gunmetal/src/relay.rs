//! Relay server — a standalone WebSocket relay wire-compatible with GUN.js.
//!
//! Equivalent to running `Gun({web: server})` on Node.js: browser clients
//! configured with `Gun({peers: ['ws://host:8765/gun']})` can connect,
//! complete the DAM `?` handshake, store and read data, and receive
//! relayed PUTs from other peers.
//!
//! # Architecture
//!
//! ```text
//! TcpListener ──► HTTP head parse ──┬─► /gun       WebSocket upgrade ─► Mesh
//!                                   ├─► /health    200 JSON status
//!                                   └─► other      404
//! ```
//!
//! Each WebSocket connection becomes a mesh peer: inbound frames go to
//! [`Mesh::hear_async`], outbound frames flow through an mpsc channel
//! pushed by the peer's `PeerSender`. Heartbeats (`[]`) are sent every
//! [`HEARTBEAT_INTERVAL`](crate::mesh::HEARTBEAT_INTERVAL). Data is
//! persisted through a [`StorageEngine`] with a pluggable
//! [`StorageAdapter`] and hydrated at startup.
//!
//! # Configuration
//!
//! CLI flags override environment variables, which override defaults:
//!
//! | Option | Flag | Env | Default |
//! |--------|------|-----|---------|
//! | port | `--port` | `PORT` | `8765` |
//! | host | `--host` | `HOST` | `0.0.0.0` |
//! | path | `--path` | `GUN_PATH` | `/gun` |
//! | file | `--file` | `GUN_FILE` | `radata` |
//! | TLS cert | `--tls-cert` | `TLS_CERT` | none |
//! | TLS key | `--tls-key` | `TLS_KEY` | none |
//! | max peers | `--mob` | `MOB` | `999999` |
//! | upstream peers | `--peer` (repeatable) | `GUN_PEERS` (comma-sep) | none |
//!
//! TLS requires the `relay-tls` feature (rustls).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::instance::{Gun, GunOptions};
use crate::mesh::{Mesh, MeshConfig, PeerSender, HEARTBEAT_INTERVAL, HEARTBEAT_RAW};
use crate::storage::{MemoryStorage, StorageAdapter, StorageEngine};
use crate::uuid::random_message_id;
use crate::wire::{self, WireMessage};

/// Maximum size of an HTTP request head we are willing to parse.
const MAX_HTTP_HEAD_BYTES: usize = 8 * 1024;

/// Maximum number of header lines in a request head.
const MAX_HTTP_HEAD_LINES: usize = 100;

/// Timeout for reading the HTTP request head.
const HTTP_HEAD_TIMEOUT: Duration = Duration::from_secs(10);

/// Relay configuration. See the module docs for the option table.
#[derive(Debug, Clone)]
pub struct RelayConfig {
    /// Listen port. Default 8765. Use 0 for an ephemeral port (tests).
    pub port: u16,
    /// Listen host. Default `0.0.0.0`.
    pub host: String,
    /// WebSocket upgrade path. Default `/gun`.
    pub path: String,
    /// Storage directory (used by filesystem-backed adapters). Default `radata`.
    pub file: String,
    /// Health check endpoint. Default `/health`.
    pub health_path: String,
    /// TLS certificate path (PEM). Requires the `relay-tls` feature.
    pub tls_cert: Option<String>,
    /// TLS private key path (PEM). Requires the `relay-tls` feature.
    pub tls_key: Option<String>,
    /// Mob threshold: above this many connected peers, new peers are
    /// redirected (`{dam:'mob'}`) and disconnected. Default 999999.
    pub max_peers: usize,
    /// Upstream relay URLs to dial and join (the AXE "up" set).
    pub peers: Vec<String>,
    /// Mesh tuning (pid, gap, puff, …).
    pub mesh: MeshConfig,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            port: 8765,
            host: "0.0.0.0".into(),
            path: "/gun".into(),
            file: "radata".into(),
            health_path: "/health".into(),
            tls_cert: None,
            tls_key: None,
            max_peers: 999_999,
            peers: Vec::new(),
            mesh: MeshConfig::default(),
        }
    }
}

impl RelayConfig {
    /// Apply environment variables (medium priority).
    pub fn apply_env(mut self) -> Self {
        if let Ok(port) = std::env::var("PORT") {
            if let Ok(port) = port.parse() {
                self.port = port;
            }
        }
        if let Ok(host) = std::env::var("HOST") {
            self.host = host;
        }
        if let Ok(path) = std::env::var("GUN_PATH") {
            self.path = path;
        }
        if let Ok(file) = std::env::var("GUN_FILE") {
            self.file = file;
        }
        if let Ok(cert) = std::env::var("TLS_CERT") {
            self.tls_cert = Some(cert);
        }
        if let Ok(key) = std::env::var("TLS_KEY") {
            self.tls_key = Some(key);
        }
        if let Ok(mob) = std::env::var("MOB") {
            if let Ok(mob) = mob.parse() {
                self.max_peers = mob;
            }
        }
        if let Ok(peers) = std::env::var("GUN_PEERS") {
            self.peers = peers
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        }
        self
    }

    /// Apply CLI flags (highest priority). Unknown flags are an error.
    pub fn apply_args<I: IntoIterator<Item = String>>(mut self, args: I) -> Result<Self, String> {
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            let mut take = |name: &str| -> Result<String, String> {
                iter.next().ok_or_else(|| format!("{} requires a value", name))
            };
            match arg.as_str() {
                "--port" => {
                    self.port = take("--port")?
                        .parse()
                        .map_err(|e| format!("invalid --port: {}", e))?;
                }
                "--host" => self.host = take("--host")?,
                "--path" => self.path = take("--path")?,
                "--file" => self.file = take("--file")?,
                "--health-path" => self.health_path = take("--health-path")?,
                "--tls-cert" => self.tls_cert = Some(take("--tls-cert")?),
                "--tls-key" => self.tls_key = Some(take("--tls-key")?),
                "--mob" | "--max-peers" => {
                    self.max_peers = take(&arg)?
                        .parse()
                        .map_err(|e| format!("invalid {}: {}", arg, e))?;
                }
                "--peer" => self.peers.push(take("--peer")?),
                other => return Err(format!("unknown flag: {}", other)),
            }
        }
        Ok(self)
    }
}

/// Shared per-relay context handed to every connection task.
#[derive(Clone)]
struct RelayCtx {
    config: Arc<RelayConfig>,
    mesh: Mesh,
}

/// Handle to a running relay.
pub struct RelayHandle {
    /// The actual bound address (resolves ephemeral ports).
    pub addr: SocketAddr,
    mesh: Mesh,
    gun: Gun,
    shutdown: watch::Sender<bool>,
    /// Keeps the put-listener persisting writes for the relay's lifetime.
    _storage: StorageEngine,
}

impl RelayHandle {
    /// The mesh routing this relay's traffic.
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    /// The relay's Gun instance (graph access).
    pub fn gun(&self) -> &Gun {
        &self.gun
    }

    /// Stop accepting connections.
    pub fn shutdown(&self) {
        let _ = self.shutdown.send(true);
    }
}

/// Start a relay with the default storage backend: RAD chunked storage
/// on the filesystem under `config.file` (GUN's `radata` layout).
pub async fn spawn(config: RelayConfig) -> Result<RelayHandle, String> {
    let store = crate::rad::fs_store::FsStore::new(&config.file)?;
    let adapter = crate::rad::RadStorageAdapter::with_store(Box::new(store));
    spawn_with_storage(config, Box::new(adapter)).await
}

/// Start a relay with in-memory storage (no persistence). Useful for
/// tests and ephemeral relays.
pub async fn spawn_in_memory(config: RelayConfig) -> Result<RelayHandle, String> {
    spawn_with_storage(config, Box::new(MemoryStorage::new())).await
}

/// Start a relay persisting through the given storage adapter.
pub async fn spawn_with_storage(
    config: RelayConfig,
    adapter: Box<dyn StorageAdapter>,
) -> Result<RelayHandle, String> {
    let gun = Gun::new(GunOptions {
        file: Some(config.file.clone()),
        ..Default::default()
    });
    let mesh = Mesh::new(gun.clone(), config.mesh.clone());

    // Hydrate the graph from storage, then persist subsequent writes.
    let storage = StorageEngine::new(gun.clone(), adapter);
    storage.load_all();

    let listener = TcpListener::bind((config.host.as_str(), config.port))
        .await
        .map_err(|e| format!("bind {}:{}: {}", config.host, config.port, e))?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let ctx = RelayCtx {
        config: Arc::new(config.clone()),
        mesh: mesh.clone(),
    };

    #[cfg(feature = "relay-tls")]
    let tls_acceptor = tls::acceptor(&config)?;
    #[cfg(not(feature = "relay-tls"))]
    if config.tls_cert.is_some() || config.tls_key.is_some() {
        return Err("TLS requested but gunmetal was built without the relay-tls feature".into());
    }

    // Dial upstream relays (the "up" set).
    for url in &config.peers {
        tokio::spawn(dial_upstream(ctx.clone(), url.clone()));
    }

    let accept_ctx = ctx.clone();
    let mut accept_shutdown = shutdown_rx.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = accept_shutdown.changed() => break,
                accepted = listener.accept() => {
                    let Ok((stream, peer_addr)) = accepted else { continue };
                    let conn_ctx = accept_ctx.clone();
                    #[cfg(feature = "relay-tls")]
                    let acceptor = tls_acceptor.clone();
                    tokio::spawn(async move {
                        #[cfg(feature = "relay-tls")]
                        {
                            if let Some(acceptor) = acceptor {
                                if let Ok(tls_stream) = acceptor.accept(stream).await {
                                    handle_connection(tls_stream, peer_addr, conn_ctx).await;
                                }
                                return;
                            }
                            handle_connection(stream, peer_addr, conn_ctx).await;
                        }
                        #[cfg(not(feature = "relay-tls"))]
                        handle_connection(stream, peer_addr, conn_ctx).await;
                    });
                }
            }
        }
    });

    Ok(RelayHandle {
        addr,
        mesh,
        gun,
        shutdown: shutdown_tx,
        _storage: storage,
    })
}

/// Run a relay until Ctrl-C.
pub async fn run(config: RelayConfig) -> Result<(), String> {
    let handle = spawn(config).await?;
    eprintln!("gunmetal-relay listening on {}", handle.addr);
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| format!("signal handler: {}", e))?;
    handle.shutdown();
    Ok(())
}

// ── HTTP handling ───────────────────────────────────────────────────

/// A parsed HTTP request head.
struct RequestHead {
    method: String,
    path: String,
    headers: HashMap<String, String>,
}

impl RequestHead {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_ascii_lowercase()).map(|s| s.as_str())
    }

    fn wants_websocket(&self) -> bool {
        self.header("upgrade")
            .is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
            && self
                .header("connection")
                .is_some_and(|v| v.to_ascii_lowercase().contains("upgrade"))
    }
}

/// Read and parse the HTTP request head from a buffered stream, leaving
/// the body/frames buffered for the WebSocket layer.
async fn read_head<S>(reader: &mut BufReader<S>) -> Result<RequestHead, String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let head = tokio::time::timeout(HTTP_HEAD_TIMEOUT, async {
        let mut request_line = String::new();
        reader
            .read_line(&mut request_line)
            .await
            .map_err(|e| e.to_string())?;

        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or_default().to_string();
        let path = parts.next().unwrap_or_default().to_string();
        if method.is_empty() || path.is_empty() {
            return Err("malformed request line".to_string());
        }

        let mut headers = HashMap::new();
        let mut total = request_line.len();
        for _ in 0..MAX_HTTP_HEAD_LINES {
            let mut line = String::new();
            reader.read_line(&mut line).await.map_err(|e| e.to_string())?;
            total += line.len();
            if total > MAX_HTTP_HEAD_BYTES {
                return Err("request head too large".to_string());
            }
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                return Ok(RequestHead {
                    method,
                    path,
                    headers,
                });
            }
            if let Some((name, value)) = trimmed.split_once(':') {
                headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
            }
        }
        Err("too many header lines".to_string())
    })
    .await
    .map_err(|_| "timed out reading request head".to_string())??;
    Ok(head)
}

async fn write_response<S>(stream: &mut S, status: &str, body: &str) -> Result<(), String>
where
    S: AsyncWrite + Unpin,
{
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())
}

async fn handle_connection<S>(stream: S, peer_addr: SocketAddr, ctx: RelayCtx)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut reader = BufReader::new(stream);
    let Ok(head) = read_head(&mut reader).await else {
        return;
    };

    let path_only = head.path.split('?').next().unwrap_or("");

    // Health check endpoint.
    if path_only == ctx.config.health_path {
        let body = format!(
            r#"{{"ok":true,"peers":{},"pid":"{}"}}"#,
            ctx.mesh.near(),
            ctx.mesh.pid()
        );
        let _ = write_response(&mut reader, "200 OK", &body).await;
        return;
    }

    // WebSocket upgrade on the configured path.
    if path_only == ctx.config.path {
        if head.method != "GET" || !head.wants_websocket() {
            let _ = write_response(
                &mut reader,
                "400 Bad Request",
                r#"{"err":"expected websocket upgrade"}"#,
            )
            .await;
            return;
        }
        let Some(key) = head.header("sec-websocket-key") else {
            let _ = write_response(
                &mut reader,
                "400 Bad Request",
                r#"{"err":"missing sec-websocket-key"}"#,
            )
            .await;
            return;
        };

        let accept = derive_accept_key(key.as_bytes());
        let response = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
            accept
        );
        if reader.write_all(response.as_bytes()).await.is_err() {
            return;
        }
        if reader.flush().await.is_err() {
            return;
        }

        let ws = WebSocketStream::from_raw_socket(reader, Role::Server, None).await;
        serve_ws(ws, peer_addr, ctx).await;
        return;
    }

    let _ = write_response(&mut reader, "404 Not Found", r#"{"err":"not found"}"#).await;
}

// ── WebSocket peer loop ─────────────────────────────────────────────

async fn serve_ws<S>(ws: WebSocketStream<S>, peer_addr: SocketAddr, ctx: RelayCtx)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (mut sink, mut source) = ws.split();

    // Mob rebalancing (AXE): shed the connection when over capacity,
    // pointing the client at our upstream peers.
    if ctx.mesh.near() >= ctx.config.max_peers {
        let mut peers_obj = serde_json::Map::new();
        for url in &ctx.config.peers {
            peers_obj.insert(url.clone(), serde_json::Value::from(1));
        }
        let mob = WireMessage {
            id: Some(random_message_id(9)),
            dam: Some("mob".into()),
            mob: Some(serde_json::Value::from(ctx.mesh.near() as u64)),
            peers: Some(serde_json::Value::Object(peers_obj)),
            ..Default::default()
        };
        if let Ok(raw) = wire::serialize_message(&mob) {
            let _ = sink.send(Message::Text(raw.into())).await;
        }
        let _ = sink.close().await;
        return;
    }

    let peer_id = format!("{}#{}", peer_addr, random_message_id(4));

    // Outbound: mesh pushes frames into the channel; this task drains it
    // into the socket.
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let sender: PeerSender = Arc::new(move |raw: &str| {
        let _ = tx.send(raw.to_string());
    });
    ctx.mesh.hi(&peer_id, None, Some(sender));

    let mut heartbeat = tokio::time::interval_at(
        tokio::time::Instant::now() + HEARTBEAT_INTERVAL,
        HEARTBEAT_INTERVAL,
    );

    loop {
        tokio::select! {
            outbound = rx.recv() => {
                let Some(raw) = outbound else { break };
                if sink.send(Message::Text(raw.into())).await.is_err() {
                    break;
                }
            }
            _ = heartbeat.tick() => {
                if sink.send(Message::Text(HEARTBEAT_RAW.into())).await.is_err() {
                    break;
                }
            }
            inbound = source.next() => {
                match inbound {
                    Some(Ok(Message::Text(text))) => {
                        ctx.mesh.hear_async(text.as_str(), &peer_id).await;
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                            ctx.mesh.hear_async(&text, &peer_id).await;
                        }
                    }
                    Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                }
            }
        }
    }

    // Cleanup: apply bye() writes, drop the peer, emit the bye event.
    ctx.mesh.bye(&peer_id);
}

// ── Upstream relays ─────────────────────────────────────────────────

/// Dial an upstream relay and keep the connection in the mesh,
/// reconnecting with a capped backoff.
async fn dial_upstream(ctx: RelayCtx, url: String) {
    let mut backoff = Duration::from_secs(1);
    loop {
        match tokio_tungstenite::connect_async(&url).await {
            Ok((ws, _)) => {
                backoff = Duration::from_secs(1);
                let peer_id = format!("up:{}", url);
                let (mut sink, mut source) = ws.split();
                let (tx, mut rx) = mpsc::unbounded_channel::<String>();
                let sender: PeerSender = Arc::new(move |raw: &str| {
                    let _ = tx.send(raw.to_string());
                });
                ctx.mesh.hi(&peer_id, Some(url.clone()), Some(sender));

                loop {
                    tokio::select! {
                        outbound = rx.recv() => {
                            let Some(raw) = outbound else { break };
                            if sink.send(Message::Text(raw.into())).await.is_err() {
                                break;
                            }
                        }
                        inbound = source.next() => {
                            match inbound {
                                Some(Ok(Message::Text(text))) => {
                                    ctx.mesh.hear_async(text.as_str(), &peer_id).await;
                                }
                                Some(Ok(Message::Binary(bytes))) => {
                                    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                                        ctx.mesh.hear_async(&text, &peer_id).await;
                                    }
                                }
                                Some(Ok(_)) => {}
                                Some(Err(_)) | None => break,
                            }
                        }
                    }
                }
                ctx.mesh.bye(&peer_id);
            }
            Err(_) => {}
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(60));
    }
}

// ── TLS ─────────────────────────────────────────────────────────────

#[cfg(feature = "relay-tls")]
mod tls {
    use super::RelayConfig;
    use std::sync::Arc;
    use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use tokio_rustls::rustls::ServerConfig;
    use tokio_rustls::TlsAcceptor;

    /// Build a TLS acceptor from the configured cert/key PEM files.
    /// Returns `None` when TLS is not configured.
    pub(super) fn acceptor(config: &RelayConfig) -> Result<Option<TlsAcceptor>, String> {
        let (Some(cert_path), Some(key_path)) = (&config.tls_cert, &config.tls_key) else {
            if config.tls_cert.is_some() || config.tls_key.is_some() {
                return Err("both --tls-cert and --tls-key are required for TLS".into());
            }
            return Ok(None);
        };

        let certs: Vec<CertificateDer> = rustls_pemfile::certs(&mut std::io::BufReader::new(
            std::fs::File::open(cert_path).map_err(|e| format!("open {}: {}", cert_path, e))?,
        ))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("parse certs: {}", e))?;

        let key: PrivateKeyDer = rustls_pemfile::private_key(&mut std::io::BufReader::new(
            std::fs::File::open(key_path).map_err(|e| format!("open {}: {}", key_path, e))?,
        ))
        .map_err(|e| format!("parse key: {}", e))?
        .ok_or("no private key found")?;

        let server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| format!("tls config: {}", e))?;

        Ok(Some(TlsAcceptor::from(Arc::new(server_config))))
    }
}

// ── tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::GunValue;
    use tokio::net::TcpStream;

    fn test_config() -> RelayConfig {
        RelayConfig {
            port: 0,
            host: "127.0.0.1".into(),
            ..Default::default()
        }
    }

    async fn ws_connect(
        addr: SocketAddr,
        path: &str,
    ) -> WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>> {
        let url = format!("ws://{}{}", addr, path);
        let (ws, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("connect");
        ws
    }

    /// Read frames until one matches, with a timeout.
    async fn wait_for_frame<S>(
        ws: &mut WebSocketStream<S>,
        pred: impl Fn(&str) -> bool,
    ) -> Option<String>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let remaining = deadline
                .checked_duration_since(tokio::time::Instant::now())
                .unwrap_or(Duration::ZERO);
            match tokio::time::timeout(remaining, ws.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    if pred(text.as_str()) {
                        return Some(text.to_string());
                    }
                }
                Ok(Some(Ok(_))) => {}
                _ => return None,
            }
        }
    }

    #[tokio::test]
    async fn config_priority_args_over_env_over_defaults() {
        let config = RelayConfig::default()
            .apply_args(vec!["--port".to_string(), "9000".to_string()])
            .unwrap();
        assert_eq!(config.port, 9000);
        assert_eq!(config.path, "/gun", "default preserved");
        assert_eq!(config.max_peers, 999_999);

        let err = RelayConfig::default().apply_args(vec!["--bogus".to_string()]);
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn health_endpoint_responds() {
        let handle = spawn_in_memory(test_config()).await.unwrap();

        let mut stream = TcpStream::connect(handle.addr).await.unwrap();
        stream
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut stream, &mut buf)
            .await
            .unwrap();
        let response = String::from_utf8_lossy(&buf);
        assert!(response.starts_with("HTTP/1.1 200 OK"), "{}", response);
        assert!(response.contains(r#""ok":true"#));

        handle.shutdown();
    }

    #[tokio::test]
    async fn unknown_path_404s() {
        let handle = spawn_in_memory(test_config()).await.unwrap();

        let mut stream = TcpStream::connect(handle.addr).await.unwrap();
        stream
            .write_all(b"GET /nope HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut stream, &mut buf)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&buf).starts_with("HTTP/1.1 404"));

        handle.shutdown();
    }

    #[tokio::test]
    async fn websocket_handshake_and_dam_exchange() {
        let handle = spawn_in_memory(test_config()).await.unwrap();
        let mut ws = ws_connect(handle.addr, "/gun").await;

        // The relay sends its DAM handshake on connect.
        let frame = wait_for_frame(&mut ws, |f| f.contains(r###""dam":"?""###))
            .await
            .expect("handshake from relay");
        let msg = wire::parse_message(&frame).unwrap();
        assert_eq!(msg.pid.as_deref(), Some(handle.mesh().pid().as_str()));

        handle.shutdown();
    }

    #[tokio::test]
    async fn put_stored_and_get_answered() {
        let handle = spawn_in_memory(test_config()).await.unwrap();

        // Client A writes.
        let mut ws_a = ws_connect(handle.addr, "/gun").await;
        ws_a.send(Message::Text(
            r###"{"#":"w1","put":{"test":{"_":{"#":"test",">":{"name":1700000000000}},"name":"Alice"}}}"###.into(),
        ))
        .await
        .unwrap();

        // PUT is acknowledged.
        let ack = wait_for_frame(&mut ws_a, |f| f.contains(r###""@":"w1""###))
            .await
            .expect("put ack");
        assert!(ack.contains(r###""ok""###));

        assert_eq!(
            handle.gun().get("test").get("name").val(),
            Some(GunValue::Text("Alice".into())),
            "relay stored the data"
        );

        // Client B reads.
        let mut ws_b = ws_connect(handle.addr, "/gun").await;
        ws_b.send(Message::Text(r###"{"#":"r1","get":{"#":"test"}}"###.into()))
            .await
            .unwrap();
        let answer = wait_for_frame(&mut ws_b, |f| {
            f.contains(r###""@":"r1""###) && f.contains("Alice")
        })
        .await
        .expect("get answered");
        let msg = wire::parse_message(&answer).unwrap();
        assert!(msg.put.is_some());

        handle.shutdown();
    }

    #[tokio::test]
    async fn put_relayed_to_other_peers() {
        let handle = spawn_in_memory(test_config()).await.unwrap();

        let mut ws_a = ws_connect(handle.addr, "/gun").await;
        let mut ws_b = ws_connect(handle.addr, "/gun").await;

        // Drain B's handshake first.
        let _ = wait_for_frame(&mut ws_b, |f| f.contains(r###""dam":"?""###)).await;

        ws_a.send(Message::Text(
            r###"{"#":"rel1","put":{"chat":{"_":{"#":"chat",">":{"msg":1700000000000}},"msg":"hello"}}}"###.into(),
        ))
        .await
        .unwrap();

        let relayed = wait_for_frame(&mut ws_b, |f| {
            f.contains(r###""#":"rel1""###) && f.contains("hello")
        })
        .await
        .expect("PUT relayed to peer B");
        let msg = wire::parse_message(&relayed).unwrap();
        assert!(msg.seen_by.is_some(), "relay populates ><");

        handle.shutdown();
    }

    #[tokio::test]
    async fn mob_threshold_sheds_new_peers() {
        let mut config = test_config();
        config.max_peers = 1;
        config.peers = vec!["ws://other.example/gun".into()];
        let handle = spawn(config).await.unwrap();

        // First peer connects fine.
        let mut ws_a = ws_connect(handle.addr, "/gun").await;
        let _ = wait_for_frame(&mut ws_a, |f| f.contains(r###""dam":"?""###)).await;

        // Second peer gets the mob redirect and is disconnected.
        let mut ws_b = ws_connect(handle.addr, "/gun").await;
        let mob = wait_for_frame(&mut ws_b, |f| f.contains(r###""dam":"mob""###))
            .await
            .expect("mob message");
        assert!(mob.contains("other.example"));

        handle.shutdown();
    }

    #[tokio::test]
    async fn storage_persists_and_rehydrates() {
        use crate::storage::{storage_key, StoredValue};

        // First relay run: write through a shared adapter.
        let adapter = crate::storage::MemoryStorage::new();
        // Seed via a fake previous run.
        let mut seeded = adapter;
        seeded
            .put(
                &storage_key("boot", "v").unwrap(),
                &StoredValue {
                    value: GunValue::Number(7.0),
                    state: 1.0,
                },
            )
            .unwrap();

        let handle = spawn_with_storage(test_config(), Box::new(seeded))
            .await
            .unwrap();
        assert_eq!(
            handle.gun().get("boot").get("v").val(),
            Some(GunValue::Number(7.0)),
            "data hydrated from storage at startup"
        );
        handle.shutdown();
    }

    #[tokio::test]
    async fn bye_writes_applied_on_disconnect() {
        let handle = spawn_in_memory(test_config()).await.unwrap();

        let mut ws = ws_connect(handle.addr, "/gun").await;
        ws.send(Message::Text(
            r###"{"#":"bye-reg","bye":{"presence":{"alice":"offline"}}}"###.into(),
        ))
        .await
        .unwrap();
        // Wait until the registration was processed (handshake exchange
        // guarantees ordering on the same connection).
        let _ = wait_for_frame(&mut ws, |f| f.contains(r###""dam":"?""###)).await;

        ws.close(None).await.unwrap();
        drop(ws);

        // Disconnection processing is async — poll for the write.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if handle.gun().get("presence").get("alice").val()
                == Some(GunValue::Text("offline".into()))
            {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "bye write applied on disconnect"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        handle.shutdown();
    }

    #[tokio::test]
    async fn default_storage_is_rad_on_filesystem() {
        let dir = std::env::temp_dir().join(format!(
            "gunmetal-relay-test-{}",
            crate::uuid::generate_uuid()
        ));
        let mut config = test_config();
        config.file = dir.to_string_lossy().to_string();

        // First run: write through a connected client.
        let handle = spawn(config.clone()).await.unwrap();
        let mut ws = ws_connect(handle.addr, "/gun").await;
        ws.send(Message::Text(
            r###"{"#":"fs1","put":{"persist":{"_":{"#":"persist",">":{"k":1700000000000}},"k":"on-disk"}}}"###.into(),
        ))
        .await
        .unwrap();
        let _ = wait_for_frame(&mut ws, |f| f.contains(r###""@":"fs1""###)).await;
        drop(ws);
        handle.shutdown();
        // RAD batches writes for 250ms before flushing to disk.
        tokio::time::sleep(Duration::from_millis(600)).await;
        drop(handle);

        // Second run from the same directory: data rehydrates from RAD.
        let handle2 = spawn(config).await.unwrap();
        assert_eq!(
            handle2.gun().get("persist").get("k").val(),
            Some(GunValue::Text("on-disk".into())),
            "relay restarted from RAD filesystem storage"
        );
        handle2.shutdown();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
