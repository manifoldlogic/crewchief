# GUN Utilities and Node Helpers

GUN ships with a set of utility functions for inspecting, creating, and manipulating nodes, as well as global extensions to `String`, `Object`, and `setTimeout` that support its internal operations. These utilities are available on the `Gun` constructor and are also used throughout GUN's internals for conflict resolution, soul management, and scheduling.

---

## Gun.node.is(data)

Returns `true` if `data` is a GUN node, `false` otherwise. A GUN node is defined as a plain object that contains a `_` metadata property, which itself contains a `#` key holding the node's soul (its unique identifier in the graph).

This is the canonical way to distinguish GUN nodes from ordinary JavaScript objects.

```js
// A valid GUN node
const node = {
  _: { '#': 'abc123', '>': { name: 1234567890 } },
  name: 'Alice'
};
Gun.node.is(node); // true

// A plain object is not a node
Gun.node.is({ name: 'Alice' }); // false

// Primitives are not nodes
Gun.node.is('hello'); // false
Gun.node.is(null);    // false
```

---

## Gun.node.soul(data)

> **DEPRECATED** -- Use `data._['#']` directly instead. This helper only works for objects and adds unnecessary indirection.

Returns the soul (unique identifier) of a GUN node. The soul is the string stored at `data._['#']` and serves as the node's address in the graph.

```js
gun.get('test').get('node').once(function (data) {
  // Preferred: access soul directly
  console.log(data._['#']); // e.g. "test/node"

  // Deprecated helper (equivalent)
  console.log(Gun.node.soul(data)); // e.g. "test/node"
});
```

If the argument is not a valid GUN node, `Gun.node.soul()` returns `undefined`.

---

## Gun.node.ify(json)

Converts a plain JSON object into a GUN node by injecting GUN metadata (the `_` property with a generated soul and state vectors). Returns the GUN-ified variant of the input.

This is useful when you need to manually construct nodes for batch operations or when interfacing with external data sources.

```js
const plain = { name: 'Bob', age: 30 };
const node = Gun.node.ify(plain);

console.log(node);
// {
//   _: { '#': '<generated-soul>', '>': { name: <timestamp>, age: <timestamp> } },
//   name: 'Bob',
//   age: 30
// }

// The result passes the node check
Gun.node.is(node); // true
```

You can also pass an existing soul to preserve identity:

```js
const node = Gun.node.ify({ title: 'Hello' }, { soul: 'posts/1' });
console.log(node._['#']); // "posts/1"
```

---

## Gun.state()

Returns GUN's current state timestamp as a number. This timestamp is used internally by the HAM (Hypothetical Amnesia Machine) conflict resolution algorithm to determine which write wins when concurrent updates occur.

```js
const timestamp = Gun.state();
console.log(timestamp); // e.g. 1700000000000
console.log(typeof timestamp); // "number"
```

The state value corresponds to milliseconds since epoch and is used to annotate every property update in the `>` state vector of a node's metadata.

---

## String Utilities (Global Extensions)

GUN extends the global `String` constructor with several utility methods used internally and available for application code.

### String.random(length?, alphabet?)

Generates a random string of the specified length using the given alphabet. Used internally for soul generation and unique ID creation.

- **length** (number, optional): Length of the generated string. Default: `24`.
- **alphabet** (string, optional): Characters to draw from. Default: alphanumeric (`0-9`, `A-Z`, `a-z`).

```js
// Default: 24-character alphanumeric string
String.random();
// e.g. "aB3kZ9mN2pQ7rT5wX1yC4eG"

// Custom length
String.random(8);
// e.g. "kZ9mN2pQ"

// Custom alphabet
String.random(6, 'abc');
// e.g. "bcaacb"

// Hex string
String.random(16, '0123456789abcdef');
// e.g. "a3f29c8b01e74d56"
```

### String.match(text, options)

Pattern matching utility used by GUN's LEX (Lexical Expression) query system. Returns `true` if the text matches the given pattern, `false` otherwise.

- **text** (string): The string to test.
- **options** (LEX object or string): Match criteria. A plain string performs an exact match. A LEX object supports structured queries.

Supported LEX operators:
- `'='` -- Exact match
- `'*'` -- Prefix match (text starts with the given value)
- `'>'` -- Greater than or equal (lexicographic)
- `'<'` -- Less than or equal (lexicographic)

```js
// Exact match with string
String.match('hello', 'hello'); // true
String.match('hello', 'world'); // false

// Exact match with LEX
String.match('hello', { '=': 'hello' }); // true

// Prefix match
String.match('username_alice', { '*': 'username_' }); // true
String.match('post_123', { '*': 'username_' });        // false

// Range match
String.match('b', { '>': 'a', '<': 'c' }); // true
String.match('d', { '>': 'a', '<': 'c' }); // false
```

### String.hash(string, seed?)

Hashes a string to a numeric value. Used internally for various operations including partitioning and routing.

- **string** (string): The input to hash.
- **seed** (number, optional): Seed value to vary the hash output.

```js
const hash = String.hash('hello');
console.log(typeof hash); // "number"

// Same input always produces same output
String.hash('hello') === String.hash('hello'); // true

// Seed changes the result
String.hash('hello', 1) !== String.hash('hello', 2); // true
```

---

## Object Utilities (Global Extensions)

GUN extends the global `Object` constructor with helpers for inspecting plain objects.

### Object.plain(o)

Returns `true` if `o` is a plain object (i.e., its constructor is `Object`). Returns `false` for class instances, arrays, `null`, and primitives.

```js
Object.plain({});                    // true
Object.plain({ a: 1 });             // true
Object.plain(new Date());           // false
Object.plain([]);                    // false
Object.plain(null);                  // false
Object.plain('string');              // false
```

### Object.empty(o, exclude?)

Returns `true` if the object has no own enumerable properties. Optionally accepts an array of keys to exclude from the check (i.e., those keys are ignored when determining emptiness).

```js
Object.empty({});                        // true
Object.empty({ a: 1 });                 // false

// Exclude specific keys
Object.empty({ _: 'meta' }, ['_']);      // true (only key is excluded)
Object.empty({ _: 'meta', a: 1 }, ['_']); // false (a is not excluded)
```

---

## Scheduling Utilities

GUN includes internal scheduling utilities attached to the global `setTimeout` function. These provide efficient, non-blocking iteration and scheduling that avoids the minimum 4ms delay of standard `setTimeout(fn, 0)`.

### setTimeout.poll(f)

Registers a function for efficient polling that respects frame timing. Uses `MessageChannel` internally for faster-than-setTimeout scheduling when available. The hold time (how long a polling cycle runs before yielding) is configurable via `setTimeout.hold`.

- **f** (function): The function to poll.
- **setTimeout.hold** (number): Maximum milliseconds per cycle. Default: `9`.

```js
setTimeout.poll(function () {
  // This runs with minimal delay, faster than setTimeout(fn, 0)
  processNextItem();
});

// Adjust hold time for different workloads
setTimeout.hold = 4; // Yield more frequently (smoother UI)
```

### setTimeout.turn(f)

Thread-like turn-based scheduling. Queues a function to run on the next available turn, preventing any single poll cycle from blocking. Functions are cycled in turns over a single thread, ensuring fair scheduling.

```js
setTimeout.turn(function () {
  // Runs on the next turn, won't block other turns
  doExpensiveWork();
});
```

### setTimeout.each(list, fn, end, size?)

Iterates over a list using turn-based scheduling to prevent blocking. Each batch processes `size` items before yielding to the next turn. Calls `end` when all items have been processed.

- **list** (array): The items to iterate over.
- **fn** (function): Called for each item with `(item, index, list)`.
- **end** (function): Called when iteration is complete.
- **size** (number, optional): Batch size per turn. Default: `9`.

```js
const items = [/* thousands of items */];

setTimeout.each(
  items,
  function (item, index) {
    // Process each item without blocking
    processItem(item);
  },
  function () {
    // All items processed
    console.log('Done processing', items.length, 'items');
  },
  50 // Process 50 items per turn
);
```

This is particularly useful when processing large datasets in environments where blocking the event loop would degrade user experience (e.g., browser UIs or real-time peer connections).
