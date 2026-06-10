# GUN Core API

Reference documentation for the GUN decentralized database core API, derived from the
[GUN source code](../gun/) and official wiki. GUN is a real-time, decentralized,
offline-first graph database that syncs data peer-to-peer using a conflict-resolution
algorithm (HAM -- Hypothetical Amnesia Machine).

---

## Gun Constructor

```
Gun(options?)
```

Creates a new GUN database instance. Works with or without the `new` keyword -- if called
as a plain function, GUN internally invokes `new Gun(o)` for you.

```javascript
// Source: src/root.js
function Gun(o) {
  if (o instanceof Gun) { return (this._ = { $: this }).$ }
  if (!(this instanceof Gun)) { return new Gun(o) }
  return Gun.create(this._ = { $: this, opt: o });
}
```

### Signatures

| Form | Description |
|------|-------------|
| `Gun()` | Local-only instance, no peers |
| `Gun(url)` | Connect to a single relay peer |
| `Gun([url1, url2, ...])` | Connect to multiple relay peers |
| `Gun(options)` | Full options object |

### Options Object

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `peers` | `string[]` or `Record<string, {}>` | `{}` | Peer relay URLs to sync with. URLs as array items or as object keys with empty-object values. |
| `radisk` | `boolean` | `true` | Enable Node.js persistence via Radisk (Radix + Disk storage). |
| `localStorage` | `boolean` | `true` | Enable browser persistence via `window.localStorage`. Set to `false` to disable. |
| `uuid` | `function` | *(24-char random alphanumeric)* | Override the default soul generator. The default produces a state-prefixed string plus 12 random alphanumeric characters. |
| `file` | `string` | `undefined` | Write data to a JSON file at this path (Node.js). |
| `web` | `any` | `undefined` | Pass an HTTP server to create a WebSocket server for peers to connect to. |
| `s3` | `{ key, secret, bucket, region?, fakes3? }` | `undefined` | Amazon S3 storage adapter configuration. |

Passing a `string` or `string[]` directly is shorthand for `{ peers: [...] }`. Internally,
the constructor calls `gun.opt(options)` which normalizes string and array forms into the
`peers` record.

### UUID Default Implementation

```javascript
// Source: src/root.js (inside Gun.chain.opt)
at.opt.uuid = at.opt.uuid || function uuid(l) {
  return Gun.state().toString(36).replace('.', '') + String.random(l || 12);
}
```

The default UUID concatenates a base-36 state timestamp with 12 random alphanumeric
characters, producing roughly 24-character souls.

### Examples

```javascript
// Local-only database (no syncing)
var gun = Gun();

// Connect to a single relay peer
var gun = Gun('http://yourdomain.com/gun');

// Connect to multiple relay peers
var gun = Gun(['http://server1.com/gun', 'http://server2.com/gun']);

// Full options object
var gun = Gun({
  peers: ['http://server1.com/gun', 'http://server2.com/gun'],
  localStorage: false,
  radisk: true
});

// Node.js server with WebSocket support
var http = require('http');
var server = http.createServer().listen(8080);
var gun = Gun({ web: server });

// Custom UUID generator
var gun = Gun({
  uuid: function () {
    return 'custom-' + Date.now() + '-' + Math.random().toString(36).slice(2);
  }
});

// Persist to JSON file (Node.js)
var gun = Gun({ file: 'data.json' });

// Amazon S3 storage
var gun = Gun({
  s3: {
    key: 'YOUR_AWS_KEY',
    secret: 'YOUR_AWS_SECRET',
    bucket: 'my-gun-bucket',
    region: 'us-east-1'
  }
});
```

---

## gun.get(key)

Navigate into a graph node by its key, soul, or LEX query. Returns a new chain context
pointing at the requested key.

```javascript
// Source: src/get.js
Gun.chain.get = function(key, cb, as) { ... }
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `key` | `string` | An ID (soul) or property name. |
| `key` | `GunSoul` (`{ '#': soul }`) | A soul reference object. |
| `key` | `LEXQuery` (`{ '.': LEX, ':'?: number }`) | A LEX query for pattern matching. |
| `cb` | `(ack) => void` | Optional callback receiving `{ put, get }`. Called potentially multiple times (reactive streaming). |

### LEX Query Operators

LEX (Lexicographic) queries allow pattern-based key matching:

| Operator | Description | Example |
|----------|-------------|---------|
| `'='` | Exact match | `{ '.': { '=': 'alice' } }` |
| `'*'` | Prefix match | `{ '.': { '*': 'user' } }` |
| `'>'` | Greater than or equal (lexicographic) | `{ '.': { '>': 'a' } }` |
| `'<'` | Less than or equal (lexicographic) | `{ '.': { '<': 'z' } }` |
| `'-'` | Reverse order | `{ '.': { '*': 'msg', '-': 1 } }` |

The `':'` property on a LEX query controls the limit (number of results).

### String.match (LEX Resolution)

```javascript
// Source: gun.js (shim)
String.match = function(t, o) {
  if ('string' !== typeof t) { return false }
  if ('string' == typeof o) { o = { '=': o } }
  // ... exact match, prefix match, range match
}
```

### Chain Traversal

Chaining `.get().get()` traverses deeper into the graph. The first `.get()` on a root
instance navigates to a soul (top-level node). Subsequent `.get()` calls navigate to
properties within that node.

### Callback Behavior

The callback may fire **multiple times** because GUN uses a reactive streaming architecture.
Data arrives as partial results from local cache, disk, and peers over time.

### Examples

```javascript
var gun = Gun();

// Navigate to a top-level node by soul
gun.get('alice');

// Traverse into a nested property
gun.get('alice').get('name');

// Deep chaining
gun.get('alice').get('address').get('city');

// Using a soul reference
gun.get({ '#': 'alice' });

// LEX query: prefix match
gun.get('chat').get({ '.': { '*': 'msg' } });

// LEX query: range match
gun.get('scores').get({ '.': { '>': 'a', '<': 'f' } });

// LEX query: exact match with limit
gun.get('users').get({ '.': { '*': 'user-' }, ':': 10 });

// With callback (fires multiple times as data arrives)
gun.get('alice', function (ack) {
  if (ack.err) {
    console.error('Error:', ack.err);
    return;
  }
  console.log('Key:', ack.get);
  console.log('Data:', ack.put);
});

// Callback with chained property
gun.get('alice').get('age', function (ack) {
  console.log('Age data:', ack.put);
});

// Numeric key (converted to string internally)
gun.get(42); // equivalent to gun.get('42')
```

---

## gun.put(data, callback?, options?)

Save data into GUN, syncing it with all connected peers. Uses HAM (Hypothetical Amnesia
Machine) for conflict resolution based on state timestamps.

```javascript
// Source: src/put.js
Gun.chain.put = function(data, cb, as) { ... }
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `data` | `object`, `string`, `number`, `boolean`, `null` | The data to save. Objects can be partial, circular, or nested. |
| `callback` | `(ack) => void` | Optional acknowledgment callback with `{ err }` or `{ ok }`. |
| `options` | `{ opt: { cert: string } }` | Optional write-permission certificate (for SEA-authenticated data). |

### Allowed Value Types

GUN validates values through `Gun.valid()`:

```javascript
// Source: src/valid.js
// Valid: null, string, boolean, number (finite, not NaN), soul reference ({ '#': soul })
// Invalid: arrays, undefined, Infinity, -Infinity, NaN, functions
```

| Type | Allowed | Notes |
|------|---------|-------|
| `null` | Yes | Used for "deleting" (nulling out) keys |
| `string` | Yes | |
| `boolean` | Yes | |
| `number` | Yes | Must be finite and not NaN. `Infinity`, `-Infinity`, and `NaN` are rejected. |
| `object` | Yes | Plain objects only. Merged partially -- existing keys not in the update are preserved. |
| `{ '#': soul }` | Yes | Soul reference (link to another node). |
| `Array` | **No** | Arrays require special concurrency algorithms. Use `.set()` instead. |
| `undefined` | **No** | |
| `function` | **No** | |

### Important Behavior

- **Does NOT change chain context.** Calling `.put()` returns the same chain it was called on.
- **Cannot save primitives at root level.** Data at the root of the graph must be a node
  (an object). You must `.get(key)` first, then `.put()`.
- **Partial updates.** Objects are merged, not replaced. Only the properties you specify
  are updated; other existing properties remain untouched.
- **Circular references.** GUN handles circular/self-referencing data structures via soul
  links.
- **Deferred function.** If `data` is a function, GUN calls it with a `go` callback,
  allowing async data resolution before the write.

### Examples

```javascript
var gun = Gun();

// Save a simple object
gun.get('alice').put({
  name: 'Alice',
  age: 30,
  online: true
});

// Partial update (only updates 'age', leaves 'name' and 'online' intact)
gun.get('alice').put({ age: 31 });

// Save a primitive on a property
gun.get('alice').get('name').put('Alice Smith');

// Null out a property ("delete")
gun.get('alice').get('phone').put(null);

// With acknowledgment callback
gun.get('alice').put({ name: 'Alice' }, function (ack) {
  if (ack.err) {
    console.error('Write failed:', ack.err);
  } else {
    console.log('Write acknowledged');
  }
});

// Nested objects (GUN creates linked sub-nodes automatically)
gun.get('alice').put({
  name: 'Alice',
  address: {
    city: 'New York',
    zip: '10001'
  }
});

// Linking nodes (circular/graph references)
var alice = gun.get('alice').put({ name: 'Alice' });
var bob = gun.get('bob').put({ name: 'Bob' });
alice.get('friend').put(bob);   // alice.friend -> bob
bob.get('friend').put(alice);   // bob.friend -> alice (circular)

// With certificate option (for SEA-authenticated writes)
gun.get('alice').get('profile').put(
  { bio: 'Hello world' },
  function (ack) { console.log(ack); },
  { opt: { cert: certificateString } }
);

// Chain context is NOT changed by put
var ref = gun.get('alice');
ref.put({ name: 'Alice' }); // returns ref (same chain)
ref.get('name').once(function (data) {
  console.log(data); // 'Alice'
});
```

---

## gun.on(callback, options?)

Subscribe to real-time updates on a node or property. The callback fires once with the
current data and then again every time the data changes.

```javascript
// Source: src/on.js
Gun.chain.on = function(tag, arg, eas, as) { ... }
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `callback` | `(data, key, message, event) => void` | Called with current data immediately, and again on every change. |
| `options` | `{ change: true }` or `true` | Only receive the changed properties, not the full node. |

### Callback Arguments

| Argument | Type | Description |
|----------|------|-------------|
| `data` | `any` | The current value of the node or property. |
| `key` | `string` | The property name or soul of the node. |
| `message` | `object` | The raw GUN message envelope. |
| `event` | `IGunOnEvent` | Event handle with `.off()` method to unsubscribe. |

### Important Behavior

- **Does NOT change chain context.** Returns the same chain.
- **Fires immediately** with current cached data, then reactively on changes.
- **Full node by default.** When listening on a node (not a single property), the callback
  receives the entire node object on every change. Use `{ change: true }` to receive only
  the modified fields.
- **Unsubscribe** by calling `event.off()` inside the callback.

### Examples

```javascript
var gun = Gun();

// Subscribe to a property
gun.get('alice').get('name').on(function (data, key) {
  console.log(key, '=', data);
  // Prints: 'name = Alice'
  // Prints again whenever alice's name changes
});

// Subscribe to an entire node
gun.get('alice').on(function (data, key) {
  console.log('Alice:', data);
  // { name: 'Alice', age: 30, _: { '#': 'alice', '>': { ... } } }
});

// Only get changes (not the full node)
gun.get('alice').on(function (data, key) {
  console.log('Changed:', data);
  // Only the changed properties, e.g. { age: 31 }
}, { change: true });

// Shorthand: pass true instead of { change: true }
gun.get('alice').on(function (data, key) {
  console.log('Changed:', data);
}, true);

// Unsubscribe from updates
gun.get('alice').get('name').on(function (data, key, msg, event) {
  console.log(data);
  // Stop listening after first update
  event.off();
});

// Subscribe to multiple properties via chaining
gun.get('alice').get('age').on(function (age) {
  console.log('Age:', age);
});

gun.get('alice').get('online').on(function (online) {
  console.log('Online:', online);
});
```

---

## gun.once(callback?, options?)

Get the current data once without subscribing to future updates. Designed for "read once"
use cases.

```javascript
// Source: src/on.js
Gun.chain.once = function(cb, opt) { ... }
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `callback` | `(data, key) => void` | Receives the data value and the key/soul. |
| `options` | `{ wait: number }` | Controls the async debounce timeout in milliseconds (default: `99`). |

### Callback Arguments

| Argument | Type | Description |
|----------|------|-------------|
| `data` | `any` | The current value. `undefined` if not found. |
| `key` | `string` | The property name or soul. |

### Important Behavior

- **Synchronous if cached.** If data is already loaded in memory, `.once()` fires the
  callback immediately at high performance.
- **Asynchronous on load.** If data is still streaming in from peers or storage, `.once()`
  debounces on a timeout (default 99ms) before firing. This means `.once()` may fire out
  of order compared to other chain methods.
- **Fires again from within.** If you update the node from inside the `.once()` callback,
  it will fire again with the new data.
- **Only 1 layer deep.** Nested objects appear as soul references (`{ '#': soul }`), not
  as resolved data. Use additional `.get().once()` calls to traverse deeper.
- **Does NOT change chain context.** Returns the same chain.
- **Chainable val (experimental).** Calling `.once()` without a callback returns a new
  chain that resolves once, enabling `gun.get('key').once().map()` patterns.

### Examples

```javascript
var gun = Gun();

// Read a property once
gun.get('alice').get('name').once(function (data, key) {
  console.log(key, '=', data);
  // 'name = Alice'
});

// Read an entire node once
gun.get('alice').once(function (data, key) {
  console.log(data);
  // { name: 'Alice', age: 30, _: { '#': 'alice', '>': { ... } } }
});

// Adjust the debounce wait time
gun.get('alice').once(function (data, key) {
  console.log(data);
}, { wait: 500 }); // wait up to 500ms for data to arrive

// Nested references are NOT auto-resolved
gun.get('alice').once(function (data, key) {
  console.log(data.friend);
  // { '#': 'bob' }  (a soul reference, not the full bob node)

  // To resolve the reference, follow it:
  gun.get('bob').once(function (bob) {
    console.log('Friend:', bob.name);
  });
});

// Chainable once (experimental) -- returns a chain
gun.get('alice').once().map().on(function (val, key) {
  console.log(key, val);
});
```

---

## gun.set(data, callback?)

Add a unique item to an unordered list (mathematical set). Internally generates a unique
soul for the item and links it into the parent node.

```javascript
// Source: src/set.js
Gun.chain.set = function(item, cb, opt) { ... }
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `data` | `object` or `GunChain` | The item to add. Only objects are supported (primitives cannot be set members). |
| `callback` | `(ack) => void` | Optional acknowledgment callback, same as `.put()`. |

### Important Behavior

- **CHANGES chain context.** Unlike `.put()`, `.set()` returns a chain pointing at the
  newly added item, not the original list. This allows immediate chaining on the new item.
- **Deduplication.** If the same object (by soul) is added twice, it is merged, not
  duplicated. This is the "set" (mathematical) behavior.
- **Soul generation.** Internally uses `opt.uuid()` to generate a unique soul for new
  items, then calls `.put()` to save and link them.

### Examples

```javascript
var gun = Gun();
var todos = gun.get('todos');

// Add an item to a set
todos.set({ title: 'Buy groceries', done: false });

// Chain on the returned item reference
var item = todos.set({ title: 'Walk the dog', done: false });
item.get('done').put(true); // immediately update the new item

// With callback
todos.set({ title: 'Clean house' }, function (ack) {
  if (ack.err) {
    console.error('Failed to add item:', ack.err);
  } else {
    console.log('Item added');
  }
});

// Add an existing node reference to the set
var alice = gun.get('alice').put({ name: 'Alice' });
var team = gun.get('team');
team.set(alice); // links alice into the team set

// Iterate over set items
todos.map().on(function (item, id) {
  console.log(id, item.title, item.done);
});

// Adding the same node twice merges (no duplicate)
team.set(alice); // alice is still only in team once
```

---

## gun.map(callback?)

Iterate over each property or item on a node. Subscribes to updates on existing items
AND to newly inserted items over time.

```javascript
// Source: src/map.js
Gun.chain.map = function(cb, opt, t) { ... }
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `callback` | `(data, key, msg, event) => any` | Optional transform function. Return `undefined` to filter the item out. Return a value to transform it. If omitted, map passes all items through unchanged. |

You can also pass a LEX object as the callback to filter keys:

```javascript
gun.get('chat').map({ '.': { '*': 'msg' } }).on(cb);
```

### Important Behavior

- **CHANGES chain context.** Returns a new chain representing each individual item.
- **Subscribes to new items.** Unlike a traditional forEach, `.map()` also fires for items
  added after the initial subscription.
- **Transform function (experimental).** When a callback is provided, it acts as a
  transform/filter. Return `undefined` to skip an item; return the data to pass it through;
  return a different value to transform it.
- **Cyclic data.** If your data has circular references, you may receive multiple callbacks
  for the same underlying data from different reference paths.

### Behavior Patterns

The interaction between `.map()`, `.on()`, and `.once()` produces four distinct patterns:

```javascript
var users = gun.get('users');

// 1. Subscribe to ALL users + newly added users, with live updates on each
users.map().on(function (user, id) {
  // Fires for every existing user
  // Fires again when any user changes
  // Fires for newly added users
  console.log(id, user);
});

// 2. Get each user once + newly added users over time (each only once)
users.map().once(function (user, id) {
  // Fires once per existing user
  // Fires once for each newly added user
  console.log(id, user);
});

// 3. Get the user list once, subscribe to changes on those users (NOT new users)
users.once().map().on(function (user, id) {
  // Fires for each user in the current list
  // Fires again when any of those users change
  // Does NOT fire for newly added users
  console.log(id, user);
});

// 4. Get the user list once, each user only once (snapshot)
users.once().map().once(function (user, id) {
  // Fires once per user in the current list
  // No further updates
  console.log(id, user);
});
```

### Transform / Filter Examples

```javascript
var gun = Gun();

// Filter: only show users who are online
gun.get('users').map(function (user, id) {
  if (user.online) {
    return user;  // pass through
  }
  return undefined; // filter out
}).on(function (user, id) {
  console.log(id, 'is online');
});

// Transform: extract just the name
gun.get('users').map(function (user, id) {
  return user.name; // transform to just the name string
}).on(function (name, id) {
  console.log(id, ':', name);
});

// LEX filter: only keys starting with 'msg-'
gun.get('chat').map({ '.': { '*': 'msg-' } }).on(function (data, key) {
  console.log(key, data);
});
```

---

## gun.off()

Remove all listeners on the chain. Cleans up subscriptions, cached data references,
and child chain listeners.

```javascript
// Source: src/on.js
Gun.chain.off = function() { ... }
```

### Behavior

- Resets the acknowledgment counter so the chain can be re-subscribed.
- Removes the chain from its parent's `next` index.
- Clears all `any` listeners (from `.get(cb)` and `.on()`).
- Clears `ask` and cached `put` entries for this key.
- If the chain is a soul chain, removes the soul from the root graph cache.
- Recursively calls `.off()` on any linked or child chains.
- Fires an `'off'` event on the chain.
- Returns the chain.

### Examples

```javascript
var gun = Gun();

// Subscribe then unsubscribe
var alice = gun.get('alice');
alice.get('name').on(function (name) {
  console.log(name);
});

// Later, remove all listeners
alice.off();

// Individual event-level unsubscribe (preferred for single listeners)
gun.get('alice').get('name').on(function (name, key, msg, event) {
  console.log(name);
  event.off(); // unsubscribe just this listener
});
```

---

## gun.back(amount?)

Move up to a parent context on the chain. Every time a new chain is created (via `.get()`,
`.map()`, etc.), a reference to the previous context is retained.

```javascript
// Source: src/back.js
Gun.chain.back = function(n, opt) { ... }
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `amount` | `number` | Number of levels to go back up. Default is `1`. |
| `amount` | `-1` | Return to the root GUN instance. |
| `amount` | `string` | Dot-separated path to look up through the parent chain (e.g., `'opt.uuid'`). |
| `amount` | `function` | Walk up the chain calling the function at each level until it returns a non-undefined value. |

### Examples

```javascript
var gun = Gun();

// Go back one level
var city = gun.get('alice').get('address').get('city');
var address = city.back();   // points to gun.get('alice').get('address')

// Go back two levels
var alice = city.back(2);    // points to gun.get('alice')

// Go back to the root
var root = city.back(-1);    // points to gun (the root instance)

// Using string path lookup (looks up through parent chain)
gun.get('alice').back('opt.uuid'); // returns the uuid function from options

// Using function walker
gun.get('alice').get('name').back(function (at) {
  if (at.soul) {
    return at.soul; // returns 'alice'
  }
});
```

---

## gun.opt(options)

Change the GUN instance configuration at runtime. Accepts the same options as the
constructor.

```javascript
// Source: src/root.js (inside Gun.chain.opt)
Gun.chain.opt = function(opt) { ... }
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `options` | `GunOptions` | Same options as the Gun constructor. |

### Important Behavior

- **Peers are ADDED, not replaced.** Calling `.opt({ peers: [...] })` merges new peers
  into the existing peer list. It does not remove previously configured peers.
- Fires the `'opt'` event on the GUN instance, which storage adapters and network
  transports listen to for re-configuration.
- Returns the gun instance.

### Examples

```javascript
var gun = Gun();

// Add a peer at runtime
gun.opt({ peers: ['http://newpeer.com/gun'] });

// Add multiple peers
gun.opt({ peers: ['http://peer1.com/gun', 'http://peer2.com/gun'] });

// Pass a string directly (shorthand for peers)
gun.opt('http://anotherpeer.com/gun');

// Disable localStorage at runtime
gun.opt({ localStorage: false });

// Provide a custom uuid generator at runtime
gun.opt({
  uuid: function () {
    return 'runtime-' + Date.now();
  }
});
```

---

## Gun.state()

Returns GUN's current state timestamp. This is used internally for conflict resolution --
every write in GUN is tagged with a state timestamp, and HAM uses these to determine which
write wins.

```javascript
// Source: src/state.js
function State() {
  var t = +new Date;
  if (last < t) {
    return N = 0, last = t + State.drift;
  }
  return last = t + ((N += 1) / D) + State.drift;
}
State.drift = 0;
```

### Behavior

- Returns a **number** (milliseconds since epoch, with sub-millisecond resolution for
  rapid successive calls).
- If multiple calls happen within the same millisecond, a fractional increment (`N / 999`)
  is added to maintain strict ordering.
- `Gun.state.drift` can be set to adjust for clock skew between peers (defaults to `0`).

### Related Internal Functions

| Function | Description |
|----------|-------------|
| `Gun.state.is(node, key)` | Get the state timestamp for a specific key on a node. |
| `Gun.state.ify(node, key, state, value, soul)` | Set a key's state and value on a node. |

### Examples

```javascript
// Get current GUN state timestamp
var now = Gun.state();
console.log(now); // e.g., 1700000000000

// State increases monotonically, even within the same millisecond
var a = Gun.state();
var b = Gun.state();
console.log(b > a); // true (b has a fractional increment)

// Adjust for clock drift (e.g., if peers are out of sync)
Gun.state.drift = 500; // offset by 500ms

// Inspect the state of a node's property
var node = { name: 'Alice', _: { '#': 'alice', '>': { name: 1700000000000 } } };
var nameState = Gun.state.is(node, 'name');
console.log(nameState); // 1700000000000

// Build a state-annotated node
var node = Gun.state.ify({}, 'name', Gun.state(), 'Alice', 'alice');
console.log(node);
// { name: 'Alice', _: { '#': 'alice', '>': { name: 1700000000000 } } }
```

---

## Summary of Chain Context Behavior

Understanding which methods change the chain context is critical for correct chaining:

| Method | Changes Context? | Returns |
|--------|-----------------|---------|
| `gun.get(key)` | **Yes** | New chain pointing at the key/node |
| `gun.put(data)` | No | Same chain |
| `gun.on(cb)` | No | Same chain |
| `gun.once(cb)` | No | Same chain |
| `gun.set(data)` | **Yes** | New chain pointing at the added item |
| `gun.map(cb)` | **Yes** | New chain representing each mapped item |
| `gun.off()` | No | Same chain |
| `gun.back(n)` | **Yes** | Parent chain (or root) |
| `gun.opt(options)` | No | Same chain |

## Valid Data Types Reference

From `src/valid.js`, GUN accepts this subset of JSON:

```
null | string | boolean | number (finite, not NaN) | { '#': soul }
```

Arrays are intentionally excluded because they require special concurrency algorithms.
Use `.set()` for list-like data, or a GUN extension that provides array support.
