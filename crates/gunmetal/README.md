# Gunmetal

A Rust/WASM decentralized graph database with CRDT conflict resolution, end-to-end encryption, and peer-to-peer sync. Inspired by GUN, built from scratch in Rust.

## Features

- **CRDT conflict resolution** via HAM (Hypothetical Amnesia Machine) -- convergent state across peers with no coordination
- **Dual-target** -- compiles to native and `wasm32-unknown-unknown` from the same codebase
- **SEA cryptography** -- ECDSA signing, ECDH key exchange, AES-256-GCM encryption, PBKDF2 key derivation (all pure Rust, no WebCrypto dependency)
- **Decentralized auth** -- user accounts stored as keypairs in the graph, no central authority
- **Certificate system** -- delegate write permissions to other users with signed, expirable certificates
- **Auto-signing** -- `SignedChain` wrapper signs all writes with the user's private key; verification on receive rejects forgeries
- **LRU eviction** -- bounded memory with configurable capacity, pinned subscriptions, whole-node eviction
- **Peer-to-peer sync** -- pluggable transport layer with reconnection backoff, peer health tracking, GET/PUT request handling
- **Persistence** -- sync and async storage adapters; IndexedDB schema for browsers; batched writes via `BatchWriter`

## Quick Start

### Add to Cargo.toml

```toml
[dependencies]
gunmetal = { path = "crates/gunmetal" }
```

### Basic CRUD

```rust
use gunmetal::{Gun, GunOptions, GunValue};

// Create a database instance
let gun = Gun::new(GunOptions::default());

// Write data
gun.get("mark").put_kv("name", GunValue::Text("Mark".into()));
gun.get("mark").put_kv("age", GunValue::Number(30.0));

// Read data
let name = gun.get("mark").get("name").val();
assert_eq!(name, Some(GunValue::Text("Mark".into())));

// Write multiple fields at once
gun.get("alice").put(vec![
    ("name".into(), GunValue::Text("Alice".into())),
    ("role".into(), GunValue::Text("admin".into())),
    ("active".into(), GunValue::Bool(true)),
]);

// Graph links (references between nodes)
gun.get("mark").put_kv("boss", GunValue::Link("alice".into()));
let boss_name = gun.get("mark").get("boss").get("name").val();
assert_eq!(boss_name, Some(GunValue::Text("Alice".into())));

// Delete (tombstone)
gun.get("mark").get("age").put_value(GunValue::Null);
```

### Realtime Subscriptions

```rust
use gunmetal::{Gun, GunOptions, GunValue};
use std::sync::{Arc, Mutex};

let gun = Gun::new(GunOptions::default());

// Subscribe to changes -- fires immediately with current data, then on every update
let log = Arc::new(Mutex::new(Vec::new()));
let log2 = log.clone();

let listener_id = gun.get("chat").get("latest").on(move |val, key| {
    log2.lock().unwrap().push(format!("{}: {:?}", key, val));
});

gun.get("chat").put_kv("latest", GunValue::Text("hello".into()));
gun.get("chat").put_kv("latest", GunValue::Text("world".into()));

// Unsubscribe
gun.get("chat").get("latest").off(listener_id);
```

### Collections with UUID

```rust
use gunmetal::{Gun, GunOptions, GunValue};

let gun = Gun::new(GunOptions::default());

// Add items with auto-generated time-sortable UUIDs
let id1 = gun.get("messages").set_value(GunValue::Text("first message".into()));
let id2 = gun.get("messages").set_value(GunValue::Text("second message".into()));

// IDs are unique and time-sortable
assert_ne!(id1, id2);

// Retrieve by ID
let msg = gun.get("messages").get(&id1).val();
assert_eq!(msg, Some(GunValue::Text("first message".into())));
```

### User Authentication

```rust
use gunmetal::{Gun, GunOptions, GunValue};
use gunmetal::user::{User, CreateResult, AuthResult};

let gun = Gun::new(GunOptions::default());
let mut user = User::new(gun.clone());

// Create account (generates keypair, encrypts with password, stores in graph)
match user.create("alice", "secure_password_123") {
    CreateResult::Ok { pub_key } => println!("Created user: {}", pub_key),
    CreateResult::Err { err } => panic!("Failed: {}", err),
}

// Write to user namespace
user.get("profile")
    .unwrap()
    .put_value(GunValue::Text("Hello world".into()));

// Log out
user.leave();

// Log back in
let mut user2 = User::new(gun.clone());
match user2.auth_with_password("alice", "secure_password_123") {
    AuthResult::Ok(auth) => println!("Logged in as: {}", auth.alias),
    AuthResult::Err { err } => panic!("Auth failed: {}", err),
}
```

### Signed Data (Auto-Signing)

```rust
use gunmetal::{Gun, GunOptions, GunValue};
use gunmetal::user::User;

let gun = Gun::new(GunOptions::default());
let mut user = User::new(gun.clone());
user.create("bob", "password12345");

// get_signed() returns a SignedChain that auto-signs writes
let chain = user.get_signed("bio").unwrap();
chain.put_value(GunValue::Text("I am Bob".into()));

// Reading through SignedChain verifies the signature
let verified = chain.val();
assert_eq!(verified, Some(GunValue::Text("I am Bob".into())));

// Raw value in graph is a SEA{...} signed string
let pub_key = user.is_authenticated().unwrap().pub_key.clone();
let raw = gun.get(&format!("~{}", pub_key)).get("bio").val().unwrap();
// raw is GunValue::Text("SEA{\"m\":\"I am Bob\",\"s\":\"...\"}")
```

### Certificates (Delegated Permissions)

```rust
use gunmetal::sea;
use gunmetal::cert::{Certificate, CertWho, CertWhat};

let alice = sea::pair().unwrap();
let bob = sea::pair().unwrap();

// Alice grants Bob write access to her shared/ namespace
let cert = Certificate::create(
    CertWho::PubKey(bob.pub_key.clone()),
    CertWhat::Prefix("shared/".into()),
    None, // no expiry
    &alice.pub_key,
    &alice.priv_key,
).unwrap();

// Verify the certificate
assert!(cert.verify().unwrap());

// Check access
assert!(cert.grants_access(&bob.pub_key, "shared/doc1", 0.0));
assert!(!cert.grants_access(&bob.pub_key, "private/secret", 0.0));

// Store in graph at ~alice/certs/<id>
let gun = gunmetal::Gun::new(gunmetal::GunOptions::default());
let cert_key = format!("certs/{}", cert.cert_id());
gun.get(&format!("~{}", alice.pub_key))
    .put_kv(&cert_key, cert.to_gun_value());
```

### SEA Cryptography

```rust
use gunmetal::sea;

// Generate keypair
let pair = sea::pair().unwrap();

// Sign & verify
let data = serde_json::json!({"action": "transfer", "amount": 100});
let signed = sea::sign(&data, &pair.priv_key, &pair.pub_key).unwrap();
let verified = sea::verify(&signed, &pair.pub_key).unwrap();
assert_eq!(verified, data);

// Encrypt & decrypt
let secret = serde_json::json!("classified");
let encrypted = sea::encrypt(&secret, &pair.epriv).unwrap();
let decrypted = sea::decrypt(&encrypted, &pair.epriv).unwrap();
assert_eq!(decrypted, secret);

// Shared secret (ECDH) for two-party encryption
let alice = sea::pair().unwrap();
let bob = sea::pair().unwrap();
let shared = sea::secret(&bob.epub, &alice.epriv).unwrap();
let enc = sea::encrypt(&serde_json::json!("hello bob"), &shared).unwrap();
let dec = sea::decrypt(&enc, &shared).unwrap();
```

### Peer-to-Peer Sync

```rust
use gunmetal::{Gun, GunOptions, GunValue};
use gunmetal::sync::sync_pair;

let gun_a = Gun::new(GunOptions::default());
let gun_b = Gun::new(GunOptions::default());

// Connect two instances for bidirectional sync
let (mut sync_a, mut sync_b) = sync_pair(gun_a.clone(), gun_b.clone());

// Write on A, flush to B
gun_a.get("shared").put_kv("status", GunValue::Text("online".into()));
sync_a.flush();

// B now has the data (HAM-resolved)
assert_eq!(
    gun_b.get("shared").get("status").val(),
    Some(GunValue::Text("online".into()))
);

// Request data from peers via GET
gun_b.get("config").put_kv("theme", GunValue::Text("dark".into()));
sync_a.request("config", Some("theme"));
assert_eq!(
    gun_a.get("config").get("theme").val(),
    Some(GunValue::Text("dark".into()))
);
```

### Persistence

```rust
use gunmetal::{Gun, GunOptions, GunValue};
use gunmetal::storage::{MemoryStorage, StorageEngine};

let gun = Gun::new(GunOptions::default());

// Attach storage -- all writes auto-persist
let engine = StorageEngine::new(gun.clone(), MemoryStorage::new());

gun.get("config").put_kv("theme", GunValue::Text("dark".into()));

// Load into a new instance
let gun2 = Gun::new(GunOptions::default());
let engine2 = StorageEngine::new(gun2.clone(), MemoryStorage::new());
// In production, you'd pass the same storage backend (filesystem, IndexedDB, etc.)
```

### WASM / JavaScript

```js
import init, { WasmGun, WasmSEA, WasmUser } from './gunmetal.js';

await init();

const gun = new WasmGun();
const sea = new WasmSEA();
const user = new WasmUser(gun);

// Create account
const result = JSON.parse(user.create("alice", "password123"));
console.log("Public key:", result.pub);

// Write signed data
user.putSigned("profile", JSON.stringify("Alice"));

// Read and verify
const profile = user.getSigned("profile"); // verified, signature stripped

// SEA crypto
const pair = JSON.parse(sea.pair());
const signed = sea.sign(JSON.stringify({msg: "hello"}), pair.priv, pair.pub);
const verified = sea.verify(signed, pair.pub);

// Direct graph access
gun.putText("chat", "msg1", "hello world");
const val = gun.get("chat", "msg1"); // JSON string
```

Build for WASM:

```bash
cargo build --target wasm32-unknown-unknown
# Or with wasm-pack:
wasm-pack build --target web crates/gunmetal
```

## Architecture

```
gunmetal/src/
├── lib.rs              # Crate root, re-exports
├── types.rs            # GunValue, Node, Soul -- core data model
├── crdt.rs             # HAM conflict resolution algorithm
├── graph.rs            # In-memory graph with LRU eviction
├── instance.rs         # Gun + GunChain -- main API surface
├── events.rs           # Pub-sub event bus
├── state.rs            # Monotonic clock for timestamps
├── dup.rs              # Message deduplication
├── lex.rs              # Lexical expression matching
├── wire.rs             # JSON wire protocol (PUT/GET/ACK)
├── sea.rs              # ECDSA, ECDH, AES-GCM, PBKDF2
├── user.rs             # Decentralized auth + SignedChain
├── cert.rs             # Certificate system for delegated writes
├── uuid.rs             # Time-sortable UUID generation
├── concurrency.rs      # Platform-gated Shared<T> (Arc/Rc)
├── runtime.rs          # spawn/sleep abstraction
├── sync.rs             # Peer replication + GET handling
├── storage/
│   ├── mod.rs          # StorageAdapter + AsyncStorageAdapter traits
│   ├── engine.rs       # BatchWriter for buffered async writes
│   └── indexeddb.rs    # IndexedDB schema + record types (WASM)
├── transport/
│   ├── mod.rs          # AsyncSyncAdapter trait
│   ├── reconnect.rs    # Exponential backoff with jitter
│   ├── peers.rs        # Peer registry + health tracking
│   ├── ws_native.rs    # Native WebSocket config (tokio-tungstenite)
│   └── ws_wasm.rs      # Browser WebSocket config (web-sys)
└── wasm.rs             # JavaScript bindings via wasm-bindgen
```

### Data Flow

```
Write: put() → HAM resolve → graph merge → emit "put" event → storage persist → sync broadcast
Read:  val() → graph lookup (cache hit) or storage load (cache miss)
Sync:  receive() → dedup → verify signatures → HAM merge → emit events
GET:   request() → broadcast to peers → peer looks up data → responds with PUT + @ack
```

### Conflict Resolution

Every key-value pair carries a state timestamp. When two writes conflict, HAM determines the winner:

1. **Newer state wins** -- higher timestamp takes precedence
2. **Same state, larger value wins** -- deterministic tiebreak by value comparison
3. **Future states deferred** -- values with timestamps too far ahead are rejected (10-minute drift cap)
4. **Convergent** -- all peers reach the same state regardless of message ordering

### Security Model

- **User namespace**: Data under `~<pubKey>/...` belongs to that keypair holder
- **Auto-signing**: `SignedChain` wraps values as `SEA{m:...,s:...}` signed strings
- **Receive verification**: `gun.receive()` verifies signatures for `~pubKey` souls, silently drops forgeries
- **Certificates**: Owners grant delegated write access via signed, expirable certificates
- **Encryption**: AES-256-GCM with PBKDF2-derived keys; ECDH shared secrets for two-party encryption

## Building

```bash
# Native
cargo build -p gunmetal
cargo test -p gunmetal

# WASM
cargo build -p gunmetal --target wasm32-unknown-unknown

# Both
cargo build -p gunmetal && cargo build -p gunmetal --target wasm32-unknown-unknown
```

## Configuration

### Graph Eviction

```rust
use gunmetal::graph::EvictionConfig;

let config = EvictionConfig {
    max_nodes: 10_000,       // max nodes in memory
    max_keys: 1_000_000,     // max total key-value pairs
    eviction_fraction: 0.1,  // evict 10% when limit hit
};
```

Subscriptions auto-pin their souls (preventing eviction). Unsubscribing unpins.

### Reconnection Backoff

```rust
use gunmetal::transport::reconnect::{ReconnectConfig, ReconnectState};
use std::time::Duration;

let config = ReconnectConfig {
    base_delay: Duration::from_secs(1),
    max_delay: Duration::from_secs(30),
    jitter_fraction: 0.25,
    max_attempts: Some(10), // or None for unlimited
};

let mut state = ReconnectState::new(config);
// state.next_delay() returns Some(Duration) or None if exhausted
// state.reset() after successful connection
```

### Batch Writes

```rust
use gunmetal::storage::engine::{BatchWriter, BatchConfig};

let config = BatchConfig {
    max_buffer_size: 1000,   // flush at 1000 entries
    flush_interval_ms: 100,  // or every 100ms
};
```

## Tests

```bash
# Run all tests (includes slow PBKDF2 crypto tests ~2-3 min in debug)
cargo test -p gunmetal

# Run fast tests only (skip PBKDF2-heavy tests)
cargo test -p gunmetal -- --skip sea::tests::work --skip user::tests::auth_with_password \
  --skip user::tests::auth_wrong --skip user::tests::full_lifecycle

# Run specific module tests
cargo test -p gunmetal -- cert::
cargo test -p gunmetal -- graph::tests::eviction
cargo test -p gunmetal -- instance::tests::get_
```

## License

MIT
