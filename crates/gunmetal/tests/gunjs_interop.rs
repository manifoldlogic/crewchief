//! Wire compatibility acceptance test against the real GUN.js client
//! (spec: gunmetal-parity.md §Acceptance Criteria).
//!
//! Flow:
//! 1. Start a gunmetal relay
//! 2. GUN.js (Node) writes `gun.get('test').put({name: 'Alice'})`
//! 3. A gunmetal client connects and reads `test.name` → `Text("Alice")`
//! 4. The gunmetal client writes `age = 30` through the mesh
//! 5. GUN.js reads `gun.get('test').on(cb)` → `{name: 'Alice', age: 30}`
//!
//! Skips (passing) when `node` or the vendored gun.js source is missing,
//! so plain `cargo test` works in minimal environments.

#![cfg(all(feature = "relay", not(target_arch = "wasm32")))]

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use gunmetal::mesh::{Mesh, MeshConfig, PeerSender};
use gunmetal::relay::{spawn_in_memory, RelayConfig};
use gunmetal::types::GunValue;
use gunmetal::{wire, Gun, GunOptions};

fn gun_js_path() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sources/gun/gun.js");
    path.exists().then_some(path)
}

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
}

fn script_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/interop").join(name)
}

/// Run a node interop script, returning (status_ok, combined_output).
async fn run_node_script(name: &str, relay_url: &str, gun_js: &std::path::Path) -> (bool, String) {
    let script = script_path(name);
    let relay_url = relay_url.to_string();
    let gun_js = gun_js.to_path_buf();
    tokio::task::spawn_blocking(move || {
        match Command::new("node")
            .arg(&script)
            .arg(&relay_url)
            .arg(&gun_js)
            .output()
        {
            Ok(out) => {
                let combined = format!(
                    "{}{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                (out.status.success(), combined)
            }
            Err(e) => (false, format!("failed to run node: {}", e)),
        }
    })
    .await
    .unwrap_or((false, "join error".into()))
}

/// A gunmetal client connected to a relay over a real WebSocket: a `Gun`
/// instance whose mesh sends/receives frames through tokio-tungstenite.
async fn gunmetal_client(relay_url: &str) -> (Gun, Mesh) {
    let (ws, _) = tokio_tungstenite::connect_async(relay_url)
        .await
        .expect("gunmetal client connects");
    let (mut sink, mut source) = ws.split();

    let gun = Gun::new(GunOptions::default());
    let mesh = Mesh::new(gun.clone(), MeshConfig::default());

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let sender: PeerSender = Arc::new(move |raw: &str| {
        let _ = tx.send(raw.to_string());
    });
    mesh.hi("relay", Some(relay_url.to_string()), Some(sender));

    let pump_mesh = mesh.clone();
    tokio::spawn(async move {
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
                            pump_mesh.hear_async(text.as_str(), "relay").await;
                        }
                        Some(Ok(_)) => {}
                        Some(Err(_)) | None => break,
                    }
                }
            }
        }
    });

    (gun, mesh)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn gunjs_full_interop_acceptance() {
    let Some(gun_js) = gun_js_path() else {
        eprintln!("SKIP: sources/gun/gun.js not present (submodule not checked out)");
        return;
    };
    if !node_available() {
        eprintln!("SKIP: node not available");
        return;
    }

    // 1. Start a gunmetal relay.
    let relay = spawn_in_memory(RelayConfig {
        port: 0,
        host: "127.0.0.1".into(),
        ..Default::default()
    })
    .await
    .expect("relay starts");
    let relay_url = format!("ws://{}/gun", relay.addr);

    // 2. GUN.js writes {name: 'Alice'}.
    let (ok, output) = run_node_script("gun-write.cjs", &relay_url, &gun_js).await;
    assert!(ok, "GUN.js write failed:\n{}", output);
    assert!(output.contains("PUT_ACK"), "no put ack:\n{}", output);

    // 3. A gunmetal client connects and reads test.name.
    let (client_gun, client_mesh) = gunmetal_client(&relay_url).await;
    client_mesh.say(
        wire::get_message(&gunmetal::uuid::random_message_id(9), "test", None),
        None,
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(val) = client_gun.get("test").get("name").val() {
            assert_eq!(
                val,
                GunValue::Text("Alice".into()),
                "gunmetal client read GUN.js's write"
            );
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for GET answer from relay"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    // 4. The gunmetal client writes age = 30 — local write propagates
    //    through the mesh to the relay.
    client_gun
        .get("test")
        .put_kv("age", GunValue::Number(30.0));

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if relay.gun().get("test").get("age").val() == Some(GunValue::Number(30.0)) {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for gunmetal write to reach the relay"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    // 5. GUN.js reads the merged document.
    let (ok, output) = run_node_script("gun-read.cjs", &relay_url, &gun_js).await;
    assert!(ok, "GUN.js read failed:\n{}", output);
    assert!(
        output.contains(r#""name":"Alice""#) && output.contains(r#""age":30"#),
        "GUN.js must see both writes:\n{}",
        output
    );

    relay.shutdown();
}
