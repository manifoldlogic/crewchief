# GUN TypeScript Type Reference

Complete type reference for GUN's TypeScript definitions. These types are available from the `gun` package and its bundled `gun/types` declarations. They describe the full surface area of GUN's API including the core graph database, SEA (Security, Encryption, Authorization) module, and networking layer.

---

## Core Types

### IGun

The top-level GUN constructor interface. This is the type of the `Gun` import itself -- it can be called as a function or with `new` to create a GUN instance.

```typescript
interface IGun {
  <TNode>(options?: GunOptions): IGunInstance<TNode>;
  new <TNode>(options?: GunOptions): IGunInstance<TNode>;
  state(): number;
  chain: IGunChain<any> & IGunInstance<any>;
  on(event: 'create', callback: GunHookCallbackCreate): void;
  on(event: 'opt', callback: GunHookCallbackOpt): void;
}
```

The `state()` method returns the current HAM timestamp. The `chain` property provides access to the prototype chain for extending GUN with custom methods. The `on()` method subscribes to lifecycle events at the constructor level.

### IGunInstance\<TNode\>

The main interface you work with after calling `Gun()`. It extends `IGunInstanceRoot<TNode, IGunInstance<TNode>>` and provides access to the full chain API (`get`, `put`, `set`, `on`, `once`, `map`, etc.) as well as instance-level concerns like `user()` for authentication and `opt()` for runtime configuration.

Every method that navigates the graph returns an `IGunChain`, allowing fluent chaining.

### IGunChain\<TNode, TChainParent, TGunInstance, TKey\>

The chain interface returned by navigational methods like `.get()`, `.put()`, and `.set()`. This is the workhorse type of the GUN API -- nearly every operation returns a chain, enabling the fluent pattern GUN is known for.

Key methods on the chain:

- **put(data, callback?, options?)** -- Write data to the current node
- **get(key)** -- Navigate to a child node by key
- **set(data, callback?, options?)** -- Add an item to a set (unordered collection)
- **back(amount?)** -- Navigate up the chain to a parent
- **on(callback, options?)** -- Subscribe to real-time updates
- **off()** -- Unsubscribe from updates
- **once(callback?, options?)** -- Read data once (no subscription)
- **map(callback?)** -- Iterate over all child nodes

The four type parameters allow TypeScript to track the shape of data, the parent chain for `.back()`, the root instance type, and the current key for precise callback typing.

### GunOptions

Configuration options for creating a GUN instance. Accepts either an options object, a single peer URL string, or an array of peer URL strings.

```typescript
type GunOptions = Partial<{
  file: string;
  web: any;
  s3: {
    key: string;
    secret: string;
    bucket: string;
    region?: string;
    fakes3?: any;
  };
  peers: string[] | Record<string, {}>;
  radisk: boolean;
  localStorage: boolean;
  uuid(): string;
  [key: string]: any;
}> | string | string[];
```

- **file** -- Path to the storage file (for RAD/radisk persistence)
- **web** -- HTTP server instance to attach WebSocket transport to
- **s3** -- Amazon S3 storage adapter configuration
- **peers** -- Relay peers to connect to, as URLs or a map
- **radisk** -- Enable/disable the RAD disk storage adapter
- **localStorage** -- Enable/disable browser localStorage adapter
- **uuid** -- Custom function for generating unique IDs

The index signature `[key: string]: any` allows storage adapters and plugins to accept arbitrary configuration.

### GunSchema

A recursive type representing any valid GUN data. GUN stores a graph of nodes, where each property can be a primitive value, a reference to another node, or a nested structure.

```typescript
type GunSchema = GunValueSimple | { [key: string]: GunSchema };
```

This means GUN data is either a simple value or an object whose values are themselves valid GUN data, recursively.

### GunValueSimple

The set of primitive types that GUN can store as leaf values.

```typescript
type GunValueSimple = string | number | boolean | null;
```

GUN does not support `undefined`, functions, or complex objects as values. Arrays are not directly supported either -- they must be modeled as sets using `.set()`.

### GunSoul\<N, Soul\>

Represents a reference link in the GUN graph. When a node property points to another node, it is stored as a soul reference rather than the actual data.

```typescript
type GunSoul<N, Soul extends string = string> = { '#': Soul };
```

For example, if a `user` node has a `profile` property that references another node, the raw graph data stores `{ '#': 'profile-soul-id' }` rather than embedding the profile object.

### GunDataNode\<T\>

Represents a node as it appears in callbacks (e.g., from `.on()` or `.once()`). Object properties that are references to other nodes become `GunSoul` values, and the node includes `IGunMeta` metadata.

This is the type you receive in event handlers -- it is not the same as the raw data you pass to `.put()`. The metadata `_` property is always present and contains the soul and state vectors.

### IGunMeta\<T\>

Metadata attached to every GUN node. Present on data returned in callbacks as the `_` property.

```typescript
interface IGunMeta<T extends object> {
  _: {
    '#': string;                          // soul (unique node ID)
    '>': { [key in keyof T]: number };    // state vectors (HAM timestamps)
  };
}
```

- **`#` (soul)** -- The unique identifier for this node in the graph. Every node has exactly one soul.
- **`>` (state vectors)** -- A map of property names to timestamps. Each timestamp records when that property was last updated. The HAM algorithm uses these to resolve conflicts deterministically.

---

## Query Types

GUN supports lexicographic queries through its LEX (Lexical Expression) system. These types define the query structure.

### LEX\<T\>

A lexicographic query expression used for pattern matching against keys.

```typescript
type LEX<T extends string = string> = {
  '='?: T;       // exact match
  '*'?: string;  // prefix match
  '>'?: string;  // greater than or equal (lexicographic)
  '<'?: string;  // less than or equal (lexicographic)
  '-'?: number;  // set to 1 for reverse ordering
};
```

LEX queries can be combined. For example, `{ '>': 'a', '<': 'c' }` matches all keys lexicographically between `'a'` and `'c'` inclusive.

### LEXQuery\<T\>

A full LEX query including the match expression and an optional limit.

```typescript
type LEXQuery<T extends string = string> = {
  '.': LEX<T>;   // the match expression
  ':'?: number;  // limit on number of results
};
```

---

## Callback Types

GUN's asynchronous API uses callbacks extensively. These types describe the signatures for each operation.

### GunCallbackPut

Called when a `.put()` operation receives an acknowledgment from the network.

```typescript
type GunMessagePut = { err: string } | { ok: { '': 1 } };
type GunCallbackPut = (ack: GunMessagePut) => void;
```

The acknowledgment is either an error (`{ err: 'message' }`) or a success signal (`{ ok: { '': 1 } }`). Note that GUN is eventually consistent -- a successful ack means the write was accepted locally, not that it has propagated to all peers.

### GunCallbackOn\<V, K\>

Called every time the subscribed data changes. This is the callback signature for `.on()`.

```typescript
type GunCallbackOn<V, K> = (
  data: GunDataNode<V>,
  key: K,
  message: GunHookMessagePut,
  event: IGunOnEvent
) => void;
```

- **data** -- The current value of the node, including metadata
- **key** -- The key of the node in its parent
- **message** -- The raw GUN message that triggered the update
- **event** -- An object with an `off()` method to unsubscribe

### GunCallbackOnce\<V, K\>

Called exactly once when data is available. This is the callback signature for `.once()`.

```typescript
type GunCallbackOnce<V, K> = (
  data: GunDataNode<V>,
  key: K
) => void;
```

Unlike `.on()`, this does not provide an event object because there is no subscription to cancel.

### GunCallbackGet\<N, K\>

Used internally for get operations. Returns the key and current value.

```typescript
// Callback receives:
{ get: K, put: GunDataNode<V> }
```

### GunCallbackMap\<V, K, N\>

Used with `.map()` for transforming child nodes. The callback receives each child node's data and key, and can return a transformed value or `undefined` to filter the item out.

### IGunOnEvent

The event handle returned to `.on()` callbacks. Provides the ability to cancel the subscription.

```typescript
interface IGunOnEvent {
  off(): void;
}
```

Call `event.off()` inside the callback to stop receiving further updates.

---

## Option Types

These types parameterize the behavior of specific chain methods.

### GunOptionsOn

Options for the `.on()` method.

```typescript
type GunOptionsOn = Partial<{ change: boolean } | boolean>;
```

When `change` is `true` (or the option is simply `true`), only the changed properties are included in the callback data rather than the full node. This is useful for reducing noise when monitoring a node with many properties.

### GunOptionsOnce

Options for the `.once()` method.

```typescript
type GunOptionsOnce = Partial<{ wait: number }>;
```

- **wait** -- Milliseconds to wait for data before the callback fires. If the data is not available within the wait period, the callback fires with `undefined`. This is useful for implementing timeouts on reads.

### GunOptionsPut

Options for the `.put()` method.

```typescript
type GunOptionsPut = Partial<{ opt: { cert: string } }>;
```

- **cert** -- A SEA certificate string authorizing the write. Required when writing to another user's graph under their permission policy.

---

## Network Types

### GunPeer

Represents a connected peer in GUN's mesh network.

```typescript
type GunPeer = {
  id: string;
  url: string;
  queue: string[];
  wire: null | WebSocket | RTCDataChannel;
};
```

- **id** -- Unique identifier for the peer
- **url** -- The peer's WebSocket URL
- **queue** -- Messages waiting to be sent to this peer
- **wire** -- The underlying transport connection. `null` when disconnected, a `WebSocket` for server-based peers, or an `RTCDataChannel` for browser-to-browser WebRTC connections.

---

## Hook Types

### IGunInstanceHookHandler

GUN exposes an internal hook/middleware system via `.on()` at the instance level. Hooks intercept messages flowing through GUN's internal pipeline.

Available events:

| Event | Description |
|-------|-------------|
| `'create'` | Instance created |
| `'put'` | Data write received |
| `'get'` | Data read requested |
| `'out'` | Outgoing message |
| `'in'` | Incoming message |
| `'hi'` | Peer connected |
| `'bye'` | Peer disconnected |

**WARNING for `'out'`**: Never use arrow functions as hook handlers. Arrow functions capture the surrounding `this` context, but GUN hooks rely on `this` being bound to the hook context. You must always call `this.to.next(message)` to pass the message to the next handler in the chain.

```typescript
// CORRECT: regular function
gun.on('out', function (message) {
  // modify or inspect message
  this.to.next(message);
});

// WRONG: arrow function loses this context
gun.on('out', (message) => {
  this.to.next(message); // TypeError: Cannot read property 'next' of undefined
});
```

---

## SEA Types

SEA (Security, Encryption, Authorization) provides cryptographic operations for GUN. These types define the SEA API surface.

### ISEA

The SEA module interface, available as `Gun.SEA` or `SEA` when imported separately.

```typescript
interface ISEA {
  err?: string;
  work(data, pair?, callback?, options?): Promise<string | undefined>;
  pair(callback?): Promise<ISEAPair>;
  sign(data, pair): Promise<string>;
  verify<T>(message, pair): Promise<T>;
  encrypt(data, pair_or_passphrase): Promise<string>;
  decrypt<T>(message, pair_or_passphrase): Promise<T>;
  secret(key, pair, callback?): Promise<string | undefined>;
  certify(who, policy, authority, callback?, options?): Promise<string>;
}
```

- **work** -- Proof of work / key derivation (PBKDF2). Used for password hashing.
- **pair** -- Generate a new cryptographic key pair (ECDSA signing + ECDH encryption).
- **sign** -- Sign data with a key pair (ECDSA). Returns the signed message.
- **verify** -- Verify a signed message. Returns the original data if valid.
- **encrypt** -- Encrypt data with a key pair or passphrase (AES-GCM).
- **decrypt** -- Decrypt an encrypted message.
- **secret** -- Derive a shared secret between two key pairs (ECDH).
- **certify** -- Create a certificate granting write permissions to other users.

### ISEAPair

A cryptographic key pair generated by `SEA.pair()`. Contains both signing (ECDSA) and encryption (ECDH) key pairs.

```typescript
interface ISEAPair {
  epriv: string;  // private encryption key (ECDH)
  epub: string;   // public encryption key (ECDH)
  priv: string;   // private signing key (ECDSA)
  pub: string;    // public signing key (ECDSA)
}
```

The `pub` key serves as the user's identity in GUN's user system. The `epub` key is used for end-to-end encrypted communication between users via `SEA.secret()`.

### IGunUserInstance

Extends `IGunInstanceRoot` with user authentication methods. This is the interface returned by `gun.user()`.

Key methods:

- **create(alias, pass, callback?)** -- Create a new user account
- **auth(alias, pass, callback?, options?)** -- Authenticate with alias and password
- **auth(pair, callback?, options?)** -- Authenticate with a key pair directly
- **is** -- Returns the current user's public data if authenticated, or `undefined`
- **leave()** -- Log out the current user
- **recall(options?, callback?)** -- Restore a previous session

### GunUser

The public-facing user data available after authentication.

```typescript
type GunUser = {
  alias: string;  // username
  epub: string;   // public encryption key
  pub: string;    // public signing key (user identity)
};
```

---

## Certificate and Policy Types

SEA certificates allow users to grant scoped write permissions to other users. These types define the policy language.

### Policy / IPolicy

Policies control what a certificate holder is allowed to write and where.

```typescript
type Policy = string | IPolicy | (string | IPolicy)[];

interface IPolicy extends LEX {
  '#'?: LEX;   // path -- constrains which graph paths can be written to
  '.'?: LEX;   // key -- constrains which property keys can be written
  '+'?: '*';   // require the certificate's pub key in the path
}
```

Policies use LEX expressions to define constraints:

- A simple string policy allows writing to that exact path
- The `'#'` field constrains the path in the graph where writes are allowed
- The `'.'` field constrains which keys within a node can be written
- The `'+'` field set to `'*'` requires that the certificate holder's public key appears in the write path, preventing one user from impersonating another

Policies can be combined in an array for granting multiple permissions in a single certificate.

### OptionsUserAuth

Options for the `user.auth()` method.

```typescript
type OptionsUserAuth = {
  change: string;  // new password (triggers password change)
};
```

When `change` is provided, the authentication also updates the user's password to the new value.

### OptionsUserRecall

Options for the `user.recall()` method.

```typescript
type OptionsUserRecall = {
  sessionStorage: boolean;  // use sessionStorage for session persistence
};
```

When `sessionStorage` is `true`, the session token is stored in the browser's `sessionStorage` instead of `localStorage`, meaning the session expires when the browser tab is closed.
