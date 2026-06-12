# GUN Documentation Index

GUN is a decentralized, real-time, offline-first graph database. Data is stored
as a graph of nodes that sync peer-to-peer across browsers, servers, and mobile
devices using a conflict-resolution algorithm called HAM (Hypothetical Amnesia
Machine). GUN requires no central server -- any peer can read, write, and relay
data.

This documentation is derived from the GUN source code and official wiki. It
covers the full API surface, security model, network architecture, and practical
patterns for building applications.

---

## Getting Started

| File | Description |
|------|-------------|
| [quickstart.md](quickstart.md) | Installation, creating an instance, connecting to peers, basic CRUD operations, realtime subscriptions, user authentication, and first-app walkthrough. Start here. |

---

## Core Reference

| File | Description |
|------|-------------|
| [core-api.md](core-api.md) | The foundational API -- `Gun()` constructor, `.get()`, `.put()`, `.on()`, `.once()`, `.set()`, `.map()`, `.off()`, `.back()`, and `.opt()`. Covers signatures, options, behavior, and source-level detail for each method. |
| [extended-api.md](extended-api.md) | Convenience and utility methods built on top of the core API -- `.path()`, `.not()`, `.open()`, `.load()`, `.then()`, `.promise()`, `.bye()`, `.later()`, `.unset()`, and more. These are provided as separate modules in `gun/lib/`. |
| [typescript-types.md](typescript-types.md) | Complete TypeScript type definitions for GUN, SEA, and the extended API. Covers `IGunInstance`, `IGunChain`, `IGunUserInstance`, `ISEAPair`, and all related interfaces. |

---

## Security

| File | Description |
|------|-------------|
| [sea.md](sea.md) | SEA (Security, Encryption, Authorization) cryptographic utilities. Covers `SEA.pair()`, `SEA.sign()`, `SEA.verify()`, `SEA.encrypt()`, `SEA.decrypt()`, `SEA.work()`, `SEA.secret()`, and `SEA.certify()` with algorithm details, key formats, and usage patterns. |
| [user-api.md](user-api.md) | User authentication and identity management. Covers `gun.user()`, `.create()`, `.auth()`, `.leave()`, `.recall()`, `.alive()`, `.delete()`, key pairs, trust, certificates, and permission models. |

---

## Architecture

| File | Description |
|------|-------------|
| [architecture.md](architecture.md) | Deep dive into GUN internals -- the graph data model, HAM conflict resolution algorithm, wire protocol format, hook/plugin system, module structure, and the `ask`/`ack` message flow. |
| [networking.md](networking.md) | Network transport layer -- AXE (Automatic eXchange Engine) for intelligent routing, DAM (Daisy-chain Ad-hoc Mesh) for transport abstraction, WebSocket and WebRTC transports, relay peer configuration, mesh topology, and peer discovery. |
| [rad.md](rad.md) | RAD (Radix Address Database) storage engine. Covers the radix trie structure, disk persistence, file format, read/write paths, caching, compaction, and how RAD integrates with GUN's storage adapter interface. |

---

## Guides

| File | Description |
|------|-------------|
| [data-patterns.md](data-patterns.md) | Practical data modeling recipes -- graph structure, partial merge semantics, circular references, one-to-one / one-to-many / many-to-many relationships, tables and collections, LEX queries for filtering and pagination, distributed counters, deletion and tombstoning, time-series patterns, content-addressed immutable data, and complete schema examples for chat, social feeds, todos, and game state. |
| [utilities.md](utilities.md) | Node helper functions and general-purpose utilities -- `Gun.node.is()`, `Gun.node.soul()`, `Gun.node.ify()`, `Gun.text` string utilities, `Gun.obj` object helpers, and scheduling/timing utilities. |
