//! # Gunmetal
//!
//! A Rust/WASM decentralized graph database with CRDT conflict resolution,
//! end-to-end encryption, and peer-to-peer sync.
//!
//! ## Quick Start
//!
//! ```rust
//! use gunmetal::{Gun, GunOptions, GunValue};
//!
//! let gun = Gun::new(GunOptions::default());
//!
//! // Write
//! gun.get("mark").put_kv("name", GunValue::Text("Mark".into()));
//!
//! // Read
//! let name = gun.get("mark").get("name").val();
//! assert_eq!(name, Some(GunValue::Text("Mark".into())));
//! ```
//!
//! ## Module Overview
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`instance`] | `Gun` + `GunChain` -- main API surface |
//! | [`types`] | `GunValue`, `Node`, `Soul` -- core data model |
//! | [`graph`] | In-memory graph store with LRU eviction |
//! | [`crdt`] | HAM conflict resolution algorithm |
//! | [`sea`] | ECDSA signing, AES-GCM encryption, PBKDF2 |
//! | [`user`] | Decentralized authentication + `SignedChain` |
//! | [`cert`] | Certificate system for delegated write permissions |
//! | [`sync`] | Peer-to-peer replication with GET handling |
//! | [`storage`] | Sync/async persistence adapters + `BatchWriter` |
//! | [`rad`] | RAD radix storage engine (chunked, batched persistence) |
//! | [`transport`] | WebSocket transport, reconnection, peer tracking |
//! | [`wire`] | JSON wire protocol (PUT/GET/ACK messages) |
//! | [`events`] | Pub-sub event bus |
//! | [`uuid`] | Time-sortable UUID generation for collections |
//! | [`concurrency`] | Platform-gated shared state (`Arc`/`Rc`) |
//! | [`runtime`] | Cross-platform `spawn()`/`sleep()` |
//!
//! ## Dual Target
//!
//! Compiles to both native and `wasm32-unknown-unknown`. Platform differences
//! are abstracted via [`concurrency`] (lock types) and [`runtime`] (spawn/sleep).
//! The [`wasm`] module provides JavaScript bindings via `wasm-bindgen`.

pub mod cert;
pub mod concurrency;
pub mod crdt;
pub mod dup;
pub mod events;
#[cfg(feature = "extended-api")]
pub mod extended;
pub mod graph;
pub mod instance;
pub mod lex;
pub mod mesh;
pub mod rad;
#[cfg(all(feature = "relay", not(target_arch = "wasm32")))]
pub mod relay;
pub mod runtime;
pub mod sea;
pub mod state;
pub mod storage;
pub mod sync;
pub mod transport;
pub mod types;
pub mod user;
pub mod uuid;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
pub mod wire;

pub use dup::Dup;
pub use events::EventBus;
pub use graph::Graph;
pub use instance::{Gun, GunChain, GunOptions};
pub use mesh::{Mesh, MeshConfig};
pub use state::State;
pub use types::{GunValue, Node, Soul};
pub use uuid::generate_uuid;
pub use wire::WireMessage;
