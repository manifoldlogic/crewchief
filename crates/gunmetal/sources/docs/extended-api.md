# GUN Extended API

GUN ships with a set of plugin methods that extend the core API. Unlike core methods such as `.get()`, `.put()`, `.on()`, `.once()`, `.set()`, and `.map()`, these extended methods are **not loaded by default**. Each one must be explicitly required before use.

```js
// Node.js / bundler
require('gun/lib/path.js');

// Browser
<script src="https://cdn.jsdelivr.net/npm/gun/lib/path.js"></script>
```

> **Important**: All of the methods documented here follow this pattern. You must include the corresponding `gun/lib/<name>.js` file before calling the method, or it will be `undefined` on the chain.

---

## Table of Contents

- [gun.path(key)](#gunpathkey)
- [gun.not(callback)](#gunnotcallback)
- [gun.open(callback, options?)](#gunopencallback-options)
- [gun.load(callback, options?)](#gunloadcallback-options)
- [gun.then(callback?)](#gunthencallback)
- [gun.promise(callback?)](#gunpromisecallback)
- [gun.bye()](#gunbye)
- [gun.later(callback, seconds)](#gunlatercallback-seconds)
- [gun.unset(node)](#gununsetnode)

---

## gun.path(key)

**Requires**: `require('gun/lib/path.js')`

A convenience wrapper around `.get()` that supports shorthand dot notation and array paths. It lets you traverse deeply nested graph structures in a single call instead of chaining multiple `.get()` calls.

### Signature

```ts
gun.path(key: string | string[] | number): IGunChain
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `key` | `string \| string[] \| number` | A dot-delimited path string, an array of path segments, or a single key. |

### Return Value

Returns a GUN chain reference pointing at the resolved path. The chain context changes to the final node in the path.

### Behavior

- **Dot notation**: A string containing dots is split on `.` and each segment becomes a chained `.get()` call.
- **Array notation**: Each element of the array becomes a `.get()` call.
- **Single key**: If the string contains no dots, it behaves identically to `.get(key)`.
- **Falsy but valid**: Passing `0` works (it is converted to the string `"0"`). Passing `null`, `undefined`, or empty string returns the current chain unchanged.
- **Non-string values**: Numbers and other types are converted to strings via `'' + field`.

### Gotcha: Decimal Numbers

Everything is converted to a string under the hood, including floating point numbers. A decimal point in a number is treated as a dot separator, splitting the path:

```js
gun.path(30.5);
// Equivalent to: gun.get('30').get('5')
// NOT: gun.get('30.5')
```

This can be especially confusing because the chain might never resolve to a value you expect.

### Custom Separator

The second argument to `.path()` allows specifying a custom separator instead of `.`:

```js
gun.path('themes/active/color', '/');
// Equivalent to: gun.get('themes').get('active').get('color')
```

### Examples

#### Basic dot notation

```js
require('gun/lib/path.js');

const gun = Gun();

// These two are equivalent:
gun.get('settings').path('themes.active');
gun.get('settings').get('themes').get('active');
```

#### Array notation

```js
const themeName = 'dark-mode';

// Use an array when path segments are dynamic or contain dots
gun.get('user').path(['themes', themeName]);
// Equivalent to: gun.get('user').get('themes').get('dark-mode')
```

#### Reading and writing through a path

```js
// Write a value through a path
gun.get('app').path('config.display.fontSize').put(16);

// Read a value through a path
gun.get('app').path('config.display.fontSize').once(function (data) {
  console.log('Font size:', data); // 16
});
```

#### Single-element array

```js
// A single-element array works the same as a plain .get()
gun.get('users').path(['alice']);
// Equivalent to: gun.get('users').get('alice')
```

---

## gun.not(callback)

**Requires**: `require('gun/lib/not.js')`

Handles cases where data cannot be found in the graph. The callback fires when there is reason to believe the requested data does not exist.

### Signature

```ts
gun.not(callback: (key: string) => void): IGunChain
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `callback` | `(key: string) => void` | Invoked with the key name if the data is not found. `this` inside the callback refers to the gun chain at that context. |

### Return Value

Returns the same GUN chain. Does **not** change the chain context.

### Caveats

- **No guarantees**: `.not()` has no guarantees, since data could theoretically exist on an unrelated peer that we have no knowledge of. If you only have one server and data is synced through it, you have a reasonable assurance that "not found" means the data does not exist yet.
- **Not a definitive check**: In a distributed system, absence of evidence is not evidence of absence. Use `.not()` as a hint, not as a guarantee.
- **`this` context**: Inside the callback, `this` refers to the gun chain, which lets you write data in response to the absence.

### Examples

#### Creating data if it does not exist

```js
require('gun/lib/not.js');

const gun = Gun();

gun.get('users').get('alice').not(function (key) {
  console.log(key, 'does not exist yet, creating...');
  // `this` is the gun chain reference
  this.put({
    name: 'Alice',
    age: 30,
    created: Date.now()
  });
});
```

#### Guard against duplicate creation

```js
gun.get('posts').get('welcome-post').not(function (key) {
  // Only create the welcome post if it does not already exist
  this.put({
    title: 'Welcome!',
    body: 'This is the first post.',
    timestamp: Date.now()
  });
}).once(function (data) {
  console.log('Post data:', data);
});
```

#### Using with user input

```js
function findOrCreatePlayer(playerName) {
  gun.get('players').get(playerName).not(function (key) {
    console.log('Player "' + key + '" not found. Creating new player.');
    this.put({
      name: key,
      score: 0,
      joinedAt: Date.now()
    });
  }).once(function (data) {
    console.log('Player loaded:', data);
  });
}

findOrCreatePlayer('bob');
```

---

## gun.open(callback, options?)

**Requires**: `require('gun/lib/open.js')`

Opens a live connection to a document and recursively loads its full depth on every update. Unlike `.on()`, which gives you the immediate node with GUN metadata, `.open()` gives you a **deep copy** of the entire document tree with all metadata stripped.

### Signature

```ts
gun.open(
  callback: (data: any) => void,
  options?: { wait?: number; meta?: boolean; off?: boolean; depth?: number }
): IGunChain
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `callback` | `(data: any) => void` | Called every time any update occurs anywhere in the full depth of the data. Receives a clean copy with no GUN metadata (`_` fields removed). |
| `options` | `object` | Optional configuration. |
| `options.wait` | `number` | Milliseconds to wait before calling the callback after changes arrive. Acts as a debounce timer so rapid updates are batched. Default: `9`. |
| `options.meta` | `boolean` | If `true`, include GUN metadata (`_` fields) in the output. Default: `false`. |
| `options.off` | `boolean` | If `true`, unsubscribe after the first callback (making it behave like `.load()`). Default: `false`. |
| `options.depth` | `number` | Maximum recursion depth. Limits how deep the document loading goes. Sub-documents beyond this depth appear as GUN references rather than resolved objects. |

### Return Value

Returns the same GUN chain. Does **not** change the chain context.

### Behavior

- Subscribes to the current node and recursively follows all references to sub-documents.
- On every change at any depth, re-invokes the callback with the full document tree.
- Handles circular references: if the same node is encountered again, it links to the already-loaded in-memory object rather than creating an infinite loop.
- Uses a debounce timer (`options.wait`) so that bursts of changes result in a single callback invocation.

### Warning

This automatically loads everything it can find on the context. While convenient, this may be unnecessary and excessive for large data sets, resulting in more bandwidth and slower load times. If your app is highly interconnected, it could attempt to load your entire database.

### Examples

#### Basic usage

```js
require('gun/lib/open.js');

const gun = Gun();

// Populate some nested data
gun.get('users').get('alice').put({
  name: 'Alice',
  address: {
    city: 'Wonderland',
    zip: '00000'
  }
});

// Open gives you the full depth, live
gun.get('users').get('alice').open(function (data) {
  console.log(data);
  // {
  //   name: 'Alice',
  //   address: {
  //     city: 'Wonderland',
  //     zip: '00000'
  //   }
  // }
  // No metadata like { _: { '#': '...', '>': { ... } } }
});
```

#### Adjusting the wait time

```js
// Wait 500ms after the last change before calling the callback
gun.get('game').get('state').open(function (data) {
  console.log('Game state updated:', data);
}, { wait: 500 });
```

#### Circular references

```js
const alice = gun.get('alice').put({ name: 'Alice' });
const bob = gun.get('bob').put({ name: 'Bob' });

// Create a circular reference
alice.get('friend').put(bob);
bob.get('friend').put(alice);

gun.get('alice').open(function (data) {
  console.log(data);
  // {
  //   name: 'Alice',
  //   friend: {
  //     name: 'Bob',
  //     friend: <reference back to the Alice object in memory>
  //   }
  // }
  // The circular reference points to the same JS object, not an infinite copy
  console.log(data.friend.friend === data); // true
});
```

#### Limiting depth

```js
gun.get('organization').open(function (data) {
  console.log(data);
  // Only loads 2 levels deep; deeper nodes appear as GUN soul references
}, { depth: 2 });
```

---

## gun.load(callback, options?)

**Requires**: `require('gun/lib/load.js')`

Works exactly like `.open()` but fires only once, combining the recursive full-depth loading of `.open()` with the single-invocation behavior of `.once()`. Internally, it calls `.open()` with the `off` option set to `true`.

### Signature

```ts
gun.load(
  callback: (data: any) => void,
  options?: { wait?: number; meta?: boolean; depth?: number }
): IGunChain
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `callback` | `(data: any) => void` | Called once with the fully loaded document tree (metadata stripped). |
| `options` | `object` | Same options as `.open()` (except `off`, which is always `true`). |

### Return Value

Returns the same GUN chain. Does **not** change the chain context.

### Dependency

`.load()` depends on `.open()`. The `load.js` module automatically requires `open.js` if it is not already loaded.

### Examples

#### Load a full document once

```js
require('gun/lib/load.js');

const gun = Gun();

gun.get('users').get('alice').put({
  name: 'Alice',
  profile: {
    bio: 'Curiouser and curiouser',
    avatar: 'alice.png'
  }
});

gun.get('users').get('alice').load(function (data) {
  console.log(data);
  // {
  //   name: 'Alice',
  //   profile: {
  //     bio: 'Curiouser and curiouser',
  //     avatar: 'alice.png'
  //   }
  // }
  // This callback fires only once, unlike .open()
});
```

#### Load with custom wait time

```js
gun.get('config').load(function (data) {
  console.log('Configuration:', data);
}, { wait: 200 });
```

#### Difference from open

```js
require('gun/lib/open.js');
require('gun/lib/load.js');

const ref = gun.get('counter').put({ value: 0 });

// .open() - fires on every update
ref.open(function (data) {
  console.log('open:', data.value); // Fires multiple times: 0, 1, 2, ...
});

// .load() - fires only once
ref.load(function (data) {
  console.log('load:', data.value); // Fires once: 0
});

// Subsequent updates
ref.put({ value: 1 });
ref.put({ value: 2 });
```

---

## gun.then(callback?)

**Requires**: `require('gun/lib/then.js')`

Converts a GUN chain into a JavaScript `Promise` that resolves with the data. This enables use with `async`/`await` and standard Promise patterns.

### Signature

```ts
gun.then(callback?: (data: any) => any): Promise<any>
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `callback` | `(data: any) => any` | Optional. If provided, it is passed to `Promise.then()` after resolution. Equivalent to calling `gun.then().then(callback)`. |

### Return Value

Returns a `Promise` that resolves with the data at the current chain context. The resolved value is the same as what `.once()` would return.

**After calling `.then()`, you are no longer in a GUN chain** -- you are in Promise-land. You cannot chain GUN methods after `.then()`, but you can chain `.then()`, `.catch()`, and `.finally()` as with any Promise.

### Important Notes

- A GUN chain is **not** already a Promise. You must explicitly call `.then()` to promisify it.
- `.then()` uses `.once()` internally, so it resolves with a single snapshot of the data.
- After resolution, the Promise follows standard JavaScript Promise semantics.

### Examples

#### Basic usage with .then()

```js
require('gun/lib/then.js');

const gun = Gun();
gun.get('greeting').put({ message: 'Hello, World!' });

gun.get('greeting').then(function (data) {
  console.log(data.message); // 'Hello, World!'
});
```

#### With async/await

```js
require('gun/lib/then.js');

const gun = Gun();
gun.get('users').get('alice').put({ name: 'Alice', score: 42 });

async function getUser() {
  const data = await gun.get('users').get('alice').then();
  console.log(data.name);  // 'Alice'
  console.log(data.score); // 42
  return data;
}

getUser();
```

#### Promise.race -- timeout pattern

```js
require('gun/lib/then.js');

const gun = Gun();

function timeout(ms) {
  return new Promise(function (_, reject) {
    setTimeout(function () {
      reject(new Error('Timed out after ' + ms + 'ms'));
    }, ms);
  });
}

Promise.race([
  gun.get('slow-peer-data').then(),
  timeout(5000)
]).then(function (data) {
  console.log('Got data:', data);
}).catch(function (err) {
  console.error(err.message); // 'Timed out after 5000ms'
});
```

#### Chaining Promises (not GUN chains)

```js
require('gun/lib/then.js');

gun.get('users').get('alice')
  .then()
  .then(function (data) {
    console.log('Step 1:', data.name);
    return data.name.toUpperCase();
  })
  .then(function (uppercased) {
    console.log('Step 2:', uppercased); // 'ALICE'
  })
  .catch(function (err) {
    console.error('Error:', err);
  });
```

---

## gun.promise(callback?)

**Requires**: `require('gun/lib/then.js')`

Similar to `.then()` but resolves with a richer context object instead of just the data. This is useful when you need the key, data, and a reference to the gun chain all at once.

### Signature

```ts
gun.promise(callback?: (ctx: { put: any, get: string, gun: IGunChain }) => any): Promise<{ put: any, get: string, gun: IGunChain }>
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `callback` | `(ctx: object) => any` | Optional. Called with the resolved context object. If omitted, returns the raw Promise. |

### Resolved Value

The Promise resolves with an object containing:

| Property | Type | Description |
|----------|------|-------------|
| `put` | `any` | The data at the current node (same as what `.then()` resolves with). |
| `get` | `string` | The key/soul of the node. |
| `gun` | `IGunChain` | A reference back to the GUN chain, allowing you to continue working with GUN after the Promise resolves. |

### Examples

#### Basic usage

```js
require('gun/lib/then.js');

const gun = Gun();
gun.get('settings').put({ theme: 'dark', lang: 'en' });

gun.get('settings').promise().then(function (ctx) {
  console.log(ctx.put);  // { theme: 'dark', lang: 'en', _: {...} }
  console.log(ctx.get);  // 'settings'
  console.log(ctx.gun);  // GUN chain reference
});
```

#### With async/await

```js
require('gun/lib/then.js');

const gun = Gun();
gun.get('items').get('item1').put({ name: 'Widget', price: 9.99 });

async function inspectNode() {
  const { put, get, gun: ref } = await gun.get('items').get('item1').promise();

  console.log('Key:', get);       // 'item1'
  console.log('Data:', put);      // { name: 'Widget', price: 9.99, ... }

  // Use the gun reference to write back
  ref.get('lastAccessed').put(Date.now());
}

inspectNode();
```

#### With a callback

```js
require('gun/lib/then.js');

gun.get('users').get('bob').promise(function (ctx) {
  console.log('User key:', ctx.get);
  console.log('User data:', ctx.put);
  return ctx.put.name;
}).then(function (name) {
  console.log('Name is:', name);
});
```

---

## gun.bye()

**Requires**: `require('gun/lib/bye.js')`

Schedules data changes to be applied after a browser peer disconnects. This is useful for cleanup tasks like marking a user offline or removing them from an active players list.

### Signature

```ts
gun.bye(): { put: (data: any) => IGunChain }
```

### Return Value

Returns a special context that exposes **only** a `.put()` method. The `.put()` call does not write immediately -- it registers data to be written when the peer disconnects.

### Behavior

- When called, it creates a "bye" message containing the data you want to write.
- The bye message is sent to the server/relay peers.
- When a peer disconnects (e.g., browser tab closes), the server processes the bye messages and applies the `.put()` operations.
- **Runs on the server, not the browser**. The browser registers the intent; the server executes it upon disconnect.

### Status

**Experimental / Alpha**. The API and behavior may change.

### Examples

#### Mark user as offline on disconnect

```js
require('gun/lib/bye.js');

const gun = Gun('https://my-relay.example.com/gun');

const userId = 'alice';

// When this peer connects, mark as online
gun.get('users').get(userId).get('status').put('online');

// Schedule: when this peer disconnects, mark as offline
gun.get('users').get(userId).get('status').bye().put('offline');
```

#### Remove player from a game lobby

```js
require('gun/lib/bye.js');

const gun = Gun('https://game-server.example.com/gun');
const playerId = 'player_42';

// Add player to active game
gun.get('game').get('lobby').get(playerId).put({
  name: 'SpacePilot42',
  joinedAt: Date.now()
});

// When player disconnects, null out their entry
gun.get('game').get('lobby').get(playerId).bye().put(null);
```

#### Set a last-seen timestamp

```js
require('gun/lib/bye.js');

const gun = Gun('https://relay.example.com/gun');

// On disconnect, record the time the user was last seen
gun.get('users').get('alice').get('lastSeen').bye().put(Date.now());
```

---

## gun.later(callback, seconds)

**Requires**: `require('gun/lib/later.js')`

Executes a callback after a specified delay, providing a safe snapshot of the data at that point. Useful for implementing TTL (time-to-live), expiration, delayed refresh, or deferred processing.

### Signature

```ts
gun.later(
  callback: (data: any, key: string) => void,
  seconds: number
): IGunChain
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `callback` | `(data: any, key: string) => void` | Called after the delay with a full-depth snapshot of the data (via `.open()`) and the key. `this` inside the callback refers to the gun chain. |
| `seconds` | `number` | Number of seconds to wait before firing. Converted to milliseconds internally. |

### Return Value

Returns the same GUN chain. Does **not** change the chain context.

### Behavior

- Uses `setTimeout` internally, so **exact timing is not guaranteed**.
- After the timeout fires, it calls `.open()` with `{ off: true }` to get a one-time full-depth snapshot, adding at least ~1ms to the effective delay.
- **If the process or browser restarts, the timeout is lost** and the callback will never fire. There is no persistence of scheduled callbacks.

### Dependency

`.later()` depends on `.open()`. The `later.js` module automatically requires `open.js` if it is not already loaded.

### Examples

#### Basic TTL / expiration

```js
require('gun/lib/later.js');

const gun = Gun();

// Save a temporary token
gun.get('sessions').get('abc123').put({
  token: 'xyz-secret',
  createdAt: Date.now()
});

// Expire it after 60 seconds
gun.get('sessions').get('abc123').later(function (data, key) {
  console.log('Expiring session:', key);
  this.put(null); // `this` is the gun reference -- nulls out the session
}, 60);
```

#### Delayed notification

```js
require('gun/lib/later.js');

const gun = Gun();

gun.get('reminders').get('meeting').put({
  title: 'Team standup',
  time: '10:00 AM'
});

// Remind in 10 seconds
gun.get('reminders').get('meeting').later(function (data, key) {
  console.log('REMINDER:', data.title, 'at', data.time);
  // Output after 10s: REMINDER: Team standup at 10:00 AM
}, 10);
```

#### Refresh cached data periodically

```js
require('gun/lib/later.js');

const gun = Gun();

function refreshWeather() {
  gun.get('cache').get('weather').put({
    temp: Math.round(Math.random() * 100),
    updatedAt: Date.now()
  });

  // Schedule the next refresh in 300 seconds (5 minutes)
  gun.get('cache').get('weather').later(function (data, key) {
    console.log('Refreshing weather data. Old value:', data.temp);
    refreshWeather(); // re-invoke to create a recurring pattern
  }, 300);
}

refreshWeather();
```

---

## gun.unset(node)

**Requires**: `require('gun/lib/unset.js')`

Removes an item from an unordered list. This is the inverse of `.set()`, which adds a node to an unordered set. Internally, it nulls out the reference to the node within the set.

### Signature

```ts
gun.unset(node: IGunChain): IGunChain
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `node` | `IGunChain` | A GUN chain reference to the node you want to remove from the set. This must be a reference that was previously added via `.set()`. |

### Return Value

Returns the same GUN chain (`this`). Does **not** change the chain context.

### How It Works

When you `.set(item)` on a node, GUN creates a reference in an unordered list using the item's soul (unique ID). Calling `.unset(item)` finds that soul and writes `null` to its key in the parent set, effectively removing the link.

### Examples

#### Add and remove from a set

```js
require('gun/lib/unset.js');

const gun = Gun();

const cats = gun.get('pets').get('cats');
const fluffy = gun.get('fluffy').put({ name: 'Fluffy', color: 'white' });
const whiskers = gun.get('whiskers').put({ name: 'Whiskers', color: 'orange' });

// Add to the set
cats.set(fluffy);
cats.set(whiskers);

// List current cats
cats.map().once(function (data, key) {
  console.log('Cat:', data.name);
});
// Output:
// Cat: Fluffy
// Cat: Whiskers

// Remove Fluffy from the set
cats.unset(fluffy);

// Now only Whiskers remains
cats.map().once(function (data, key) {
  console.log('Cat:', data.name);
});
// Output:
// Cat: Whiskers
```

#### Todo list with add/remove

```js
require('gun/lib/unset.js');

const gun = Gun();
const todos = gun.get('todos');

// Add items
const task1 = gun.get('task1').put({ text: 'Buy groceries', done: false });
const task2 = gun.get('task2').put({ text: 'Walk the dog', done: false });
const task3 = gun.get('task3').put({ text: 'Write docs', done: false });

todos.set(task1);
todos.set(task2);
todos.set(task3);

// Complete and remove a task
function completeTask(taskRef) {
  taskRef.put({ done: true });
  todos.unset(taskRef);
}

completeTask(task2);

// Remaining todos
todos.map().once(function (data) {
  console.log(data.text, '- done:', data.done);
});
// Output:
// Buy groceries - done: false
// Write docs - done: false
```

#### Removing a user from a group

```js
require('gun/lib/unset.js');

const gun = Gun();
const group = gun.get('groups').get('admins');

const alice = gun.get('alice').put({ name: 'Alice', role: 'admin' });
const bob = gun.get('bob').put({ name: 'Bob', role: 'admin' });

group.set(alice);
group.set(bob);

// Remove Bob from admins
group.unset(bob);
```

---

## Summary

| Method | Require Path | Behavior | Chain Context |
|--------|-------------|----------|---------------|
| `.path(key)` | `gun/lib/path.js` | Shorthand for nested `.get()` calls via dot or array notation | **Changes** to target node |
| `.not(cb)` | `gun/lib/not.js` | Fires callback if data is not found | Unchanged |
| `.open(cb, opt?)` | `gun/lib/open.js` | Live full-depth document loading on every update | Unchanged |
| `.load(cb, opt?)` | `gun/lib/load.js` | Full-depth document loading, fires once | Unchanged |
| `.then(cb?)` | `gun/lib/then.js` | Promisifies the chain (one-shot via `.once()`) | Exits GUN chain, enters Promise chain |
| `.promise(cb?)` | `gun/lib/then.js` | Like `.then()` but resolves `{ put, get, gun }` | Exits GUN chain, enters Promise chain |
| `.bye()` | `gun/lib/bye.js` | Schedules data writes for peer disconnect | Returns special context with only `.put()` |
| `.later(cb, sec)` | `gun/lib/later.js` | Fires callback after delay with full-depth snapshot | Unchanged |
| `.unset(node)` | `gun/lib/unset.js` | Removes a node from an unordered set | Unchanged |

### Dependencies Between Extensions

Some extensions depend on others:

```
gun/lib/load.js  --> gun/lib/open.js
gun/lib/later.js --> gun/lib/open.js
gun/lib/then.js  (provides both .then() and .promise())
```

If you require `load.js` or `later.js`, they will automatically require `open.js` for you. However, explicitly requiring all dependencies is recommended for clarity.
