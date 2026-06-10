# GUN Networking -- AXE and DAM

## Overview

GUN's networking layer is built on two complementary systems:

- **DAM (Daisy-chain Ad-hoc Mesh)** -- GUN's core message routing protocol. DAM handles serialization, deduplication, batching, peer handshakes, and relay logic. It is the transport-agnostic "mesh" that all messages flow through regardless of the underlying wire (WebSocket, WebRTC, UDP multicast).

- **AXE (Automatic eXchange Equilibrium)** -- An optional higher-level layer that provides automatic peer discovery, DHT-based clustering, subscription routing, and mob rebalancing. AXE sits on top of DAM and optimizes which peers receive which messages.

Both are implemented as GUN middleware plugins that hook into the `Gun.on('opt', ...)` lifecycle.

**Source files:**

| File | Role |
|------|------|
| `gun/src/mesh.js` | DAM core -- the `Gun.Mesh` factory |
| `gun/src/websocket.js` | Browser-side WebSocket transport (bundled into `gun.js`) |
| `gun/axe.js` | AXE entry point (browser side) |
| `gun/lib/axe.js` | AXE relay/server logic (Node.js side) |
| `gun/lib/wire.js` | Server-side WebSocket transport |
| `gun/lib/ws.js` | Legacy WebSocket transport |
| `gun/lib/webrtc.js` | WebRTC peer-to-peer transport |
| `gun/lib/multicast.js` | LAN UDP multicast discovery |

---

## DAM (Daisy-chain Ad-hoc Mesh)

### What It Is

DAM is GUN's default transport layer abstraction and P2P networking algorithm. Every GUN message -- PUTs, GETs, and ACKs -- flows through DAM. It prevents infinite broadcast loops via message ID deduplication, optimizes relay paths by tagging neighbor peer IDs onto messages, and batches outgoing messages for performance.

DAM is to networking what RAD is to storage: a pluggable abstraction layer. You can swap transports (WebSocket, WebRTC, UDP) the same way RAD lets you swap storage backends.

### The Mesh Object (`Gun.Mesh`)

`Gun.Mesh(root)` is a factory function (defined in `src/mesh.js`) that returns a `mesh` object. It is instantiated once per GUN instance:

```javascript
var mesh = opt.mesh = opt.mesh || Gun.Mesh(root);
```

The mesh object exposes the following API:

| Method | Purpose |
|--------|---------|
| `mesh.hear(raw, peer)` | Process an incoming message |
| `mesh.say(msg, peer)` | Send an outgoing message |
| `mesh.hi(peer)` | Register/connect a peer |
| `mesh.bye(peer)` | Disconnect a peer |
| `mesh.wire(peer)` | Open a transport connection (set by transport plugins) |
| `mesh.hash(msg, peer)` | Compute content hash for deduplication |
| `mesh.raw(msg, peer)` | Serialize a message to JSON |
| `mesh.near` | Count of currently connected peers |

### `mesh.hear(raw, peer)` -- Incoming Messages

`hear` is the entry point for all incoming data. It handles:

1. **Size check** -- Rejects messages exceeding `opt.max` (default ~90MB, 30% of 300MB).
2. **Batch parsing** -- If the raw data starts with `[`, it is parsed as a JSON array and each element is processed individually, with CPU yielding every `opt.puff` (default 9) messages.
3. **Single message parsing** -- If the raw data starts with `{` or is already an object, it is processed as one message.
4. **Deduplication** -- Each message must have a `#` (message ID). If the ID has already been seen (tracked by `dup`), the message is silently dropped.
5. **Hash deduplication** -- If a message carries `##` (content hash), and the same ack+hash combo was already seen, it is dropped. This prevents redundant data replies.
6. **Peer exclusion tracking** -- The `><` field lists peer IDs that have already received this message. DAM parses this into a `yo` map on the message metadata so `say` can skip those peers.
7. **DAM protocol dispatch** -- If `msg.dam` is set, the message is routed to a registered handler (`mesh.hear[dam_value]`) instead of being processed as a GUN data message. This is how peer handshakes (`?`), errors (`!`), mob rebalancing (`mob`), and custom protocols work.
8. **GUN core dispatch** -- Non-DAM messages are emitted as `root.on('in', msg)`.

```javascript
// Simplified flow of mesh.hear:
function hear(raw, peer) {
  if (raw starts with '[') { parse as batch, process each }
  if (raw starts with '{') { parse as single message }

  // hear.one(msg, peer):
  if (dup.check(msg['#']))   { return }         // already seen
  if (dup.check(ack + hash)) { return }         // hash dedup
  msg._.via = peer;                              // track origin
  parse msg['><'] into msg._.yo exclusion map;

  if (msg.dam) { mesh.hear[msg.dam](msg, peer); return }  // protocol
  root.on('in', msg);                            // data message
  dup.track(msg['#']);
}
```

### `mesh.say(msg, peer)` -- Outgoing Messages

`say` is the entry point for all outgoing data. It handles:

1. **Message ID** -- Ensures every message has a `#` field (generates `String.random(9)` if missing).
2. **Hashing** -- If the message has a `put` payload, no hash (`##`), and is an ACK (`@`), DAM computes a JSON hash before sending. This enables downstream hash-based deduplication.
3. **ACK routing** -- If no explicit peer is given and the message is an ACK (`@`), DAM traces back through `dup.s` to find the original sender (`via`) and routes the ACK directly to them. This avoids flooding ACKs to all peers.
4. **`mesh.way` fallback** -- If no peer is determined and `mesh.way` is set (by AXE), the message is delegated to AXE's routing logic.
5. **Peer exclusion** -- The `><` field is populated with the IDs/URLs of up to 7 connected peers, so downstream relays know who already received the message.
6. **Serialization** -- `mesh.raw(msg)` converts the message to a JSON string, cached on `msg._.raw` for reuse.
7. **Self-send prevention** -- Messages are not sent back to the peer they came from (`peer === meta.via`), nor to peers listed in the `><` exclusion map.
8. **Batching** -- Messages are batched into JSON arrays (`[msg1,msg2,...]`) per peer. A `setTimeout` of `opt.gap` milliseconds (default 0) flushes the batch. This reduces the number of wire sends.
9. **Broadcast** -- If `peer` is not a single peer but a peer map (or not specified), DAM iterates all peers in `opt.peers` and sends to each (with dedup/exclusion checks).

```javascript
// Sending to a specific peer:
mesh.say({ put: { 'soul': { 'key': 'value' } } }, peer);

// Broadcast to all peers (AXE or default):
mesh.say(msg);  // routes through mesh.way or opt.peers

// Acknowledging a GET:
mesh.say({ '@': originalMsgId, put: data });
```

### `mesh.hi(peer)` -- Connect to Peer

Registers a peer with the mesh. Behavior depends on what is passed:

- **String (URL)**: Wraps it as `{url: peer, id: peer}` and calls `mesh.wire(peer)` to open a transport connection.
- **Peer object without `wire`**: Calls `mesh.wire(peer)` to establish the connection.
- **Peer object with `wire`**: Registers the peer directly.

On first connection, `mesh.hi`:
1. Assigns an `id` to the peer (from `peer.url` or random).
2. Sends a DAM handshake: `{dam: '?', pid: root.opt.pid}` -- this exchanges process IDs so peers can identify each other and detect self-connections.
3. Increments `mesh.near` (connected peer count).
4. Emits `root.on('hi', peer)`.
5. Flushes any queued messages that accumulated while the connection was opening.

```javascript
// Connect to a peer by URL:
mesh.hi('https://relay.example.com/gun');

// Connect a peer object (e.g., from WebSocket 'connection' event):
mesh.hi({ wire: wsConnection });
```

### `mesh.bye(peer)` -- Disconnect Peer

Disconnects a peer:
1. Decrements `mesh.near`.
2. Emits `root.on('bye', peer)`.
3. Closes the wire (`peer.wire.close()` or `peer.bye()`).
4. Removes the peer from `opt.peers`.
5. Sets `peer.wire = null`.

```javascript
mesh.bye(peer);
// or by ID:
mesh.bye(peerId);
```

### Message ID Tracking and Deduplication

DAM uses a deduplication system (`root.dup`) that maintains a time-limited, fixed-size in-memory map of message IDs. Every message carries a unique `#` field:

- When a message is **heard**, its ID is checked against `dup`. If found, the message is dropped (already processed). If not, it is tracked.
- When a message is **said**, its ID is tracked so the sender's own echo is ignored.
- IDs are "bumped" (kept alive) when re-seen, keeping active message storms deduplicated.
- IDs eventually expire and are purged, which is safe because by then the broadcast storm has died.

This prevents infinite relay loops in mesh networks where A sends to B, B relays to C, C relays back to A, etc.

**Hash deduplication** adds another layer: if a message carries `##` (a hash of its `put` data), and the combination of `ack_id + hash` has already been seen, the message is dropped. This prevents peers with identical data from sending redundant ACKs.

### Wire Serialization

All messages are serialized as JSON for transmission. DAM handles:

- **Single messages**: `{"#":"abc123","put":{...}}`
- **Batched messages**: `[{"#":"msg1",...},{"#":"msg2",...}]` -- DAM batches multiple outgoing messages into a single array per `opt.gap` timeout interval.
- **The `><` field**: A comma-separated string of peer IDs already aware of this message: `"peerA,peerB,peerC"`. Limited to the first 99 characters. Each relay strips this and replaces with its own peer list.
- **Heartbeats**: Empty arrays `[]` are sent periodically (every 20 seconds on WebSocket) to keep connections alive.

### DAM Protocol Messages

DAM uses the `dam` field for protocol-level messages that are handled by `mesh.hear[dam_value]` handlers rather than processed as GUN data:

| `dam` value | Purpose | Payload |
|-------------|---------|---------|
| `?` | Peer handshake / PID exchange | `{dam:'?', pid:'...'}` |
| `!` | Error notification | `{dam:'!', err:'...'}` |
| `mob` | Mob rebalancing (too many peers) | `{dam:'mob', mob:count, peers:{...}}` |
| `opt` | Peer introduction (ask peer to connect elsewhere) | `{dam:'opt', opt:{peers:'url'}}` |
| `hi` | Announce presence | `{dam:'hi'}` |
| `rtc` | WebRTC signaling | `{dam:'rtc', ok:{rtc:{offer/answer/candidate}}}` |

Custom protocol handlers can be registered:

```javascript
var mesh = gun.back('opt.mesh');
mesh.hear['myProtocol'] = function(msg, peer) {
  console.log('Received custom message:', msg);
  // These messages are NOT relayed to other peers.
};
mesh.say({dam: 'myProtocol', data: 'hello'}, peer);
```

DAM protocol messages are "dammed" -- they are consumed by the handler and **not** relayed to other peers, making them ideal for neighbor-only coordination.

---

## AXE (Automatic eXchange Equilibrium)

### What It Is

AXE is an optional optimization layer built on top of DAM. While DAM provides brute-force mesh networking (every message goes to every peer), AXE adds intelligent routing:

- **Subscription tracking**: AXE knows which peers are interested in which data, so PUT updates are only sent to subscribed peers instead of everyone.
- **Peer discovery**: In browsers, AXE automatically finds relay peers through multiple fallback strategies.
- **DHT clustering**: Relay peers maintain an "up" set of upstream relay connections and route GETs through subscription routes before falling back to broadcast.
- **Mob rebalancing**: When a relay has too many connections, it redirects excess peers to other relays.

### Browser-Side AXE (`axe.js`)

When `opt.axe` is not `false` and running in a browser, AXE:

1. **Initializes DAM**: `opt.mesh = opt.mesh || Gun.Mesh(root)`
2. **Adds default peers**: Tries `location.origin + '/gun'` and `http://localhost:8765/gun`
3. **Discovers fallback peers** through a cascade:
   - **(1) Local peer**: The current origin's `/gun` endpoint
   - **(2) Last used peers**: Stored in `localStorage.peers`
   - **(3) URL parameter**: `?peers=url1,url2` in the page URL
   - **(4) Hardcoded DHT list**: Fetches a peer list from `https://raw.githubusercontent.com/wiki/amark/gun/volunteer.dht.md` (or a custom URL via `?axe=url`)
4. **Handles disconnections**: When a peer disconnects (`bye` event), AXE randomly selects a replacement from the fallback list. If no fallbacks remain, it falls back to `https://gunjs.herokuapp.com/gun`.

```javascript
// AXE is enabled by default in browsers. Disable it:
var gun = Gun({ axe: false, peers: ['https://my-relay.com/gun'] });

// AXE will log:
// "AXE enabled: Trying to find network via (1) local peer (2) last used peers (3) a URL parameter, and last (4) hard coded peers."
// "Warning: AXE is in alpha, use only for testing!"
```

### Server-Side AXE (`lib/axe.js`)

On Node.js relays, AXE provides subscription-based routing:

#### `mesh.way` -- Smart Routing

AXE overrides `mesh.way` (the default broadcast behavior) with intelligent routing:

```javascript
mesh.way = function(msg) {
  if (msg.get) { return GET(msg) }  // Route GETs through subscription table
  if (msg.put) { return }           // PUTs handled by subscription push below
  fall(msg);                         // Everything else: broadcast
}
```

#### GET Routing with Subscription Turns

When a GET arrives from a peer, AXE:

1. Records the asking peer in a **subscription route** (`ref._.route` -- a Map of peer IDs to peer objects) for the requested soul.
2. Tries subscribed peers first (`GET.turn(msg, route, 0)`), sending the GET to 3 peers at a time.
3. If no subscribed peer answers (hash mismatch or timeout after 25ms), asks the next batch.
4. If all subscribed peers are exhausted, falls back to broadcasting to `opt.peers`.

#### PUT Subscription Push

On the `put` event, AXE checks each soul's subscription route. For each subscribed peer:

1. Verifies the peer is still connected (`peer.wire` exists).
2. Checks if the peer's subscription includes the specific `has` (property) or the whole soul (`''`).
3. Batches the update into `peer.put` and flushes after `opt.gap` milliseconds.
4. Tracks back-references so ACKs can be correlated to the original message.

```javascript
// Subscription data structure per peer:
// peer.sub = Map { soul -> Map { has -> 1, '' -> 1 } }
// Example: peer is subscribed to all properties of 'users/alice'
//   peer.sub.get('users/alice') => Map { '' => 1 }
// Example: peer is subscribed to 'name' property of 'users/bob'
//   peer.sub.get('users/bob') => Map { 'name' => 1 }
```

#### The UP Module -- Relay Interconnection

The UP module manages connections between relay peers:

- **Self-connection detection**: If `peer.pid === opt.pid`, the connection is dropped.
- **Duplicate connection resolution**: If two relays both connect to each other, the one with the higher PID keeps its connection and the other is dropped. This deterministic sort prevents oscillation.
- **Peer introduction**: Relays can tell each other about additional peers via `{dam: 'opt', opt: {peers: 'url'}}`. A relay accepts up to 99 upstream peer connections.
- **Stay persistence**: `axe.stay()` periodically saves the current upstream peer URLs so they can be restored on restart.

```javascript
// Ask a relay to connect to another relay:
mesh.say({dam: 'opt', opt: {peers: 'https://other-relay.com/gun'}});
```

#### The MOB Module -- Load Shedding

When a relay exceeds `opt.mob` (default from `MOB` env var, or 999999) connected peers:

1. New incoming peers receive a `{dam: 'mob', mob: count, peers: {...}}` message containing URLs of the relay's upstream peers.
2. The relay then disconnects the new peer (`mesh.bye(peer)`).
3. The disconnected peer picks a random peer from the list and connects there instead.

```javascript
// Set mob threshold via environment:
// MOB=5000 node server.js

// Or via options:
var gun = Gun({ mob: 5000, web: httpServer });
```

### Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `opt.axe` | `true` (browser) | Set to `false` to disable AXE entirely |
| `opt.mob` | `999999` or `$MOB` | Max peers before load shedding |
| `opt.super` | `false` | Super-peer mode (relay only, don't re-subscribe on `hi`) |
| `env.AXE` | `'true'` | Set to `'false'` to disable AXE on Node.js relays |

---

## Transport Layers

### WebSocket (Default)

WebSocket is the primary transport for both browser-to-server and server-to-server communication.

#### Browser Side (`src/websocket.js`, bundled into `gun.js`)

The browser WebSocket transport hooks into `Gun.on('opt')` and overrides `mesh.wire`:

```javascript
mesh.wire = function open(peer) {
  var url = peer.url.replace(/^http/, 'ws');
  var wire = peer.wire = new WebSocket(url);
  wire.onopen    = function() { mesh.hi(peer) };
  wire.onmessage = function(msg) { mesh.hear(msg.data, peer) };
  wire.onclose   = function() { reconnect(peer); mesh.bye(peer) };
  wire.onerror   = function() { reconnect(peer) };
};
```

**Reconnection logic**: On disconnect, the browser waits ~2 seconds and retries. The retry counter decrements, and if the page is hidden (`document.hidden`), reconnection is deferred until the tab is visible again. This prevents background tabs from hammering a dead server.

```javascript
// The retry count decreases over time:
peer.retry = (peer.retry || opt.retry+1 || 60) - (timeSinceLastTry < 8s ? 1 : 0);
// When retry reaches 0, the browser stops trying.
```

#### Server Side (`lib/wire.js`)

The server WebSocket transport creates a `ws.Server` and handles upgrades:

```javascript
var ws = require('ws');
ws.web = new ws.Server({ noServer: true });

opt.web.on('upgrade', (req, socket, head) => {
  if (req.url === '/gun') {
    ws.web.handleUpgrade(req, socket, head, function(wire) {
      mesh.hi({ wire: wire });
      wire.on('message', function(msg) { mesh.hear(msg.data || msg, peer) });
      wire.on('close',   function()    { mesh.bye(peer) });
    });
  }
});
```

**Heartbeats**: The server sends empty arrays (`[]`) every 20 seconds to keep connections alive, which is required by platforms like Heroku that timeout idle connections.

**Configuration**:

```javascript
var http = require('http');
var server = http.createServer();
var gun = Gun({ web: server });
server.listen(8765);

// WebSocket path (default: '/gun'):
var gun = Gun({ web: server, ws: { path: '/gun' } });

// Disable WebSocket transport:
var gun = Gun({ ws: false });
```

#### Legacy Server Transport (`lib/ws.js`)

An older WebSocket implementation using the `connection` event pattern instead of the `upgrade` pattern. It includes its own batching (`ws.drain`) and reconnection logic. This is largely superseded by `lib/wire.js` and `src/websocket.js` but remains available.

### WebRTC (`lib/webrtc.js`)

WebRTC enables direct peer-to-peer connections between browsers, bypassing relay servers for data transfer after the initial signaling.

#### How It Works

1. **Room announcement**: When a GUN instance starts, it writes its `pid` to a shared GUN path `/RTC/<room>` and listens for other peers doing the same.

2. **Signaling via relay**: WebRTC signaling (offers, answers, ICE candidates) is sent through existing GUN relay connections using the `{dam: 'rtc'}` protocol message.

3. **Connection setup**: Uses the standard RTCPeerConnection API with STUN servers for NAT traversal:
   ```javascript
   opt.rtc = {
     iceServers: [
       { urls: 'stun:stun.l.google.com:19302' },
       { urls: 'stun:stun.cloudflare.com:3478' }
     ],
     dataChannel: { ordered: false, maxRetransmits: 2 },
     max: 55  // max WebRTC peer connections
   };
   ```

4. **Data channel**: Once the RTCPeerConnection is established, a data channel (`dc`) is opened. All GUN mesh traffic then flows through this channel via `mesh.hear` / `mesh.say`, just like WebSocket.

5. **Mesh integration**: On data channel open, `mesh.hi(peer)` is called. On close, `mesh.bye(peer)`. Messages received on the data channel go through `mesh.hear(msg.data, peer)`.

#### Signaling Flow

```
Browser A                  Relay Server                Browser B
    |                          |                          |
    |-- PUT /RTC/room: pidA -->|                          |
    |                          |-- notify: pidA --------->|
    |                          |<-- rtc offer (pidB) -----|
    |<-- rtc offer (pidB) -----|                          |
    |-- rtc answer (pidA) ---->|                          |
    |                          |-- rtc answer (pidA) ---->|
    |<-- ICE candidates ------>|<-- ICE candidates ------>|
    |                          |                          |
    |<=============== Direct Data Channel ===============>|
```

#### Configuration

```javascript
// Room defaults to location.hash or location.pathname:
var gun = Gun({ rtc: { room: 'my-chat-room' } });

// Custom ICE servers:
var gun = Gun({
  rtc: {
    iceServers: [
      { urls: 'stun:stun.example.com:3478' },
      { urls: 'turn:turn.example.com:3478', username: 'user', credential: 'pass' }
    ]
  }
});

// Disable WebRTC:
var gun = Gun({ RTCPeerConnection: false });
```

#### Media Streams

WebRTC in GUN also supports media tracks (audio/video). The `mesh.wire(mediaStream)` method distributes tracks to all connected WebRTC peers and renegotiates offers:

```javascript
navigator.mediaDevices.getUserMedia({ video: true }).then(function(stream) {
  gun.back('opt.mesh').wire(stream);
});

gun.on('rtc', function(event) {
  if (event.track) {
    videoElement.srcObject = event.streams[0];
  }
});
```

### Multicast (`lib/multicast.js`)

UDP multicast enables zero-configuration LAN peer discovery. Peers on the same local network automatically find each other without needing a relay server.

#### How It Works

1. A UDP socket is created and bound to port 8765 (default).
2. The socket joins multicast group `233.255.255.255`.
3. Peers periodically broadcast a short ID to the multicast address.
4. When a peer hears a broadcast from a different peer, it hooks into the GUN `out` event to relay messages via multicast.
5. All GUN messages are sent as UDP datagrams to the multicast group, where all LAN peers receive them.

```javascript
// Multicast is enabled by default on Node.js.
var gun = Gun({ multicast: { address: '233.255.255.255', port: 8765 } });

// Disable multicast:
var gun = Gun({ multicast: false });
// Or via environment variable:
// MULTICAST=false node server.js
```

#### Configuration

| Option | Default | Description |
|--------|---------|-------------|
| `multicast.address` | `233.255.255.255` | Multicast group address |
| `multicast.port` | `8765` | UDP port |
| `multicast.pack` | `50000` | Max UDP message size (bytes). Messages larger than this are silently dropped. |

#### Limitations

- UDP messages are limited to ~65KB; the default pack size is 50KB.
- Messages exceeding the pack size are silently dropped (no fragmentation).
- Multicast requires all peers to be on the same LAN/subnet.
- There is no encryption on multicast -- all messages are plaintext on the LAN.

---

## Wire Protocol

### Message Format

Every GUN message is a JSON object with the following fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `#` | string | Yes | Unique message ID (9 random chars) |
| `@` | string | No | ACK -- references the `#` of the message being replied to |
| `put` | object | No | Data payload: `{ soul: { key: { value, state } } }` |
| `get` | object | No | Request payload: `{ '#': soul, '.': key }` |
| `ok` | object | No | Acknowledgment metadata: `{ '@': hops, '/': nearPeerCount }` |
| `##` | string/number | No | Content hash of `put` data for deduplication |
| `><` | string | No | Comma-separated peer IDs that have already seen this message |
| `dam` | string | No | DAM protocol message type (not relayed) |
| `err` | string | No | Error description |

### PUT Command

Writes data to the graph. The `put` field contains a graph diff:

```json
{
  "#": "a1b2c3d4e",
  "put": {
    "users/alice": {
      "_": { "#": "users/alice", ">": { "name": 1681234567890 } },
      "name": "Alice"
    }
  }
}
```

### GET Command

Requests data from the graph. The `get` field specifies what to fetch:

```json
{
  "#": "f5g6h7i8j",
  "get": { "#": "users/alice", ".": "name" }
}
```

A GET for an entire node omits the `.` field:

```json
{
  "#": "k9l0m1n2o",
  "get": { "#": "users/alice" }
}
```

### Acknowledgments

ACKs reference the original message ID with `@`:

```json
{
  "#": "p3q4r5s6t",
  "@": "f5g6h7i8j",
  "put": {
    "users/alice": {
      "_": { "#": "users/alice", ">": { "name": 1681234567890 } },
      "name": "Alice"
    }
  }
}
```

An ACK with no `put` indicates "not found":

```json
{
  "#": "u7v8w9x0y",
  "@": "f5g6h7i8j"
}
```

### Deduplication via `><`

When DAM sends a message, it attaches the IDs of its known peers to `><`:

```json
{
  "#": "a1b2c3d4e",
  "put": { ... },
  "><": "peer1,peer2,peer3"
}
```

Receiving peers parse this field and skip relaying to any peer already listed. Each relay **replaces** (not appends) the `><` field with its own peer list. This keeps message size bounded regardless of network depth.

### Batching

For performance, DAM batches multiple messages into a single JSON array per wire send:

```json
[
  { "#": "msg1", "put": { ... } },
  { "#": "msg2", "@": "msg0", "put": { ... } },
  { "#": "msg3", "get": { "#": "soul" } }
]
```

Batching is controlled by `opt.gap` (default 0ms). All messages queued within the gap window are combined into one array.

---

## Relay Peer Setup

### Basic Node.js Relay

```javascript
var http = require('http');
var Gun = require('gun');

var server = http.createServer().listen(8765);
var gun = Gun({ web: server });

console.log('GUN relay running on port 8765');
```

### HTTPS Relay

```javascript
var https = require('https');
var fs = require('fs');
var Gun = require('gun');

var server = https.createServer({
  key: fs.readFileSync('key.pem'),
  cert: fs.readFileSync('cert.pem')
}).listen(443);

var gun = Gun({ web: server });
```

### nginx Reverse Proxy

```nginx
server {
    listen 443 ssl;
    server_name gun.example.com;

    ssl_certificate     /etc/letsencrypt/live/gun.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/gun.example.com/privkey.pem;

    location /gun {
        proxy_pass http://127.0.0.1:8765;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_read_timeout 86400;
    }
}
```

The `proxy_read_timeout` should be set high (86400 = 1 day) to prevent nginx from closing idle WebSocket connections. GUN's heartbeat (every 20s) keeps the connection alive.

### Docker Deployment

```dockerfile
FROM node:18-alpine
WORKDIR /app
RUN npm init -y && npm install gun
COPY server.js .
EXPOSE 8765
CMD ["node", "server.js"]
```

```javascript
// server.js
var http = require('http');
var Gun = require('gun');
var server = http.createServer().listen(8765);
Gun({ web: server });
```

```bash
docker build -t gun-relay .
docker run -d -p 8765:8765 --name gun-relay gun-relay
```

### Express Integration

```javascript
var express = require('express');
var Gun = require('gun');

var app = express();
var server = app.listen(8765);

Gun({ web: server });

app.use(Gun.serve);  // Serve gun.js client files
app.use(express.static('public'));
```

---

## Peer Events

### `gun.on('hi', peer)` -- Peer Connected

Fired when a new peer connection is fully established (wire open, handshake complete):

```javascript
gun.on('hi', function(peer) {
  console.log('Peer connected:', peer.id, peer.url);
});
```

On `hi`, GUN also re-syncs any active subscriptions with the new peer -- it sends GET requests for all souls in `root.next` (the live subscription table) to the newly connected peer.

### `gun.on('bye', peer)` -- Peer Disconnected

Fired when a peer connection is lost:

```javascript
gun.on('bye', function(peer) {
  console.log('Peer disconnected:', peer.id, peer.url);
});
```

### Peer Object Shape

```typescript
interface GunPeer {
  id: string;           // Unique peer identifier (URL or random)
  pid?: string;         // Process ID exchanged via DAM '?' handshake
  url?: string;         // Peer URL (e.g., 'https://relay.example.com/gun')
  wire: WebSocket       // The underlying transport connection
    | RTCPeerConnection
    | UDPSocket
    | null;
  queue: string[];      // Messages queued while wire was connecting
  batch?: string;       // Current outgoing batch being assembled
  tail?: number;        // Current batch byte size
  last?: string;        // ID of last message sent to this peer (dedup)
  met?: number;         // Timestamp of when peer was first connected
  retry?: number;       // Remaining reconnection attempts
  say?: (raw: string) => void;  // Custom send function (e.g., multicast)
  // AXE-specific:
  sub?: Map<string, Map<string, 1>>;  // Subscription map: soul -> { has -> 1 }
  put?: object;         // Batched PUT data pending flush
  next?: string;        // Message ID for batched PUT
  to?: number;          // setTimeout ID for batched flush
}
```

---

## Configuration Reference

| Option | Default | Description |
|--------|---------|-------------|
| `opt.peers` | `{}` | Initial peer URLs: `Gun({ peers: ['https://relay.com/gun'] })` |
| `opt.web` | `null` | HTTP(S) server for WebSocket transport |
| `opt.axe` | `true` (browser) | Enable/disable AXE peer discovery |
| `opt.mesh` | auto-created | Custom mesh instance (rarely needed) |
| `opt.ws` | `{}` | WebSocket options: `{ path: '/gun' }` |
| `opt.ws.path` | `'/gun'` | WebSocket upgrade path |
| `opt.WebSocket` | auto-detect | WebSocket constructor override |
| `opt.RTCPeerConnection` | auto-detect | Set to `false` to disable WebRTC |
| `opt.rtc` | see below | WebRTC configuration |
| `opt.rtc.iceServers` | Google/Cloudflare STUN | ICE server list |
| `opt.rtc.max` | `55` | Max WebRTC peer connections |
| `opt.rtc.room` | `location.hash` or `pathname` | WebRTC room name |
| `opt.multicast` | `{}` | Multicast options or `false` to disable |
| `opt.multicast.address` | `233.255.255.255` | Multicast group IP |
| `opt.multicast.port` | `8765` | Multicast UDP port |
| `opt.multicast.pack` | `50000` | Max UDP message size |
| `opt.gap` | `0` | Batching delay in ms |
| `opt.pack` | ~300 bytes | Max batch size before forced flush |
| `opt.max` | ~90MB | Max single message size |
| `opt.puff` | `9` | Messages processed per tick (CPU yielding) |
| `opt.mob` | `999999` | Max peers before load shedding (AXE) |
| `opt.super` | `false` | Super-peer / relay-only mode |
| `opt.pid` | random | Process ID for this GUN instance |
| `opt.retry` | `60` | Max reconnection attempts (browser) |
| `opt.lack` | `9000` | Milliseconds to suppress re-GETs after peer disconnect |
