//! R21 + R-RPC-4 socket daemon lifecycle tests.
//!
//! Fix spec §6.4/§5.4: the socket file is unlinked on EVERY shutdown path
//! (the pre-fix select! dropped the run future on SIGTERM, so the unlink
//! after the accept loop never ran), and socket notifications (absent id)
//! receive no reply.
//!
//! Unix-only (signals + unix sockets).
#![cfg(unix)]

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn binary_path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    p.pop();
    p.push("maproom");
    assert!(p.exists(), "maproom binary not built");
    p
}

fn spawn_socket_daemon(db_dir: &std::path::Path, sock: &std::path::Path, idle: u32) -> Child {
    // Review [34]/[37]: a per-test --pid-path. Without it every daemon in
    // this binary raced for the global /tmp/maproom-{uid}.pid: under the
    // default parallel harness (or with a real user daemon running) the
    // flock losers exited AlreadyRunning and 'socket must appear' flaked.
    let pid_path = db_dir.join("daemon.pid");
    Command::new(binary_path())
        .args(["serve", "--socket", "--socket-path"])
        .arg(sock)
        .arg("--pid-path")
        .arg(&pid_path)
        .args(["--idle-timeout", &idle.to_string()])
        .env(
            "MAPROOM_DATABASE_URL",
            format!("sqlite://{}/w.db", db_dir.display()),
        )
        .env_remove("MAPROOM_EMBEDDING_PROVIDER")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn wait_for_socket(sock: &std::path::Path, secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        if sock.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// Kill-on-drop guard: a panicking assertion must never leak a daemon (the
/// PID file is global per-uid, so a leaked daemon cascades into
/// AlreadyRunning failures in later tests).
struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The socket protocol is LENGTH-PREFIXED (u32 BE + JSON payload), not
/// newline-delimited — see daemon/protocol.rs JsonRpcCodec.
fn write_framed(stream: &mut UnixStream, json: &str) {
    let len = (json.len() as u32).to_be_bytes();
    stream.write_all(&len).unwrap();
    stream.write_all(json.as_bytes()).unwrap();
}

fn read_framed(stream: &mut UnixStream) -> std::io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    let mut read = 0;
    while read < 4 {
        let n = stream.read(&mut len_buf[read..])?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "eof in length prefix",
            ));
        }
        read += n;
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut payload = vec![0u8; len];
    let mut got = 0;
    while got < len {
        let n = stream.read(&mut payload[got..])?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "eof in payload",
            ));
        }
        got += n;
    }
    Ok(payload)
}

/// R21 / R-SOCK-1: idle shutdown unlinks the socket file.
#[test]
fn socket_file_removed_after_idle_shutdown() {
    let db = tempfile::TempDir::new().unwrap();
    let sock = db.path().join("idle.sock");
    let child = spawn_socket_daemon(db.path(), &sock, 2);
    let mut guard = ChildGuard(child);
    let child = &mut guard.0;
    assert!(wait_for_socket(&sock, 10), "socket must appear");

    // R20 gives tick=1s for a 2s timeout: exit within ~5s.
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(st) = child.try_wait().unwrap() {
            break st;
        }
        assert!(
            Instant::now() < deadline,
            "daemon did not idle-exit in time (R20 regression?)"
        );
        std::thread::sleep(Duration::from_millis(200));
    };
    assert!(status.success(), "idle exit should be clean");
    assert!(
        !sock.exists(),
        "socket file must be unlinked after idle shutdown (R21)"
    );
}

/// R21 / R-SOCK-1/2: SIGTERM unlinks the socket file (the exact leaked path).
#[test]
fn socket_file_removed_after_sigterm() {
    let db = tempfile::TempDir::new().unwrap();
    let sock = db.path().join("term.sock");
    let child = spawn_socket_daemon(db.path(), &sock, 300);
    let mut guard = ChildGuard(child);
    let child = &mut guard.0;
    assert!(wait_for_socket(&sock, 10), "socket must appear");

    // SIGTERM via kill(2) — no external `kill` dependency.
    unsafe {
        libc_kill(child.id() as i32, 15);
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        assert!(Instant::now() < deadline, "daemon did not exit on SIGTERM");
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(
        !sock.exists(),
        "socket file must be unlinked on SIGTERM (pre-R21 the select! dropped run() before cleanup)"
    );
}

extern "C" {
    #[link_name = "kill"]
    fn libc_kill(pid: i32, sig: i32) -> i32;
}

/// R-RPC-4: a socket notification (absent id) gets NO reply within the read
/// timeout, while a normal request on the same connection is answered.
/// Speaks the daemon's length-prefixed framing (JsonRpcCodec).
#[test]
fn socket_notification_receives_no_reply() {
    let db = tempfile::TempDir::new().unwrap();
    let sock = db.path().join("notif.sock");
    let child = spawn_socket_daemon(db.path(), &sock, 60);
    let _guard = ChildGuard(child);
    assert!(wait_for_socket(&sock, 10), "socket must appear");

    let mut stream = UnixStream::connect(&sock).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();

    // Notification: no id -> no reply expected (read must time out).
    write_framed(&mut stream, r#"{"jsonrpc":"2.0","method":"ping"}"#);
    match read_framed(&mut stream) {
        Ok(payload) => panic!(
            "notification must not be answered (R-RPC-4); got: {}",
            String::from_utf8_lossy(&payload)
        ),
        Err(e) => assert!(
            matches!(
                e.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ),
            "expected read timeout, got: {e:?}"
        ),
    }

    // Control: a normal request IS answered (echo stub is fine).
    write_framed(&mut stream, r#"{"jsonrpc":"2.0","method":"ping","id":1}"#);
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let payload = read_framed(&mut stream).expect("request must be answered");
    let v: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(v["id"], 1, "echo-stub reply carries the request id");
}
