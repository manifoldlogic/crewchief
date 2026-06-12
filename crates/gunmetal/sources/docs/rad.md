# RAD -- Radix Storage Engine

## Overview

RAD (Radisk) is GUN's persistent storage layer. It uses a radix tree (also called a
patricia trie or compact prefix tree) to organize key-value data, then chunks that
tree into files on disk (or objects in S3, records in IndexedDB, entries in
localStorage, etc.). The radix structure lets keys that share a common prefix
collapse into shared internal nodes, so storage and lookup are proportional to the
key's distinguishing suffix rather than its full length.

RAD sits between GUN's in-memory graph and whatever durable storage backend you
choose. It handles:

- **Chunking** -- splitting a large radix tree across multiple files so no single
  file grows unbounded.
- **Batching** -- coalescing many writes into a single flush to reduce I/O.
- **Memory management** -- capping the byte size of in-memory data and evicting
  stale chunks.
- **Range queries** -- reading across chunk boundaries with prefix, range, and
  reverse iteration.
- **Serialization** -- converting the in-memory tree to and from a JSON envelope
  (or the legacy RAD binary format) for storage.

RAD is storage-agnostic. The only requirement is a store object that implements
`put(key, data, cb)` and `get(key, cb)`. Built-in adapters exist for the
filesystem, IndexedDB, localStorage (fallback), and S3.

---

## How Radix Trees Work

A radix tree is a space-optimized trie where each internal node with only one child
is merged with that child. Instead of storing one character per node, each edge
carries an arbitrary-length string prefix.

### Example

Inserting the keys `alex`, `alexandria`, and `andrew`:

```
(root)
  |
  "a"
  |
  +-- "lex" ----+-- "" => 27           (key: "alex")
  |             |
  |             +-- "andria" => "library" (key: "alexandria")
  |
  +-- "ndrew" => true                   (key: "andrew")
```

The common prefix `a` is shared. `alex` and `alexandria` further share the
prefix `lex`. The tree only branches where keys actually diverge, so lookup is
O(k) where k is the length of the key, and shared prefixes cost no extra space.

### In-memory representation

Internally, GUN's `Radix()` stores the tree as nested plain objects. Each object
maps a prefix string to either another object (subtree) or a terminal value at the
empty-string key `""`:

```javascript
// After inserting alex=27, alexandria="library", andrew=true:
{
  "a": {
    "lex": {
      "": 27,              // "alex"
      "andria": {
        "": "library"      // "alexandria"
      }
    },
    "ndrew": {
      "": true             // "andrew"
    }
  }
}
```

This is the raw `radix.$` object that gets serialized to disk.

---

## RAD Architecture

```
  GUN in-memory graph
        |
        | put / get events (soul + key + value + state)
        v
  +-----------+
  |  store.js |  GUN event adapter -- translates graph ops into RAD calls
  +-----------+
        |
        | dare(soul+ESC+key, {':': val, '>': state}, cb)
        v
  +------------+
  | radisk.js  |  Core RAD engine -- batching, chunking, read/write, directory
  +------------+
        |
        | opt.store.put(key, data, cb) / opt.store.get(key, cb)
        v
  +---------------------+
  | Storage adapter      |
  | (rfs / rindexed /    |
  |  rs3 / custom)       |
  +---------------------+
```

### Key concatenation

GUN stores graph data as `soul + ESC + key` where `ESC` is `String.fromCharCode(27)`.
So a property `name` on node `users/alice` becomes the RAD key `users/alice\x1bname`.
The value stored is `{':': <value>, '>': <state>}` -- a small envelope with the
conflict-resolution state timestamp.

### The directory file

RAD maintains a directory radix tree stored under the special file key
`String.fromCharCode(28)`. This directory maps file names to `1` (present) or `0`
(deleted). When RAD needs to find which file contains a given key, it walks the
directory in reverse to find the greatest file name that is <= the key. The
directory is itself persisted through the same `opt.store.put`/`get` interface.

---

## Storage Interface

Every storage adapter must expose two functions and may optionally expose a third:

### `put(key, data, cb)`

Persist a chunk of serialized data under `key`.

| Parameter | Type     | Description |
|-----------|----------|-------------|
| `key`     | `string` | The URL-encoded file name (e.g. `"!"`  or `"%21users%2Falice"`) |
| `data`    | `string` | The serialized JSON string of the radix subtree |
| `cb`      | `function(err, ok)` | Callback. Call with `(null, 1)` on success or `(error)` on failure. |

### `get(key, cb)`

Retrieve a previously stored chunk.

| Parameter | Type     | Description |
|-----------|----------|-------------|
| `key`     | `string` | The URL-encoded file name |
| `cb`      | `function(err, data)` | Callback. `data` is the raw string (or `undefined` if not found). For missing keys, call `cb()` with no arguments -- do NOT call `cb(error)`. |

### `list(cb)` (optional)

Enumerate all stored file names. Called once at startup to import the directory.

| Parameter | Type     | Description |
|-----------|----------|-------------|
| `cb`      | `function(file)` | Called once per file with the file name string. Call with no arguments (or falsy) when enumeration is complete. |

### Filesystem adapter (`rfs.js`)

The filesystem adapter writes files into a directory named by `opt.file` (default:
`radata`). It uses an atomic write strategy:

1. Write to a temporary file: `<opt.file>-<key>-<random>.tmp`
2. Rename (move) the temp file to `<opt.file>/<key>`
3. If rename fails with `EXDEV` (cross-device), fall back to stream copy + unlink.

This prevents partial writes from corrupting data on crash.

```javascript
// Simplified filesystem adapter
var fs = require('fs');
var store = {};

store.put = function(file, data, cb) {
  var tmp = 'radata-' + file + '-' + Math.random().toString(36).slice(-3) + '.tmp';
  fs.writeFile(tmp, data, function(err) {
    if (err) { return cb(err); }
    fs.rename(tmp, 'radata/' + file, function(err) {
      cb(err, !err || undefined);
    });
  });
};

store.get = function(file, cb) {
  fs.readFile('radata/' + file, function(err, data) {
    if (err && err.code === 'ENOENT') { return cb(); }  // not found, no error
    cb(err, data);
  });
};

store.list = function(cb) {
  var dir = fs.readdirSync('radata');
  dir.forEach(function(file) { cb(file); });
  cb();  // signal end
};
```

The filesystem adapter also maintains an in-memory cache (`puts`) of pending writes.
If a `get` arrives for a file currently being written, it returns the cached data
immediately instead of reading from disk.

### IndexedDB adapter (`rindexed.js`)

The IndexedDB adapter opens (or creates) a database named by `opt.file` with a
single object store. It has a built-in workaround for a WebKit bug: every 15 seconds
it closes and reopens the database connection.

```javascript
// store.js (browser) -- uses rindexed.js internally
// IndexedDB initialization
var o = indexedDB.open(opt.file, 1);
o.onupgradeneeded = function(eve) {
  eve.target.result.createObjectStore(opt.file);
};

// put: readwrite transaction
store.put = function(key, data, cb) {
  var tx = db.transaction([opt.file], 'readwrite');
  var obj = tx.objectStore(opt.file);
  var req = obj.put(data, '' + key);
  req.onsuccess = function() { cb(null, 1); };
  req.onerror = function(eve) { cb(eve || 'put.tx.error'); };
};

// get: readonly transaction
store.get = function(key, cb) {
  var tx = db.transaction([opt.file], 'readonly');
  var obj = tx.objectStore(opt.file);
  var req = obj.get('' + key);
  req.onsuccess = function() { cb(null, req.result); };
  req.onerror = function(eve) { cb(eve || 5); };
};
```

If IndexedDB is not available (e.g. `file:` protocol pages), the adapter falls back
to an in-memory object store with artificial latency to simulate async behavior.

### localStorage adapter (minimal example)

```javascript
var store = {};
store.put = function(key, data, cb) {
  try {
    localStorage['' + key] = data;
    cb(null, 1);
  } catch (e) {
    cb(e);
  }
};
store.get = function(key, cb) {
  cb(null, localStorage['' + key] || undefined);
};
```

### S3 adapter (`rs3.js`)

See the [S3 Integration](#s3-integration) section below.

---

## Batching and Flushing

RAD does not write to disk on every `put`. Instead it buffers writes in memory and
flushes them according to two triggers:

### Timer-based flush

After the first write arrives for a given chunk file, a `setTimeout` is scheduled
for `opt.until` milliseconds (default: **250 ms**). When the timer fires, all
buffered writes for that chunk are serialized and written to storage in a single
operation.

```
write("alex", 27)      --> starts 250ms timer for chunk "!"
write("alexandria", …) --> appends to same buffer, timer still running
write("andrew", …)     --> appends to same buffer
                        ... 250ms passes ...
                        --> flush: serialize all three, write to storage
```

### Forced flush on batch size

There is no explicit batch-size flush in the timer path. However, the `opt.batch`
value (default: **10,000**) is available for the higher-level GUN store adapter
(`store.js`) to use. The GUN `file.js` adapter, for example, triggers an immediate
flush when `count >= opt.batch`.

### Callback queuing

Every write call receives a callback. These callbacks are collected in an array
(`disk.Q`) attached to the chunk's in-memory radix tree. When the flush completes
(success or failure), every queued callback is invoked with the same `(err, ok)`
arguments. This means 1000 writes that batch into one flush produce only one disk
I/O but 1000 callback invocations.

### Tag-based acknowledgment

RAD supports an optional `tag` parameter on writes. Tagged writes share a single
callback per tag across multiple chunk files. The callback fires only when all
chunks associated with that tag have been flushed. This is used internally by GUN to
acknowledge a put message only after all affected chunks are persisted.

---

## Chunking

### How data splits across files

Each chunk file holds a serialized radix subtree. The file name is the
lexicographically smallest key in that subtree (URL-encoded). The very first chunk
file is always named `!` (the `opt.code.from` value, default `"!"`).

When a chunk's serialized size exceeds `opt.chunk` bytes, the write path triggers a
**split**:

1. Count the total entries in the radix tree.
2. Set a limit at `ceil(count / 2)`.
3. Walk the tree **in reverse** (important -- the last half is moved first so that
   reads during the split still find data).
4. Move the last half into a new `Radix()` subtree.
5. Write the new subtree to a new file (named after the first key moved).
6. Write the remaining first half back to the original file.
7. Update the directory to include the new file.

```javascript
// From radisk.js -- the split logic
f.split = function() {
  f.limit = Math.ceil(f.count / 2);
  f.sub = Radix();
  Radix.map(rad, f.slice, {reverse: 1});   // move last half to f.sub
  r.write(f.end, f.sub, function(err, ok) { // write new chunk
    f.hub = Radix();
    Radix.map(rad, f.stop);                 // copy first half to f.hub
    r.write(rad.file, f.hub, cb, o);        // rewrite original chunk
  }, o);
};
```

### When splits happen

A split occurs when `opt.chunk < serializedText.length + nextEntry.length` and the
chunk contains more than one entry. Single-entry chunks are never split (they
represent a single very large value). In JSON mode (`opt.jsonify = true`, which is
the default), the check compares `opt.chunk` against `JSON.stringify(rad.$).length`.

### Chunk size defaults

| Environment | Default `opt.chunk` | Notes |
|-------------|---------------------|-------|
| Node.js (radisk.js) | `1,048,576` (1 MB) | Set in radisk.js constructor |
| Browser (IndexedDB) | `1,048,576` (1 MB) | Same default from radisk.js |

The original GUN documentation references 10 MB for Node.js, but the source code
in `radisk.js` sets `opt.chunk = opt.chunk || (1024 * 1024 * 1)` -- 1 MB. This can
be overridden:

```javascript
var gun = Gun({ chunk: 1024 * 1024 * 10 }); // 10 MB chunks
```

---

## Memory Management

### The `pack` / `max` limit

RAD caps the maximum size of any single value at `opt.max` bytes. This is computed
from the `opt.memory` option or defaults to `300,000,000 * 0.3` = **90,000,000
bytes** (~90 MB):

```javascript
opt.max = opt.max || (opt.memory ? (opt.memory * 999 * 999) : 300000000) * 0.3;
```

If a value exceeds `opt.max`, the write is rejected with `"Data too big!"`. During
reads, if raw chunk data from storage exceeds `opt.max`, parsing is aborted with
`"Chunk too big!"`.

### Chunk eviction

After a chunk is written to storage and there are no pending writes queued
(`!rad.Q`), the in-memory copy is deleted:

```javascript
// In the write callback:
if (!rad.Q) { delete r.disk[file]; }
// "VERY IMPORTANT! Clean up memory, but not if there is already queued writes on it!"
```

This means chunks are loaded on demand for reads, held in memory only while writes
are pending, and evicted once flushed. Reads that hit a cached chunk (`r.disk[file]`)
skip the storage round-trip.

### Corrupt file recovery

If a read returns data that fails to parse, RAD:

1. Removes the file from the directory (`r.find.bad(file)`).
2. Retries the read, which will now skip the bad file and fall through to the next
   chunk or to the root file.

This provides basic self-healing for corrupted chunk files.

### Mislocated data correction

After reading a chunk, RAD walks every key-value pair and checks whether each key
belongs in the file it was found in (by consulting the directory). If a key should
live in a different file, RAD issues a background `save` to move it to the correct
location. This handles data that ended up in the wrong chunk due to concurrent
operations or prior bugs.

---

## Serialization

### JSON mode (default)

When `opt.jsonify` is `true` (the default), chunks are serialized as a JSON string
of the raw radix `.$` object:

```javascript
// Serialization
var raw = JSON.stringify(rad.$);

// Deserialization
var tree = JSON.parse(data);
disk.$ = tree;
```

The stored format is a plain JSON object representing the nested radix tree:

```json
{
  "a": {
    "lex": {
      "": 27,
      "andria": {
        "": "library"
      }
    },
    "ndrew": {
      "": true
    }
  }
}
```

### Legacy RAD binary format

If `opt.jsonify` is `false`, or if stored data does not start with `{`, RAD falls
back to its legacy binary encoding. This format uses `String.fromCharCode(31)` (the
Unit Separator, `US`) as a delimiter and encodes each entry as:

```
<encoded-prefix-depth>#<encoded-key-fragment>:<encoded-value>\n
```

The encoding scheme (`Radisk.encode` / `Radisk.decode`) uses a type prefix after the
delimiter:

| Prefix | Type    | Example encoded form |
|--------|---------|---------------------|
| `"`    | string  | `\x1f"hello\x1f` |
| `+`    | number  | `\x1f+42\x1f` |
| `+`    | true    | `\x1f+\x1f` (empty number = true) |
| `-`    | false   | `\x1f-\x1f` |
| ` `    | null    | `\x1f \x1f` (space) |
| `#`    | soul ref | `\x1f#soul-id\x1f` |

Strings that contain the delimiter character escape it by repeating the delimiter
prefix. The decoder counts consecutive delimiter characters to determine where the
value ends.

### Async parsing

RAD uses `JSON.parseAsync` and `JSON.stringifyAsync` if available (provided by
GUN's `yson.js`). These break large parse/stringify operations into smaller chunks
to avoid blocking the event loop. If not available, standard synchronous
`JSON.parse`/`JSON.stringify` are used with try/catch.

---

## Lexical Queries (Lex) in Depth

GUN's wire protocol supports lexical queries -- structured get requests that go
beyond exact key matches. RAD implements these using the radix tree's natural
ordering.

### Exact match (`=`)

```javascript
gun.get('users').get('alice');
// Wire: {'#': 'users', '.': 'alice'}
// RAD key: "users\x1balice"
```

Returns the single value at the exact key.

### Prefix match (`*`)

```javascript
gun.get('chat').get({ '.': { '*': '2024/06/' } }).map().once(cb);
// Wire: {'#': 'chat', '.': {'*': '2024/06/'}}
```

Returns all keys in the `chat` node that start with `2024/06/`. Internally, RAD
looks up the prefix in the radix tree, which returns the entire subtree rooted at
that prefix. `Radix.map` then iterates the subtree.

**Edge case:** A prefix of `""` (empty string) matches all keys in the node.

### Range queries (`>` and `<`)

```javascript
gun.get('friends').get({
  '.': { '>': 'alice', '<': 'fred' },
  '%': 50000
}).once().map().once(cb);
// Wire: {'#': 'friends', '.': {'>': 'alice', '<': 'fred'}, '%': 50000}
```

Returns all keys between `alice` and `fred` (inclusive on both bounds), with a
byte-size limit of 50,000 bytes for pagination.

**Both bounds are inclusive.** This simplifies pagination: to get the next page, use
the last received key as the new `>` bound. The last key will appear again as the
first result of the next page (the consumer de-duplicates).

```javascript
// Page 1
gun.get('friends').get({'.': {'>': 'alice', '<': 'zach'}, '%': 50000})
  .once().map().once(function(data, key) {
    // last key received: "dave"
  });

// Page 2 -- start from "dave"
gun.get('friends').get({'.': {'>': 'dave', '<': 'zach'}, '%': 50000})
  .once().map().once(cb);
```

**Single-direction ranges:**

```javascript
// All keys after "m"
gun.get('words').get({'.': {'>': 'm'}}).map().once(cb);

// All keys before "m"
gun.get('words').get({'.': {'<': 'm'}}).map().once(cb);
```

### Reverse iteration (`-`)

```javascript
gun.get('data').get({
  '.': { '<': 'zach', '-': 1 },
  '%': 50000
});
```

The `-` flag reverses iteration order. Combined with `<` you get the last N entries.
Internally, `Radix.map` is called with `{reverse: 1}` which reverses the sorted
keys array before iteration.

### Match hierarchy (cascading specificity)

When multiple match operators are present, they follow a strict precedence:

1. **`=`** -- Exact match. If present, `*`, `>`, `<` are ignored.
2. **`*`** -- Prefix match. If present, `>` and `<` are ignored.
3. **`>` AND `<`** -- Range match (inclusive on both bounds).
4. **`>` OR `<`** -- Single-direction match.

### Cross-chunk range reads

Range queries can span multiple chunk files. RAD handles this by:

1. Finding the chunk file that contains (or would contain) the start key.
2. Reading and filtering that chunk.
3. Checking the directory for the next chunk file.
4. If `o.more` is true and `o.parsed < o.limit`, scheduling an async read of the
   next chunk.
5. Repeating until there are no more relevant chunks or the byte limit is reached.

Each chunk callback fires independently, so the consumer may receive multiple
callbacks for a single range query.

---

## Radix In-Memory API

The `Radix` module (`radix.js`) provides the in-memory radix tree used by RAD.

### Creating a tree

```javascript
// Node.js
var Radix = require('gun/lib/radix');

// Browser (after loading radix.js)
var Radix = window.Radix;

// Create a new empty tree
var tree = Radix();
```

### Writing values

```javascript
tree('alex', 27);
tree('alexandria', 'library');
tree('andrew', true);
tree('bob', null);        // null is a valid value
tree('charlie', false);   // false is a valid value
```

Values can be: strings, numbers, booleans, `null`, or soul references (`{'#': 'soul-id'}`).
Objects (other than soul refs) and arrays are NOT supported as leaf values.

### Reading values

```javascript
// Exact match -- returns the value or undefined
var val = tree('alex');
// val === 27

// Prefix match -- returns a subtree object if no exact match
var sub = tree('ale');
// sub is the subtree containing "x" -> {""  : 27, "andria": {"": "library"}}

// Reading the root -- returns the entire raw tree
var all = tree('');
// all === tree.$
```

### The `tree.$` property

The raw nested object backing the tree is accessible as `tree.$`. This is what gets
serialized to JSON for storage.

### The `tree.last` property

Tracks the lexicographically greatest key ever inserted. Updated on every write.

### The `tree.unit` property

Set to `1` when an exact leaf value is found during a read (as opposed to a subtree).
Set to `0` at the start of every operation. Used by RAD to distinguish "found a
single record" from "found a subtree of records".

### Iterating with `Radix.map`

`Radix.map(tree, callback, options, prefix)` walks the tree in sorted key order and
calls `callback(value, fullKey, lastKeyFragment, prefixArray)` for each leaf.

```javascript
Radix.map(tree, function(value, key) {
  console.log(key, '=', value);
});
// Output (sorted):
// "alex" = 27
// "alexandria" = "library"
// "andrew" = true
// "bob" = null
// "charlie" = false
```

**Stopping early:** Return any non-undefined value from the callback to stop
iteration.

```javascript
var first;
Radix.map(tree, function(value, key) {
  first = { key: key, value: value };
  return true; // stop
});
// first === { key: "alex", value: 27 }
```

**Options:**

| Option    | Type      | Description |
|-----------|-----------|-------------|
| `reverse` | truthy    | Iterate in reverse lexicographic order |
| `start`   | `string`  | Skip keys before this value (inclusive) |
| `end`     | `string`  | Skip keys after this value (inclusive; defaults to `\uffff`) |
| `branch`  | `boolean` | If true, callback fires for intermediate nodes (not just leaves) |

```javascript
// Reverse iteration
Radix.map(tree, function(v, k) {
  console.log(k, v);
}, { reverse: 1 });

// Range: only keys from "b" to "d"
Radix.map(tree, function(v, k) {
  console.log(k, v);
}, { start: 'b', end: 'd' });

// Combine: reverse range
Radix.map(tree, function(v, k) {
  console.log(k, v);
}, { reverse: 1, start: 'a', end: 'b' });
```

### `Radix.object` (aliased as `each`)

A utility for iterating plain objects:

```javascript
Radix.object(obj, function(value, key) {
  // called for each own property
  // return non-undefined to stop
});
```

---

## Custom Storage Adapters

To use RAD with a custom backend, provide a `store` object with `put` and `get`.

### Minimal adapter template

```javascript
var Radisk = require('gun/lib/radisk');

function MyStore(opt) {
  var store = {};

  store.put = function(key, data, cb) {
    // key: string (URL-encoded file name)
    // data: string (serialized JSON of the radix subtree)
    // cb(err, ok): call cb(null, 1) on success, cb(error) on failure
    myBackend.write(key, data)
      .then(function() { cb(null, 1); })
      .catch(function(err) { cb(err); });
  };

  store.get = function(key, cb) {
    // key: string
    // cb(err, data): call cb(null, data) on success
    //   For missing keys: cb(null, undefined) or just cb()
    //   Do NOT cb(error) for "not found"
    myBackend.read(key)
      .then(function(data) { cb(null, data); })
      .catch(function(err) {
        if (err.code === 'NOT_FOUND') { cb(); return; }
        cb(err);
      });
  };

  // Optional: list all stored file names at startup
  store.list = function(cb) {
    myBackend.listKeys().then(function(keys) {
      keys.forEach(function(k) { cb(k); });
      cb(); // signal end of list
    });
  };

  return store;
}

// Use it:
var rad = Radisk({
  store: MyStore({ file: 'mydata' }),
  file: 'mydata'
});
```

### Using with GUN

To wire a custom adapter into GUN, hook into the `create` event:

```javascript
var Gun = require('gun');

Gun.on('create', function(root) {
  this.to.next(root);
  root.opt.store = root.opt.store || MyStore(root.opt);
});
```

### Important adapter contract details

1. **Async required:** Both `put` and `get` must call their callbacks asynchronously
   (even if the operation is synchronous, wrap in `setTimeout`).
2. **Not-found is not an error:** When `get` finds no data for a key, call `cb()`
   or `cb(null, undefined)`. Calling `cb(error)` for missing keys will cause RAD to
   log errors and potentially retry indefinitely.
3. **String data:** The `data` argument to `put` is always a string (JSON). The
   `data` returned from `get` should be a string or Buffer. RAD will call
   `.toString()` on Buffer data.
4. **Key encoding:** File names passed to the adapter are already URL-encoded by
   RAD using `encodeURIComponent` with `*` escaped as `%2A`.
5. **Singleton pattern:** Store implementations should cache by `opt.file` to prevent
   opening multiple connections to the same backend.

---

## Configuration Reference Table

All options are passed through the GUN constructor or directly to `Radisk()`.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `file` | `string` | `'radata'` | Directory/database name for storage |
| `chunk` | `number` | `1048576` (1 MB) | Max bytes per chunk file before splitting |
| `until` / `wait` | `number` | `250` | Milliseconds to wait before flushing a batch to disk |
| `batch` | `number` | `10000` | Max write count before forced flush (used by `file.js` adapter) |
| `max` | `number` | `90000000` (~90 MB) | Max byte size for a single value. Computed as `(opt.memory * 999 * 999 \|\| 300000000) * 0.3` |
| `memory` | `number` | `undefined` | If set, `max` is derived as `memory * 999 * 999 * 0.3` |
| `store` | `object` | auto-detected | Storage adapter with `put`/`get`/`list` methods |
| `code.from` | `string` | `'!'` | Name of the initial/root chunk file |
| `jsonify` | `boolean` | `true` | Use JSON serialization (vs legacy RAD binary format) |
| `compare` | `function` | `undefined` | Custom comparator `(existing, incoming, key, file) => value`. Return `undefined` to skip the write. |
| `log` | `function` | `console.log` | Logging function for errors and warnings |
| `rad` / `radisk` | `boolean` | `true` | Set to `false` to disable RAD entirely |
| `localStorage` | `boolean` | `true` (browser) | Set to `false` to disable localStorage fallback |
| `s3` | `object` | `undefined` | S3 configuration object (see S3 Integration) |
| `rfs` | `boolean` | `undefined` | Set to `false` to disable filesystem adapter; `true` to force it even in browser |
| `indexedDB` | `object` | `window.indexedDB` | Custom IndexedDB implementation |

### GUN constructor examples

```javascript
// Default Node.js -- filesystem storage in ./radata/
var gun = Gun();

// Custom directory
var gun = Gun({ file: 'my-data-dir' });

// Larger chunks, longer flush interval
var gun = Gun({ chunk: 1024 * 1024 * 10, until: 1000 });

// Disable RAD entirely (in-memory only)
var gun = Gun({ rad: false });

// Disable localStorage in browser
var gun = Gun({ localStorage: false });

// Custom memory limit
var gun = Gun({ memory: 1 }); // ~300 MB max value size
```

---

## Performance Considerations

### Chunk size tuning

| Chunk size | Pros | Cons |
|------------|------|------|
| Smaller (100 KB) | Faster individual reads, lower memory per chunk | More files, more directory overhead, more frequent splits |
| Default (1 MB) | Balanced for most workloads | -- |
| Larger (10 MB) | Fewer files, fewer splits, better sequential write throughput | Higher memory per chunk, slower individual reads, longer parse times |

For write-heavy workloads with large values, increase chunk size. For read-heavy
workloads with many small lookups, keep chunks small.

### Batch/flush timing

| Flush interval | Pros | Cons |
|----------------|------|------|
| Short (50 ms) | Lower latency to durability | More I/O operations, higher disk load |
| Default (250 ms) | Good balance of latency and throughput | -- |
| Long (1-3 s) | Fewer I/O operations, better throughput | Higher risk of data loss on crash, higher callback latency |

### Serialization performance

JSON mode (`opt.jsonify = true`) is the default and is generally faster than the
legacy RAD binary format for both serialization and deserialization. The legacy
format requires character-by-character parsing with the custom `Radisk.encode`/
`Radisk.decode` functions, while JSON benefits from V8's optimized native parser.

### Key design

Since RAD uses a radix tree, keys with shared prefixes are stored efficiently.
Design your key schema to take advantage of this:

```javascript
// Good: shared prefixes collapse in the tree
"users/alice/name"
"users/alice/email"
"users/bob/name"

// Less efficient: no shared prefixes
"a1b2c3d4"
"e5f6g7h8"
"i9j0k1l2"
```

### Read-before-write

RAD must read a chunk from storage before writing to it (to merge the new data into
the existing tree). This is unavoidable for any storage system that supports updates.
The in-memory cache (`r.disk`) mitigates this by keeping recently-accessed chunks
in memory. Writes to the same chunk within a flush window avoid redundant reads.

---

## S3 Integration

### Automatic detection

If the environment variable `AWS_S3_BUCKET` is set, GUN automatically uses S3 as the
storage backend instead of the filesystem. The `aws-sdk` npm package must be
installed.

### Environment variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `AWS_S3_BUCKET` | Yes | -- | S3 bucket name |
| `AWS_REGION` | No | `us-east-1` | AWS region |
| `AWS_ACCESS_KEY_ID` | Yes | -- | AWS access key |
| `AWS_SECRET_ACCESS_KEY` | Yes | -- | AWS secret key |

### Programmatic configuration

```javascript
var gun = Gun({
  s3: {
    bucket: 'my-gun-bucket',
    region: 'us-west-2',
    key: 'AKIAIOSFODNN7EXAMPLE',            // or accessKeyId
    secret: 'wJalrXUtnFEMI/K7MDENG/bPxR',  // or secretAccessKey
  }
});
```

### S3-compatible services (fakes3)

For S3-compatible services (MinIO, LocalStack, etc.), set the `fakes3` option or
`fakes3` environment variable to the endpoint URL:

```javascript
var gun = Gun({
  s3: {
    bucket: 'my-bucket',
    fakes3: 'http://localhost:9000'
  }
});
```

When `fakes3` is active:
- SSL is disabled (`sslEnabled: false`)
- Dots in bucket names are replaced with `p` (e.g., `my.bucket` becomes `mybucket`)
- The endpoint is set to the fakes3 URL

### S3 adapter internals (`rs3.js`)

The S3 adapter implements `put`, `get`, and `list`:

- **put:** Calls `s3.putObject` with `{Bucket, Key, Body}`. Maintains an in-flight
  write cache (`c.p`) so that concurrent `get` calls for the same key return the
  pending data.
- **get:** Calls `s3.getObject`. Coalesces concurrent reads for the same key into a
  single S3 request (using a callback queue `c.g`). Returns the `Body` buffer.
  Treats `NoSuchKey` errors as "not found" (no error).
- **list:** Calls `s3.listObjectsV2` with automatic pagination via
  `ContinuationToken` for truncated results. Calls the callback once per key, then
  once with no arguments to signal completion.

```javascript
// S3 adapter usage (automatic via Gun.on('create'))
// Just set env vars and create Gun:
// AWS_S3_BUCKET=my-bucket AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=... node app.js
var gun = Gun();
// S3 is now the storage backend
```

### Filesystem hybrid

If `opt.rfs !== false`, the S3 adapter also loads `rfsmix.js` which provides a
filesystem cache layer in front of S3, reducing S3 API calls for frequently accessed
chunks.

---

## Using RAD Without GUN

RAD can be used as a standalone persistent radix tree without GUN's graph layer:

```javascript
// Node.js
var Radisk = require('gun/lib/radisk');
var Store = require('gun/lib/rfs');

// Create storage adapter
var store = Store({ file: 'my-radix-data' });

// Create RAD instance
var rad = Radisk({
  store: store,
  file: 'my-radix-data',
  chunk: 1024 * 1024,    // 1 MB chunks
  until: 250,            // 250ms flush delay
});

// Write data
rad('users/alice', 'admin', function(err, ok) {
  if (err) { console.error('Write failed:', err); return; }
  console.log('Written successfully');
});

// Read exact key
rad('users/alice', function(err, data, info) {
  console.log(data); // "admin"
  console.log(info); // { parsed: ..., chunks: ..., more: 0, next: undefined }
});

// Read prefix (returns subtree)
rad('users/', function(err, tree, info) {
  // tree is a radix subtree -- iterate with Radix.map
  var Radix = Radisk.Radix;
  Radix.map(tree, function(value, key) {
    console.log(key, '=', value);
  });
});

// Range query
var opt = { start: 'users/a', end: 'users/m' };
rad('', function(err, tree, info) {
  Radisk.Radix.map(tree, function(value, key) {
    console.log(key, '=', value);
  }, opt);
}, opt);
```

### Accepted value types

RAD accepts only atomic values:

| Type | Example | Notes |
|------|---------|-------|
| `string` | `'hello'` | Any string |
| `number` | `42`, `3.14` | Any finite number |
| `boolean` | `true`, `false` | -- |
| `null` | `null` | Represents deletion in GUN |
| Soul link | `{'#': 'soul-id'}` | GUN node reference (single `#` key) |

Objects (other than soul links), arrays, `undefined`, and functions are NOT
supported and will produce undefined behavior.

---

## Installation

### Node.js

RAD is the default storage adapter. Just `require('gun')` -- it automatically uses
the filesystem adapter writing to `./radata/`. If `AWS_S3_BUCKET` is set in the
environment, S3 is used instead (requires `aws-sdk` in your `package.json`).

### Browser

```html
<script src="https://cdn.jsdelivr.net/npm/gun/gun.js"></script>
<script src="https://cdn.jsdelivr.net/npm/gun/lib/radix.js"></script>
<script src="https://cdn.jsdelivr.net/npm/gun/lib/radisk.js"></script>
<script src="https://cdn.jsdelivr.net/npm/gun/lib/store.js"></script>
<script src="https://cdn.jsdelivr.net/npm/gun/lib/rindexed.js"></script>
<script>
  var gun = Gun();
  // Automatically uses IndexedDB
</script>
```

Disable localStorage with `Gun({ localStorage: false })`.

### Disabling RAD

```javascript
var gun = Gun({ rad: false });
// or via environment variable:
// RAD=false node app.js
```

When RAD is disabled, GUN operates as an in-memory-only database with no
persistence. The `process.env.RAD` check is in `store.js`:

```javascript
if ((u + '' != typeof process) && 'false' === '' + (process.env || '').RAD) { return; }
```
