# GUN Quickstart Guide

Get productive with GUN in 15 minutes. This guide covers installation, running a
relay server, reading and writing data, realtime subscriptions, user
authentication, and the most common pitfalls.

---

## Table of Contents

- [Installation](#installation)
- [Import Patterns](#import-patterns)
- [Creating an Instance](#creating-an-instance)
- [Running a Relay Server](#running-a-relay-server)
- [CRUD Operations](#crud-operations)
- [Working with Sets (Lists)](#working-with-sets-lists)
- [Graph Traversal](#graph-traversal)
- [Realtime Patterns](#realtime-patterns)
- [Async / Await](#async--await)
- [User Authentication and Data](#user-authentication-and-data)
- [Multi-Peer Setup](#multi-peer-setup)
- [Common Gotchas](#common-gotchas)
- [Deployment Options](#deployment-options)
- [Next Steps](#next-steps)

---

## Installation

### npm / yarn / pnpm

```bash
npm install gun
# or
yarn add gun
# or
pnpm add gun
```

### CDN (Browser)

```html
<script src="https://cdn.jsdelivr.net/npm/gun/gun.js"></script>
```

If you need the user/auth system, add SEA right after:

```html
<script src="https://cdn.jsdelivr.net/npm/gun/gun.js"></script>
<script src="https://cdn.jsdelivr.net/npm/gun/sea.js"></script>
```

---

## Import Patterns

GUN supports every mainstream module system. Pick the one that matches your
environment.

### Browser script tag

When loaded via `<script>`, the constructor is available as a global:

```html
<script src="https://cdn.jsdelivr.net/npm/gun/gun.js"></script>
<script>
  // GUN and Gun are both available as globals
  var gun = Gun();
</script>
```

### ESM (ES Modules)

```javascript
import GUN from 'gun';

var gun = GUN();
```

### CommonJS (Node.js)

```javascript
const GUN = require('gun');

var gun = GUN();
```

### React / Next.js

React bundlers sometimes struggle with GUN's auto-detection of environments.
Import from `gun/gun` to get the browser-only build, then pull in extensions
explicitly:

```javascript
import GUN from 'gun/gun';
import 'gun/sea';
import 'gun/lib/radix.js';
import 'gun/lib/radisk.js';
import 'gun/lib/store.js';
import 'gun/lib/rindexed.js';

var gun = GUN({ peers: ['https://your-relay.example.com/gun'] });
```

### React Native

React Native does not have `localStorage` or `WebSocket` built in. You need
polyfills before importing GUN:

```javascript
// Install these first:
//   npm install text-encoding @peculiar/webcrypto react-native-get-random-values

import 'text-encoding';
import 'react-native-get-random-values';
import { WebSocket } from 'react-native'; // built-in in RN, but may need explicit import

import GUN from 'gun/gun';
import 'gun/sea';

var gun = GUN({ peers: ['https://your-relay.example.com/gun'] });
```

> **Tip:** If you see errors about `crypto.getRandomValues`, make sure
> `react-native-get-random-values` is imported *before* GUN/SEA.

---

## Creating an Instance

```javascript
// 1. Local only — no networking, data stays in this tab/process
var gun = Gun();

// 2. Connect to a single relay peer
var gun = Gun('https://yourdomain.com/gun');

// 3. Connect to multiple relay peers (see "Multi-Peer Setup" below)
var gun = Gun(['https://server1.com/gun', 'https://server2.com/gun']);

// 4. Full options object
var gun = Gun({
  peers: ['https://server1.com/gun'],
  localStorage: true,   // use browser localStorage (default true in browsers)
  radisk: true,          // use RAD on-disk storage (default true in Node)
  file: 'my-data-dir'   // Node.js: directory name for RAD files (default "radata")
});
```

The string shorthand (`Gun('url')`) is equivalent to `Gun({ peers: ['url'] })`.

---

## Running a Relay Server

A relay server lets browsers and other nodes discover each other and sync data.
GUN attaches to a standard Node.js `http` or `https` server.

### Minimal HTTP relay

```javascript
const Gun = require('gun');
const http = require('http');

var server = http.createServer(function (req, res) {
  // GUN handles its own requests; this handler is for anything else
  if (Gun.serve(req, res)) { return; }
  res.writeHead(200);
  res.end('GUN relay is running');
});

server.listen(8765, function () {
  console.log('Relay listening on http://localhost:8765/gun');
});

// Attach GUN to the HTTP server
var gun = Gun({ web: server });
```

Clients connect with:

```javascript
var gun = Gun('http://localhost:8765/gun');
```

### Express relay

```javascript
const Gun = require('gun');
const express = require('express');

var app = express();
var port = process.env.PORT || 8765;

app.use(Gun.serve);
app.get('/', function (req, res) {
  res.status(200).send('GUN relay is running');
});

var server = app.listen(port, function () {
  console.log('Relay listening on port', port);
});

var gun = Gun({ web: server });
```

### HTTPS relay with certificates

For production, serve over TLS. Set `HTTPS_CERT` and `HTTPS_KEY` environment
variables to the paths of your certificate and private key:

```javascript
const Gun = require('gun');
const https = require('https');
const fs = require('fs');

var certPath = process.env.HTTPS_CERT || '/etc/letsencrypt/live/example.com/fullchain.pem';
var keyPath  = process.env.HTTPS_KEY  || '/etc/letsencrypt/live/example.com/privkey.pem';
var port     = process.env.PORT       || 443;

var options = {
  cert: fs.readFileSync(certPath),
  key:  fs.readFileSync(keyPath)
};

var server = https.createServer(options, function (req, res) {
  if (Gun.serve(req, res)) { return; }
  res.writeHead(200);
  res.end('GUN relay (HTTPS) is running');
});

server.listen(port, function () {
  console.log('Relay listening on https://localhost:' + port + '/gun');
});

var gun = Gun({ web: server });
```

> **Environment variables summary:**
>
> | Variable | Purpose | Default |
> |----------|---------|---------|
> | `PORT` | Listen port | `8765` (HTTP) or `443` (HTTPS) |
> | `HTTPS_CERT` | Path to TLS certificate file | none |
> | `HTTPS_KEY` | Path to TLS private key file | none |

---

## CRUD Operations

GUN's data model is a graph of nodes. Each node is a flat object of key/value
pairs. You navigate the graph with `.get(key)` and store values with `.put()`.

### Create (Write)

`.put()` writes data into the graph. If the node does not exist, it is created.
If it does exist, the properties you supply are merged in (existing properties
you do not mention are left alone).

```javascript
// Write an entire node
gun.get('mark').put({
  name: 'Mark',
  email: 'mark@gun.eco',
  age: 32
});

// Write a single property on an existing node
gun.get('mark').get('company').put('GUN');
```

`.put()` returns the same chain, so you can keep chaining:

```javascript
gun.get('mark').put({ name: 'Mark' }).get('email').put('mark@gun.eco');
```

### Read (once)

`.once()` reads the current value and fires once. It does *not* subscribe to
future changes.

```javascript
gun.get('mark').once(function (data, key) {
  console.log(key, data);
  // "mark" { name: "Mark", email: "mark@gun.eco", age: 32, _ {...} }
  // Note: `_` is internal metadata — ignore it in application code
});

// Read a single property
gun.get('mark').get('name').once(function (data) {
  console.log(data); // "Mark"
});
```

### Update

Updating is the same as writing. `.put()` merges new properties into the
existing node. Properties you omit stay unchanged.

```javascript
// Only the 'age' property changes; 'name' and 'email' are untouched
gun.get('mark').put({ age: 33 });
```

To update a single field:

```javascript
gun.get('mark').get('age').put(33);
```

### Delete

GUN is an append-only, CRDT-based system. You cannot truly delete data in the
distributed graph (other peers may still hold it). However, you can **null out**
a property, which is the conventional way to "delete":

```javascript
// Null out a single property
gun.get('mark').get('email').put(null);

// Null out all properties on a node
gun.get('mark').put({
  name: null,
  email: null,
  age: null
});
```

Reading a nulled-out property returns `null` (or `undefined` depending on
context), which your application treats as "deleted."

---

## Working with Sets (Lists)

`.set()` adds a node as a member of an unordered set. Under the hood it creates
a unique key (a soul) pointing to the node.

```javascript
var alice = gun.get('alice').put({ name: 'Alice', role: 'engineer' });
var bob   = gun.get('bob').put({ name: 'Bob', role: 'designer' });

var team = gun.get('team');
team.set(alice);
team.set(bob);

// Iterate all members
team.map().once(function (member, id) {
  console.log(id, member.name);
});
```

> **Why `.set()` and not `.put()` into an array?** GUN's graph does not support
> arrays. `.set()` stores each item under a unique auto-generated key, which
> makes concurrent additions from different peers merge cleanly via CRDTs.

---

## Graph Traversal

GUN stores data as a flat graph where nodes reference each other. You traverse
references by chaining `.get()` calls.

### Walking references

```javascript
// Create linked data
gun.get('mark').put({
  name: 'Mark',
  boss: gun.get('alice').put({ name: 'Alice' })
});

// Walk the reference: mark -> boss -> name
gun.get('mark').get('boss').get('name').once(function (name) {
  console.log("Mark's boss is", name); // "Alice"
});
```

### Following links between nodes

You can create arbitrary links between nodes. Each `.get().put()` pointing to
another `.get()` creates a graph edge:

```javascript
// Build a small social graph
var alice = gun.get('alice').put({ name: 'Alice' });
var bob   = gun.get('bob').put({ name: 'Bob' });
var carol = gun.get('carol').put({ name: 'Carol' });

// Alice follows Bob and Carol
gun.get('alice').get('follows').get('bob').put(bob);
gun.get('alice').get('follows').get('carol').put(carol);

// Bob follows Alice
gun.get('bob').get('follows').get('alice').put(alice);

// Traverse: who does Alice follow?
gun.get('alice').get('follows').map().once(function (person, key) {
  console.log('Alice follows', person.name);
});
```

### Circular references

GUN handles cycles naturally because it stores references (souls), not copies:

```javascript
var cat  = gun.get('fluffy').put({ name: 'Fluffy', species: 'cat' });
var mark = gun.get('mark').put({ name: 'Mark', pet: cat });
cat.get('owner').put(mark); // circular: mark -> pet -> owner -> mark

gun.get('mark').get('pet').get('owner').get('name').once(function (name) {
  console.log(name); // "Mark" — no infinite loop
});
```

---

## Realtime Patterns

GUN provides four subscription patterns. Choosing the right one avoids
confusion and unnecessary re-renders.

### `.on()` — continuous subscription

Fires every time the data changes, indefinitely. Use this for live UI updates.

```javascript
gun.get('score').on(function (data, key) {
  console.log('Score updated:', data);
});
// Fires now with current value, then again every time `score` changes.
```

To stop listening, save the return and call `.off()`:

```javascript
var listener = gun.get('score').on(function (data) {
  console.log(data);
});

// Later: stop listening
listener.off();
```

### `.once()` — read once

Fires a single time with the current value and then stops listening. Use this
when you want a one-shot read (like a REST GET).

```javascript
gun.get('score').once(function (data) {
  console.log('Current score:', data);
});
// Will NOT fire again if the score changes later.
```

> **Debounce note:** `.once()` waits a few milliseconds for data to arrive from
> peers before firing. This is by design — if data is coming from a remote relay,
> it needs a moment. See [Common Gotchas](#once-debounce-behavior).

### `.map().on()` — live subscription on a set

Fires for every existing member of a set, *and* fires again whenever a member
is added or any member's data changes. Use this for real-time lists.

```javascript
gun.get('chat').map().on(function (msg, id) {
  // Fires for each existing message, plus every new message in real time
  console.log(id, msg.text);
});
```

### `.map().once()` — snapshot of a set

Fires once per existing member, then stops. New members added later will *not*
trigger the callback. Use this for loading an initial list without subscribing
to changes.

```javascript
gun.get('chat').map().once(function (msg, id) {
  // Fires once per existing message — no further updates
  console.log(id, msg.text);
});
```

### Quick comparison

| Pattern | Fires for existing data | Fires on updates | Fires for new set members |
|---------|------------------------|-------------------|--------------------------|
| `.on()` | Yes | Yes | N/A (single node) |
| `.once()` | Yes | No | N/A (single node) |
| `.map().on()` | Yes | Yes | Yes |
| `.map().once()` | Yes | No | No |

---

## Async / Await

GUN is callback-based by default. The `gun/lib/then` extension adds
`.then()` and `.promise()` so you can use `async`/`await`.

```javascript
// Load the extension
require('gun/lib/then');
// or in the browser:
// <script src="https://cdn.jsdelivr.net/npm/gun/lib/then.js"></script>
```

### `.then()` — returns a Promise

```javascript
var data = await gun.get('mark').then();
console.log(data); // { name: "Mark", email: "mark@gun.eco", ... }

var name = await gun.get('mark').get('name').then();
console.log(name); // "Mark"
```

### `.promise()` — returns `{ put, get, gun }`

`.promise()` gives you more context than `.then()`:

```javascript
var result = await gun.get('mark').promise();
console.log(result.put);  // the data
console.log(result.get);  // the key ("mark")
console.log(result.gun);  // the gun chain reference
```

### Practical example: async function

```javascript
require('gun/lib/then');

async function getProfile(username) {
  var profile = await gun.get('profiles').get(username).then();
  if (!profile) {
    console.log(username, 'not found');
    return null;
  }
  console.log('Found:', profile.name, profile.email);
  return profile;
}

getProfile('mark');
```

> **Caveat:** `.then()` and `.promise()` resolve with the first value they
> receive. They do not re-resolve on updates. For realtime data, stick with
> `.on()`.

---

## User Authentication and Data

GUN's user system (SEA) gives each user a cryptographic key pair. User data
lives in a namespace under their public key, so no one else can overwrite it.

### Setup

```javascript
const GUN = require('gun');
require('gun/sea');

var gun = Gun({ peers: ['https://your-relay.example.com/gun'] });
var user = gun.user();
```

### Register a new user

```javascript
user.create('alice', 'passphrase123', function (ack) {
  if (ack.err) {
    console.error('Registration failed:', ack.err);
    return;
  }
  console.log('User created. Public key:', ack.pub);
});
```

### Log in

```javascript
user.auth('alice', 'passphrase123', function (ack) {
  if (ack.err) {
    console.error('Login failed:', ack.err);
    return;
  }
  console.log('Logged in as', ack.sea.pub);
});
```

### Write data to the authenticated user's graph

After login, `user.get(...)` writes to the user's private namespace (keyed by
their public key). No other user can modify this data.

```javascript
user.auth('alice', 'passphrase123', function (ack) {
  if (ack.err) { return; }

  // Write to alice's user space
  user.get('profile').put({
    name: 'Alice',
    bio: 'Decentralization enthusiast',
    joined: Date.now()
  });

  // Write a collection
  user.get('posts').set({
    title: 'Hello World',
    body: 'My first GUN post!',
    ts: Date.now()
  });
});
```

### Read another user's public data

Anyone can read another user's data if they know the public key. Use
`gun.user(pubKey)`:

```javascript
var alicePub = 'the-public-key-from-registration';

// Read alice's profile (no login needed)
gun.user(alicePub).get('profile').once(function (profile) {
  console.log(profile.name, '-', profile.bio);
});

// List alice's posts
gun.user(alicePub).get('posts').map().once(function (post) {
  console.log(post.title, post.body);
});
```

### Full registration + login + read flow

```javascript
const GUN = require('gun');
require('gun/sea');

var gun  = Gun({ peers: ['https://relay.example.com/gun'] });
var user = gun.user();

// 1. Register
user.create('bob', 'strongpassword', function (ack) {
  if (ack.err) { console.error(ack.err); return; }

  // 2. Log in (create does not auto-login in all versions)
  user.auth('bob', 'strongpassword', function (ack) {
    if (ack.err) { console.error(ack.err); return; }

    var pubKey = ack.sea.pub;
    console.log('Bob logged in. Public key:', pubKey);

    // 3. Write to bob's user graph
    user.get('profile').put({ name: 'Bob', role: 'admin' });

    // 4. Read back bob's data (as anyone, via public key)
    gun.user(pubKey).get('profile').once(function (data) {
      console.log('Public read:', data.name, data.role);
    });
  });
});
```

---

## Multi-Peer Setup

Connecting to multiple relays improves availability. If one relay goes offline,
clients stay synced through the others.

### Connect to multiple peers at initialization

```javascript
var gun = Gun({
  peers: [
    'https://relay1.example.com/gun',
    'https://relay2.example.com/gun',
    'https://relay3.example.com/gun'
  ]
});
```

### Add or remove peers at runtime

```javascript
// Add a peer after initialization
gun.opt({ peers: ['https://relay4.example.com/gun'] });

// Mesh status — inspect connected peers (internal, useful for debugging)
console.log(gun._.opt.peers);
```

### Reconnection behavior

GUN automatically attempts to reconnect to peers that drop. You do not need
to write reconnection logic. If a relay goes offline and comes back, GUN
will re-establish the WebSocket and sync any missed data through its CRDT
conflict resolution (HAM algorithm).

For advanced control, you can listen for peer connection events:

```javascript
gun.on('hi',  function (peer) { console.log('Peer connected:',    peer); });
gun.on('bye', function (peer) { console.log('Peer disconnected:', peer); });
```

---

## Common Gotchas

These are the mistakes that trip up nearly every new GUN developer.

### You cannot `.put()` primitives at the root

GUN's root is the entry point to the graph. You must `.get(key)` a node first,
then `.put()`:

```javascript
// WRONG — this will not work
gun.put('hello');

// RIGHT — navigate to a node first
gun.get('greeting').put({ text: 'hello' });
// or for a single value:
gun.get('greeting').get('text').put('hello');
```

### `.once()` debounce behavior

`.once()` does not fire instantly. It waits a short time (~50-100ms by default)
for data from peers to arrive so you get the most up-to-date value. If you call
`.once()` immediately after `.put()`, the data may not have propagated yet.

```javascript
gun.get('counter').put({ n: 1 });

// This MAY log undefined if the internal write has not settled
gun.get('counter').once(function (data) {
  console.log(data); // might be undefined briefly
});

// Safer: use .on() which fires whenever data arrives
gun.get('counter').on(function (data) {
  console.log(data.n); // 1
});
```

### Nested objects get flattened into separate nodes

When you `.put()` a nested object, GUN does **not** store it as a single
document. Each nested object is broken into its own node with an auto-generated
soul (UUID). This means the inner object is a *reference*, not an inline value.

```javascript
gun.get('mark').put({
  name: 'Mark',
  address: {         // this becomes a separate node!
    city: 'Seattle',
    zip: '98101'
  }
});

// The address is stored under a generated key like "T0abc123..."
// You traverse it the same way — GUN handles the dereferencing:
gun.get('mark').get('address').get('city').once(function (city) {
  console.log(city); // "Seattle"
});
```

This is important because if you `.put()` the same parent object again later,
the nested object gets a **new** soul — effectively creating a new node rather
than updating the old one. To update a nested value, traverse to it directly:

```javascript
// WRONG — creates a new address node, orphaning the old one
gun.get('mark').put({ address: { city: 'Portland' } });

// RIGHT — update the existing address node in place
gun.get('mark').get('address').put({ city: 'Portland' });
```

### GUN data is one layer deep by default

`.once()` and `.on()` only return the immediate properties of a node. Nested
references appear as `{ "#": "soul-id" }` stubs, not resolved objects.

```javascript
gun.get('mark').once(function (data) {
  console.log(data.address);
  // { "#": "T0abc123..." } — a reference stub, NOT the address data
});

// To get the nested data, chain .get():
gun.get('mark').get('address').once(function (addr) {
  console.log(addr.city); // "Seattle"
});
```

If you need the full document tree in one callback, use the `open` extension:

```javascript
require('gun/lib/open');

gun.get('mark').open(function (data) {
  console.log(data.address.city); // "Seattle" — fully resolved
});
```

### `.off()` to prevent memory leaks

Every `.on()` creates a persistent listener. In single-page apps or long-lived
processes, always clean up when a component unmounts or a subscription is no
longer needed:

```javascript
var ref = gun.get('status').on(function (data) {
  updateUI(data);
});

// When done (e.g., component unmount):
ref.off();
```

---

## Deployment Options

### Heroku

GUN works on Heroku with minimal configuration. Create a `server.js` as shown
in the [Express relay](#express-relay) section, add a `Procfile`:

```
web: node server.js
```

Set `PORT` via Heroku config (Heroku sets it automatically). Note that Heroku's
ephemeral filesystem means RAD data is lost on dyno restart; use S3 or another
persistent storage adapter for production.

### Docker

```dockerfile
FROM node:20-alpine
WORKDIR /app
COPY package*.json ./
RUN npm install
COPY server.js .
EXPOSE 8765
CMD ["node", "server.js"]
```

```bash
docker build -t gun-relay .
docker run -d -p 8765:8765 -v gun-data:/app/radata gun-relay
```

The `-v gun-data:/app/radata` volume mount ensures RAD data persists across
container restarts.

### Linux with systemd

Create `/etc/systemd/system/gun-relay.service`:

```ini
[Unit]
Description=GUN Relay Server
After=network.target

[Service]
Type=simple
User=gun
WorkingDirectory=/opt/gun-relay
ExecStart=/usr/bin/node server.js
Restart=on-failure
Environment=PORT=8765

[Install]
WantedBy=multi-user.target
```

Then:

```bash
sudo systemctl daemon-reload
sudo systemctl enable gun-relay
sudo systemctl start gun-relay
```

---

## Adding Extended APIs

GUN ships with optional extensions that are *not* loaded by default. Require
only the ones you need:

```javascript
// Node.js / bundler
require('gun/lib/path.js');   // gun.path()
require('gun/lib/not.js');    // gun.not()
require('gun/lib/open.js');   // gun.open()  — resolve nested data
require('gun/lib/load.js');   // gun.load()  — like open for sets
require('gun/lib/then.js');   // gun.then() / gun.promise()
require('gun/lib/bye.js');    // gun.bye()   — cleanup on disconnect
require('gun/lib/later.js');  // gun.later() — delayed operations
require('gun/lib/unset.js');  // gun.unset() — remove from set
```

In the browser, include them as additional script tags:

```html
<script src="https://cdn.jsdelivr.net/npm/gun/lib/open.js"></script>
<script src="https://cdn.jsdelivr.net/npm/gun/lib/then.js"></script>
```

---

## Key Concepts

A brief summary of how GUN works under the hood:

- **Graph database** — data is a graph of nodes connected by references, not a
  document store or relational tables.
- **Realtime by default** — `.on()` gives you live updates as data changes
  anywhere in the network.
- **Offline-first** — data persists locally (localStorage, IndexedDB, or
  filesystem). Syncs automatically when a connection is available.
- **Decentralized** — no single server required. Any peer can relay data to any
  other peer.
- **Conflict resolution** — uses the HAM (Hypothetical Amnesia Machine) CRDT
  algorithm. Concurrent writes from different peers are resolved
  deterministically without coordination.
- **~9 KB gzipped** — the core library is extremely lightweight.

### Where data lives

| Environment | Default storage |
|-------------|----------------|
| Browser | localStorage + IndexedDB (via RAD) |
| Node.js | Filesystem in `radata/` directory (via RAD) |
| All peers | Synced in realtime across every connected peer |

---

## Next Steps

Now that you have the basics, dive deeper with the other reference docs in this
folder:

- **[Core API](core-api.md)** — complete reference for `Gun()`, `.get()`,
  `.put()`, `.on()`, `.once()`, `.set()`, `.map()`, `.back()`, and `.off()`.
- **[SEA (Security, Encryption, Authorization)](sea.md)** — cryptographic
  signing, encryption, certificates, and the `SEA.pair()` / `SEA.sign()` /
  `SEA.verify()` / `SEA.encrypt()` / `SEA.decrypt()` API.
- **[User API](user-api.md)** — `gun.user()`, `user.create()`, `user.auth()`,
  `user.leave()`, and user-scoped data storage.
- **[Extended API](extended-api.md)** — plugin methods like `.path()`,
  `.open()`, `.load()`, `.not()`, `.then()`, `.bye()`, and `.unset()`.
- **[RAD (Radix Storage Engine)](rad.md)** — how GUN persists data to disk,
  S3, IndexedDB, and custom storage adapters.
- **[Utilities](utilities.md)** — `Gun.node.is()`, `Gun.node.soul()`, and
  other helper functions for inspecting and manipulating graph nodes.
- **[TypeScript Types](typescript-types.md)** — full type reference for GUN's
  TypeScript definitions (`IGun`, `IGunChain`, `ISEA`, etc.).
