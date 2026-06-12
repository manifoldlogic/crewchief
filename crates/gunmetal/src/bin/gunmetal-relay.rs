//! gunmetal-relay — a GUN.js-compatible WebSocket relay.
//!
//! ```text
//! gunmetal-relay [--port 8765] [--host 0.0.0.0] [--path /gun]
//!                [--file radata] [--health-path /health]
//!                [--tls-cert cert.pem --tls-key key.pem]   (relay-tls builds)
//!                [--mob 999999] [--peer wss://other/gun]...
//! ```
//!
//! Environment variables (`PORT`, `HOST`, `GUN_PATH`, `GUN_FILE`,
//! `TLS_CERT`, `TLS_KEY`, `MOB`, `GUN_PEERS`) are overridden by flags.

#[cfg(not(target_arch = "wasm32"))]
const USAGE: &str = "gunmetal-relay — GUN.js-compatible WebSocket relay

USAGE:
    gunmetal-relay [OPTIONS]

OPTIONS:
    --port <PORT>           Listen port            [env: PORT]      [default: 8765]
    --host <HOST>           Listen host            [env: HOST]      [default: 0.0.0.0]
    --path <PATH>           WebSocket path         [env: GUN_PATH]  [default: /gun]
    --file <DIR>            Storage directory      [env: GUN_FILE]  [default: radata]
    --health-path <PATH>    Health check endpoint                   [default: /health]
    --tls-cert <PEM>        TLS certificate        [env: TLS_CERT]  (relay-tls builds)
    --tls-key <PEM>         TLS private key        [env: TLS_KEY]   (relay-tls builds)
    --mob <N>               Max peers before shedding [env: MOB]    [default: 999999]
    --peer <URL>            Upstream relay to join (repeatable) [env: GUN_PEERS]
    --help                  Show this help
";

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    use gunmetal::relay::{run, RelayConfig};

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{}", USAGE);
        return;
    }

    let config = match RelayConfig::default().apply_env().apply_args(args) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("error: {}\n\n{}", err, USAGE);
            std::process::exit(2);
        }
    };

    if let Err(err) = run(config).await {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}

/// The relay is a native-only server; wasm builds get a stub.
#[cfg(target_arch = "wasm32")]
fn main() {}
