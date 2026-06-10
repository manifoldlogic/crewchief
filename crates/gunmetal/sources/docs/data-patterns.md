# GUN Data Patterns and Recipes

Practical patterns for modeling data in GUN -- from simple key/value pairs through
relational graphs, collections, counters, deletion, time-series data, and common
application schemas. Every pattern includes runnable JavaScript examples.

---

## Table of Contents

- [Graph Data Model](#graph-data-model)
- [Partials and Merge Semantics](#partials-and-merge-semantics)
- [Circular References](#circular-references)
- [Modeling Relationships](#modeling-relationships)
- [Tables and Collections](#tables-and-collections)
- [Counters and Incrementing](#counters-and-incrementing)
- [Deleting Data](#deleting-data)
- [Timestamps and Time-Series Data](#timestamps-and-time-series-data)
- [Immutable / Content-Addressed Data](#immutable--content-addressed-data)
- [Common Schema Patterns](#common-schema-patterns)

---

## Graph Data Model

GUN stores all data as a graph of **nodes**. Unlike document databases (MongoDB,
CouchDB) that store self-contained JSON trees, or relational databases that store
rows in fixed-schema tables, GUN stores a flat collection of nodes that reference
each other through **soul pointers**.

### Nodes and Souls

Every node in the graph has a unique identifier called a **soul** (stored under the
`_` metadata key as `#`). When you call `gun.get('person/alice')`, the string
`'person/alice'` is the soul. Nested objects are automatically split into separate
nodes with generated souls.

```javascript
var gun = Gun();

// This creates a node with soul 'person/alice'
gun.get('person/alice').put({
  name: 'Alice',
  age: 30,
  address: {
    city: 'Portland',
    state: 'OR'
  }
});
```

Internally, GUN stores this as two nodes:

```javascript
// Node 1: soul = 'person/alice'
{
  "_": { "#": "person/alice", ">": { "name": 1713168000000, "age": 1713168000000, "address": 1713168000000 } },
  "name": "Alice",
  "age": 30,
  "address": { "#": "auto-generated-soul-xyz" }  // reference, not inline data
}

// Node 2: soul = 'auto-generated-soul-xyz'
{
  "_": { "#": "auto-generated-soul-xyz", ">": { "city": 1713168000000, "state": 1713168000000 } },
  "city": "Portland",
  "state": "OR"
}
```

### References vs. Values

GUN nodes can hold:

- **Primitive values**: strings, numbers, booleans, null
- **References**: pointers to other nodes via `{ "#": "soul" }`

GUN does **not** store arrays, functions, or deeply nested objects as single values.
Objects are always decomposed into separate nodes linked by references.

### How This Differs from Other Databases

| Aspect | SQL | Document DB | GUN |
|--------|-----|-------------|-----|
| Structure | Fixed-schema tables | Nested JSON trees | Flat graph of nodes |
| Relationships | Foreign keys + JOINs | Embedded or manual refs | Native soul pointers |
| Circular refs | Possible via FKs | Not supported natively | First-class support |
| Partial updates | UPDATE SET col=val | Varies (often full replace) | Always partial by default |
| Schema | Required | Optional | None (convention-based) |

---

## Partials and Merge Semantics

GUN always performs **partial merges** when you call `.put()`. This is fundamental
to how its CRDT conflict resolution works and is one of the most important
behaviors to understand.

### Partial Updates Preserve Existing Data

In most databases, putting a new object replaces the old one entirely. In GUN,
`.put()` merges the new properties into the existing node:

```javascript
var gun = Gun();
var mark = gun.get('marknadal');

// First put
mark.put({
  username: 'marknadal',
  name: 'Mark Nadal',
  email: 'mark@gunDB.io'
});

// Second put -- this does NOT erase username, name, or email
mark.put({
  hacker: true,
  country: 'USA'
});

// The node now contains ALL five properties:
// { username: 'marknadal', name: 'Mark Nadal', email: 'mark@gunDB.io',
//   hacker: true, country: 'USA' }
mark.once(function (data) {
  console.log(data);
  // { username: 'marknadal', name: 'Mark Nadal', email: 'mark@gunDB.io',
  //   hacker: true, country: 'USA' }
});
```

This is not just a convenience -- it is essential for concurrent, peer-to-peer
operation. When two peers independently update different properties of the same
node, both updates merge cleanly without one overwriting the other.

### Why Partials Matter for CRDTs

Because GUN is a CRDT (Conflict-free Replicated Data Type), every property on a
node is independently versioned with a **state timestamp** (managed by the HAM
algorithm). When two peers send conflicting updates, GUN resolves them
property-by-property, not object-by-object. Partial merging makes this possible.

### Nested Objects Get Their Own Souls

When you put a nested object, GUN automatically creates a separate node for it
and links via a soul reference:

```javascript
var gun = Gun();

gun.get('company/acme').put({
  name: 'Acme Corp',
  address: {
    street: '123 Main St',
    city: 'Springfield'
  }
});

// The address is now its own node. To update it, traverse the chain:
gun.get('company/acme').get('address').put({
  zip: '62701'  // adds zip without erasing street or city
});
```

### Deep Updates Require Chain Traversal

Because nested objects become separate nodes, you cannot update a deep property
by putting a flat path from the root. You must traverse the chain:

```javascript
// WRONG -- this replaces the address reference, not the city inside it
gun.get('company/acme').put({
  address: {
    city: 'Shelbyville'
  }
});
// This creates a NEW address node with only city set

// CORRECT -- traverse into the existing address node
gun.get('company/acme').get('address').put({
  city: 'Shelbyville'  // updates city, preserves street and zip
});
```

### Explicit Replacement

If you genuinely want to replace all data on a node, you must explicitly null out
every property you want removed, then put the new data:

```javascript
// To fully replace, null out old properties explicitly
mark.put({
  username: null,
  name: null,
  email: null,
  hacker: null,
  country: null,
  newField: 'fresh start'
});
```

---

## Circular References

GUN natively supports circular references. This is possible because the graph is
stored as a flat collection of nodes with soul pointers -- there is no tree to
become infinite.

### The Problem in JSON

In plain JavaScript, circular references cause `JSON.stringify` to throw:

```javascript
var mark = { name: 'Mark Nadal' };
var cat = { name: 'Timber', human: mark };
mark.pet = cat;

JSON.stringify(mark); // TypeError: Converting circular structure to JSON
```

### How GUN Handles It

GUN represents references as soul pointers (`{ "#": "soul" }`), so circular
structures serialize cleanly:

```javascript
var gun = Gun();

// Create mark
var mark = gun.get('marknadal').put({
  name: 'Mark Nadal'
});

// Create the cat
var cat = gun.get('timber').put({
  name: 'Timber'
});

// Create circular references
cat.get('human').put(mark);
mark.get('pet').put(cat);

// Traverse the circle without errors
gun.get('marknadal').get('pet').get('human').get('name').once(function (name) {
  console.log("Mark's pet's human's name is " + name);
  // "Mark's pet's human's name is Mark Nadal"
});
```

### Internal Representation

The two nodes are stored as:

```javascript
{
  "marknadal": {
    "_": { "#": "marknadal" },
    "name": "Mark Nadal",
    "pet": { "#": "timber" }        // soul pointer, not embedded object
  },
  "timber": {
    "_": { "#": "timber" },
    "name": "Timber",
    "human": { "#": "marknadal" }   // points back -- circular, but flat
  }
}
```

### Practical Examples

**Social graph (mutual friends):**

```javascript
var gun = Gun();
var alice = gun.get('user/alice').put({ name: 'Alice' });
var bob = gun.get('user/bob').put({ name: 'Bob' });

// Mutual friendship
alice.get('friends').get('bob').put(bob);
bob.get('friends').get('alice').put(alice);

// Traverse: Alice -> friend Bob -> friend Alice -> name
gun.get('user/alice')
  .get('friends').get('bob')
  .get('friends').get('alice')
  .get('name')
  .once(function (name) {
    console.log(name); // "Alice"
  });
```

**Organizational chart (manager/report cycle):**

```javascript
var gun = Gun();

var ceo = gun.get('employee/ceo').put({ name: 'Dana', role: 'CEO' });
var vp = gun.get('employee/vp').put({ name: 'Eli', role: 'VP Engineering' });

ceo.get('reports').get('vp').put(vp);
vp.get('manager').put(ceo);

// The CEO's report's manager is the CEO
gun.get('employee/ceo')
  .get('reports').get('vp')
  .get('manager')
  .get('name')
  .once(function (name) {
    console.log(name); // "Dana"
  });
```

---

## Modeling Relationships

GUN supports one-to-one, one-to-many, and many-to-many relationships natively
through soul references and sets.

### One-to-One

Use `.put()` with a reference to another node:

```javascript
var gun = Gun();
var alice = gun.get('user/alice').put({ name: 'Alice' });
var bob = gun.get('user/bob').put({ name: 'Bob' });

// Alice's spouse is Bob (one-to-one)
alice.get('spouse').put(bob);

// Read it back
alice.get('spouse').get('name').once(function (name) {
  console.log(name); // "Bob"
});
```

To make it bidirectional, set both sides:

```javascript
alice.get('spouse').put(bob);
bob.get('spouse').put(alice);
```

### One-to-Many

Use `.set()` to add items to a collection. Each `.set()` call adds a reference
to the set without removing existing entries:

```javascript
var gun = Gun();
var company = gun.get('company/acme').put({ name: 'Acme Corp' });
var employees = company.get('employees');

var alice = gun.get('user/alice').put({ name: 'Alice' });
var bob = gun.get('user/bob').put({ name: 'Bob' });
var carl = gun.get('user/carl').put({ name: 'Carl' });

// Add each employee to the company's employee set
employees.set(alice);
employees.set(bob);
employees.set(carl);

// Point back from each employee to the company
alice.get('employer').put(company);
bob.get('employer').put(company);
carl.get('employer').put(company);

// List all employees
employees.map().once(function (employee) {
  console.log(employee.name);
  // "Alice", "Bob", "Carl" (order not guaranteed)
});
```

### Many-to-Many

Use `.set()` on both sides to create bidirectional many-to-many relationships:

```javascript
var gun = Gun();

// Students
var alice = gun.get('student/alice').put({ name: 'Alice' });
var bob = gun.get('student/bob').put({ name: 'Bob' });

// Courses
var math = gun.get('course/math').put({ name: 'Mathematics' });
var physics = gun.get('course/physics').put({ name: 'Physics' });

// Alice takes Math and Physics
alice.get('courses').set(math);
alice.get('courses').set(physics);
math.get('students').set(alice);
physics.get('students').set(alice);

// Bob takes Math
bob.get('courses').set(math);
math.get('students').set(bob);

// List all students in Math
math.get('students').map().once(function (student) {
  console.log(student.name); // "Alice", "Bob"
});

// List all courses Alice takes
alice.get('courses').map().once(function (course) {
  console.log(course.name); // "Mathematics", "Physics"
});
```

### Relationship Metadata (Edge Nodes)

If you need to store metadata about a relationship (e.g., when a friendship was
formed, or a role within a group), create an intermediate "edge" node:

```javascript
var gun = Gun();

var alice = gun.get('user/alice').put({ name: 'Alice' });
var bob = gun.get('user/bob').put({ name: 'Bob' });

// Create an edge node with metadata about the relationship
var friendship = gun.get('friendship/alice-bob').put({
  since: '2024-03-15',
  closeness: 'best friends'
});
friendship.get('person1').put(alice);
friendship.get('person2').put(bob);

// Link from both users to the edge
alice.get('friendships').set(friendship);
bob.get('friendships').set(friendship);

// Read the metadata
alice.get('friendships').map().once(function (edge) {
  console.log(edge.since); // "2024-03-15"
});
```

---

## Tables and Collections

GUN uses `.set()` to build unordered collections (analogous to tables in SQL or
collections in MongoDB). Combined with `.map()` for iteration and LEX queries
for filtering, you can model tabular data effectively.

### Creating a Collection with `.set()`

```javascript
var gun = Gun();
var people = gun.get('people');

// .set() inserts a node into the collection with an auto-generated key
people.set({ name: 'Alice', age: 22 });
people.set({ name: 'Bob', age: 24 });
people.set({ name: 'Carl', age: 16 });
people.set({ name: 'Dave', age: 42 });
```

Each `.set()` call creates a new node with a unique soul and adds a reference to
it under the `people` node. The keys are auto-generated (timestamp-based UUIDs),
so the collection is **unordered**.

### Iterating with `.map()`

`.map()` iterates over every item in a collection. It is also **reactive** -- it
fires for existing items and again whenever a new item is added or an existing
item is updated.

```javascript
// Iterate all people
people.map().once(function (person) {
  console.log(person.name + ' is ' + person.age + ' years old');
});
```

### Filtering with `.map(filterFn)`

When `.map()` receives a callback, it acts as a filter/transform. Return
`undefined` to skip an item:

```javascript
// Only adults (age >= 18)
people.map(function (person) {
  if (person && person.age >= 18) {
    return person;
  }
  // returning undefined skips this item
}).once(function (person) {
  console.log(person.name + ' is an adult');
});
```

### Accessing a Single Property Across All Items

Chain `.get()` after `.map()` to project a single field:

```javascript
// Get just the names
people.map().get('name').once(function (name) {
  console.log(name); // "Alice", "Bob", "Carl", "Dave"
});
```

### LEX Queries for Prefix Matching

The LEX (lexicographic) API lets you query keys by prefix, range, or pattern.
This is powerful for pagination and sorted access when you control the key
structure.

LEX operators:

| Operator | Meaning | Example |
|----------|---------|---------|
| `*` | Prefix match | `{ '*': 'user/' }` -- all keys starting with `user/` |
| `>` | Greater than or equal | `{ '>': 'a' }` -- keys >= `'a'` |
| `<` | Less than or equal | `{ '<': 'c' }` -- keys <= `'c'` |
| `%` | Limit (max results) | `{ '%': 10 }` -- return at most 10 |
| `-` | Reverse order | `{ '-': 1 }` -- iterate in reverse |
| `+` | Includes substring | `{ '+': 'foo' }` -- keys containing `'foo'` |

```javascript
var gun = Gun();

// Store items with structured keys for lexicographic querying
gun.get('directory').get('user/alice').put({ name: 'Alice' });
gun.get('directory').get('user/bob').put({ name: 'Bob' });
gun.get('directory').get('user/carl').put({ name: 'Carl' });
gun.get('directory').get('team/alpha').put({ name: 'Alpha Team' });
gun.get('directory').get('team/beta').put({ name: 'Beta Team' });

// Get all users (prefix match)
gun.get('directory').get({ '*': 'user/' }).map().once(function (val, key) {
  console.log(key, val);
});

// Get users in a range (alice through bob, inclusive)
gun.get('directory').get({ '>': 'user/alice', '<': 'user/bob~' }).map().once(function (val, key) {
  console.log(key, val);
});
```

### LEX with the Fluent Builder

GUN provides a fluent LEX builder via `.lex()`:

```javascript
var gun = Gun();

// Fluent LEX query
gun.get('directory')
  .lex()
  .prefix('user/')
  .limit(10)
  .map()
  .once(function (val, key) {
    console.log(key, val);
  });

// Range query with fluent API
gun.get('directory')
  .lex()
  .more('user/a')   // >= 'user/a'
  .less('user/d')   // <= 'user/d'
  .map()
  .once(function (val, key) {
    console.log(key, val);
  });
```

### Pagination with LEX

Combine range queries with limits for cursor-based pagination:

```javascript
var gun = Gun();
var pageSize = 10;

function loadPage(startAfter) {
  var query = { '%': pageSize };
  if (startAfter) {
    query['>'] = startAfter;
  } else {
    query['*'] = 'item/';  // prefix for all items
  }

  var lastKey = null;
  gun.get('items').get(query).map().once(function (val, key) {
    console.log(key, val);
    lastKey = key;
  });

  // To load the next page, call: loadPage(lastKey)
  return lastKey;
}

// Load first page
var cursor = loadPage(null);

// Load next page (after some user action)
// cursor = loadPage(cursor);
```

### Sorting with LEX Prefix Patterns

GUN does not have a built-in `ORDER BY`. Instead, design your keys so that
lexicographic order matches your desired sort order:

```javascript
var gun = Gun();
var tasks = gun.get('tasks');

// Pad numbers for correct lexicographic sorting
// "001", "002", ... "999" sort correctly as strings
function priorityKey(priority, id) {
  return String(priority).padStart(3, '0') + '/' + id;
}

tasks.get(priorityKey(1, 'urgent-fix')).put({ title: 'Urgent Fix', priority: 1 });
tasks.get(priorityKey(5, 'nice-to-have')).put({ title: 'Nice to Have', priority: 5 });
tasks.get(priorityKey(3, 'medium-task')).put({ title: 'Medium Task', priority: 3 });

// Iterating with prefix match yields items in priority order
tasks.get({ '*': '' }).map().once(function (val, key) {
  console.log(key, val.title);
  // "001/urgent-fix" -> "Urgent Fix"
  // "003/medium-task" -> "Medium Task"
  // "005/nice-to-have" -> "Nice to Have"
});
```

---

## Counters and Incrementing

GUN does not have an atomic increment operation. In a distributed CRDT system,
`counter += 1` from two peers simultaneously would conflict. Instead, use the
**distributed counter pattern**: each peer maintains its own counter node, and
the sum is computed on read.

### The CRDT Counter Extension

This extension can be implemented in just 12 lines:

```javascript
Gun.chain.count = function (num) {
  if (typeof num === 'number') {
    this.set(num);
  }
  if (typeof num === 'function') {
    var sum = 0;
    this.map().once(function (val) {
      num(sum += val);
    });
  }
  return this;
};
```

### Usage

```javascript
var gun = Gun();
var counter = gun.get('page-views');

// Increment by 5
counter.count(+5);

// Decrement by 8
counter.count(-8);

// Read the running total (callback fires as each value arrives)
counter.count(function (value) {
  console.log('Current total:', value);
  // prints: 5
  // prints: -3
});

// Add 10 more
counter.count(+10);
// prints: 7
```

### How It Works

Each call to `.count(n)` calls `.set(n)`, which inserts the number as a new entry
in an unordered set. Reading sums all entries. Because each `.set()` creates a
unique key, concurrent increments from multiple peers never conflict -- they each
add their own entry and the sum converges.

### Per-User Counter Pattern (Manual)

For finer control, use explicit per-user counter nodes:

```javascript
var gun = Gun();
var userId = 'user123'; // current user's ID

// Each user increments their own counter
gun.get('likes/post/abc').get(userId).put(
  // Merge the increment into the user's running total
  // You'd read the current value first in a real app
  1
);

// Sum all users' counters
function getTotalLikes(postId, cb) {
  var total = 0;
  gun.get('likes/post/' + postId).map().once(function (count) {
    if (typeof count === 'number') {
      total += count;
    }
    cb(total);
  });
}

getTotalLikes('abc', function (total) {
  console.log('Total likes:', total);
});
```

### Caveats

- The `.count()` extension calls the callback **incrementally** -- once per entry
  as it arrives. The last call has the final sum, but you do not know which call
  is "last" without additional logic (e.g., a debounce timer).
- For very high-frequency counters (thousands of increments per second), the set
  grows unbounded. Consider periodic compaction: sum all entries, write a new
  compacted node, and null out the old entries.

---

## Deleting Data

In a distributed CRDT, there is no true "delete." Data exists on multiple peers,
and a peer that was offline when a delete happened would re-sync the data when it
reconnects. GUN handles this with **nullification** (tombstoning).

### Setting Values to Null

The primary way to "delete" data in GUN is to set it to `null`:

```javascript
var gun = Gun();

// Create some data
gun.get('data').put({
  name: 'Alice',
  email: 'alice@example.com',
  temp: 'this will be removed'
});

// "Delete" the temp field by nullifying it
gun.get('data').put({ temp: null });

// The node now has: { name: 'Alice', email: 'alice@example.com', temp: null }
gun.get('data').once(function (data) {
  console.log(data.temp); // null
  console.log(data.name); // "Alice" -- other fields preserved
});
```

### Removing from a Set with `.unset()`

The `.unset()` method (from `lib/unset.js`) removes a node from a set by
nullifying the reference:

```javascript
// Requires: require('gun/lib/unset')
var gun = Gun();
var people = gun.get('people');

var alice = gun.get('user/alice').put({ name: 'Alice' });
var bob = gun.get('user/bob').put({ name: 'Bob' });

people.set(alice);
people.set(bob);

// Remove alice from the set
// Note: .unset() needs the chain reference to the item
alice.once(function () {
  people.unset(alice);
});
```

Under the hood, `.unset()` reads the soul from the node's metadata and puts
`null` at that key in the parent set:

```javascript
// What .unset() does internally:
// people.put({ 'soul-of-alice': null });
```

### Null Is Still Data

An important consequence: `null` is itself a value that syncs across peers.
Setting something to `null` does not remove it from storage -- it creates a new
state entry with the value `null`. The tombstone propagates to all peers.

```javascript
gun.get('data').get('temp').on(function (val) {
  // This WILL fire with val === null
  // If you're listening for changes, filter out nulls
  if (val === null) { return; }
  console.log('temp is:', val);
});
```

### Breaking References to Sub-Objects

When you null out a property that pointed to a sub-object, the reference is
broken but the sub-object still exists in the graph:

```javascript
gun.get('company').put({
  name: 'Acme',
  address: {
    city: 'Springfield'
  }
});

// This breaks the reference from company -> address
gun.get('company').put({ address: null });

// The address node still exists in the graph if you know its soul,
// but it's no longer discoverable through the company node.
// Think of it like Google removing a search result:
// the page still exists, but nobody can find it.
```

### The Tombstone Pattern

For application-level "soft delete," add a metadata flag rather than nullifying:

```javascript
var gun = Gun();

// "Soft delete" -- mark as deleted but preserve data
gun.get('post/123').put({
  deleted: true,
  deletedAt: Date.now()
});

// When reading, filter out deleted items
gun.get('posts').map(function (post) {
  if (post && !post.deleted) {
    return post;
  }
}).once(function (post) {
  console.log('Active post:', post.title);
});
```

### Why True Deletion Is Hard in Distributed Systems

Consider 9 peers where 5 are online and 4 are offline:

1. The 5 online peers all null out a piece of data and sync the nulls.
2. If those peers then truly purge the null tombstones from storage...
3. When the 4 offline peers come back online, they still have the original data.
4. They sync it to the 5 peers, and the data reappears.

The only reliable solution is to keep the null tombstone so it can "win" against
the old data via conflict resolution. This is why GUN keeps nulls by default.

### Storage Cleanup Modules

For applications where storage is a concern, GUN provides several adapter modules:

- **`lib/memdisk`** -- In-memory storage with periodic disk flush
- **`lib/erase`** -- Adapter for purging old data
- **`lib/forget`** -- Adapter for discarding unused data

These are storage-level solutions that trade consistency guarantees for disk space.

### Mutable vs. Immutable Cleanup

GUN supports both mutable and immutable data. For mutable data, when a value
changes from `"long string"` to `"x"`, the old bytes are not retained (only the
current state is stored). The storage concern is mainly around sub-objects
that are unlinked via null: the sub-object nodes persist in the graph even though
nothing points to them.

---

## Timestamps and Time-Series Data

GUN does not have built-in date types or time-series indexing. Instead, use
**lexicographic keys** to encode timestamps so that string sorting produces
chronological order.

### Lexicographic Timestamp Keys

```javascript
var gun = Gun();

function timeKey() {
  // ISO 8601 sorts lexicographically
  return new Date().toISOString();
  // e.g., "2024-01-15T14:30:00.000Z"
}

// Store time-series data
var readings = gun.get('sensor/temperature');

readings.get(timeKey()).put({
  value: 72.5,
  unit: 'F'
});

// Later...
readings.get(timeKey()).put({
  value: 73.1,
  unit: 'F'
});
```

### Custom Time Formats

For more compact keys, use a custom format:

```javascript
function compactTimeKey() {
  var d = new Date();
  return d.getFullYear() + '/' +
    String(d.getMonth() + 1).padStart(2, '0') + '/' +
    String(d.getDate()).padStart(2, '0') + ':' +
    String(d.getHours()).padStart(2, '0') + ':' +
    String(d.getMinutes()).padStart(2, '0') + ':' +
    String(d.getSeconds()).padStart(2, '0');
  // e.g., "2024/01/15:14:30:00"
}
```

### Querying Time Ranges with LEX

LEX range queries are ideal for time-series data:

```javascript
var gun = Gun();
var events = gun.get('events');

// Query all events in January 2024
events.get({
  '>': '2024/01/',
  '<': '2024/02/'
}).map().once(function (val, key) {
  console.log('Event at', key, ':', val);
});

// Query events in the last hour (approximate)
var now = new Date();
var oneHourAgo = new Date(now - 3600000);
events.get({
  '>': oneHourAgo.toISOString(),
  '<': now.toISOString()
}).map().once(function (val, key) {
  console.log('Recent event:', key, val);
});

// Query with a limit (most recent 50 events)
events.get({
  '*': '2024/',
  '%': 50,
  '-': 1  // reverse order (newest first)
}).map().once(function (val, key) {
  console.log(key, val);
});
```

### Timegraph Pattern for Ordered Feeds

For social feeds, chat messages, or activity logs, combine time-based keys with
a dedicated feed node:

```javascript
var gun = Gun();

function postToFeed(feedId, content, author) {
  var timestamp = new Date().toISOString();
  var postId = timestamp + '/' + author;

  var post = gun.get('post/' + postId).put({
    content: content,
    author: author,
    createdAt: timestamp
  });

  // Add to the feed with a time-sorted key
  gun.get('feed/' + feedId).get(postId).put(post);

  return post;
}

// Post some messages
postToFeed('general', 'Hello everyone!', 'alice');
postToFeed('general', 'Welcome, Alice!', 'bob');

// Read the feed in chronological order (keys sort by time)
gun.get('feed/general').map().once(function (post) {
  console.log(post.author + ': ' + post.content);
});

// Read only recent posts
gun.get('feed/general').get({
  '>': '2024-01-15T00:00:00.000Z',
  '<': '2024-01-16T00:00:00.000Z'
}).map().once(function (post, key) {
  console.log(key, post.content);
});
```

### Bucketing for Scale

For high-volume time-series data, bucket by day or hour to keep individual
nodes from growing too large:

```javascript
var gun = Gun();

function logEvent(event) {
  var now = new Date();
  var bucket = now.toISOString().slice(0, 10); // "2024-01-15"
  var key = now.toISOString();                 // full timestamp as key

  gun.get('logs/' + bucket).get(key).put(event);
}

// Query a specific day
function getEventsForDay(dateStr, cb) {
  gun.get('logs/' + dateStr).map().once(cb);
}

logEvent({ level: 'info', message: 'Server started' });
logEvent({ level: 'error', message: 'Connection timeout' });

getEventsForDay('2024-01-15', function (event, key) {
  console.log('[' + event.level + '] ' + event.message);
});
```

---

## Immutable / Content-Addressed Data

GUN supports content-addressed storage where the soul of a node is derived from
its content hash. This guarantees data integrity: if the content changes, the
hash changes, so the soul changes -- making the original immutable.

### The `#` Namespace Pattern

By convention, content-addressed data is stored under the `#` namespace:

```javascript
var gun = Gun();

// Compute a hash of the content (using SEA or any hash function)
async function storeImmutable(data) {
  var json = JSON.stringify(data);
  var hash = await SEA.work(json, null, null, { name: 'SHA-256' });

  // Store under the content hash
  gun.get('#').get(hash).put(data);

  return hash;
}

async function readImmutable(hash) {
  return new Promise(function (resolve) {
    gun.get('#').get(hash).once(function (data) {
      resolve(data);
    });
  });
}

// Usage
var hash = await storeImmutable({
  title: 'Immutable Blog Post',
  body: 'This content can never be altered.',
  timestamp: '2024-01-15T12:00:00Z'
});

console.log('Stored at hash:', hash);

var post = await readImmutable(hash);
console.log(post.title); // "Immutable Blog Post"
```

### Verification

Because the soul is the content hash, any peer can verify data integrity:

```javascript
async function verifyImmutable(hash, data) {
  var json = JSON.stringify(data);
  var computed = await SEA.work(json, null, null, { name: 'SHA-256' });
  return computed === hash;
}
```

### Use Cases

- **Content deduplication**: Identical content produces the same hash, so it is
  stored only once.
- **Audit trails**: Immutable records that provably have not been tampered with.
- **Caching**: Content-addressed data is eternally cacheable since the content
  at a given hash never changes.
- **Merkle structures**: Build Merkle trees/DAGs on top of GUN for verifiable
  data structures.

---

## Common Schema Patterns

### User Profiles

```javascript
var gun = Gun();
var user = gun.user(); // requires SEA

// After authentication:
user.get('profile').put({
  name: 'Alice Smith',
  bio: 'Software developer',
  avatar: 'https://example.com/alice.jpg',
  createdAt: new Date().toISOString()
});

// Public profile (readable by anyone)
user.get('profile').get('name').once(function (name) {
  console.log(name);
});
```

### Chat Messages

```javascript
var gun = Gun();

function sendMessage(chatId, author, text) {
  var timestamp = new Date().toISOString();
  var msgId = timestamp + '/' + Math.random().toString(36).slice(2, 8);

  gun.get('chat/' + chatId).get(msgId).put({
    author: author,
    text: text,
    timestamp: timestamp
  });
}

// Send messages
sendMessage('room/general', 'alice', 'Hey everyone!');
sendMessage('room/general', 'bob', 'Hi Alice!');

// Subscribe to new messages (realtime)
gun.get('chat/room/general').map().on(function (msg, key) {
  if (!msg) { return; }
  console.log('[' + msg.author + ']: ' + msg.text);
});

// Load message history for a time range
gun.get('chat/room/general').get({
  '>': '2024-01-15T00:00:00.000Z',
  '<': '2024-01-16T00:00:00.000Z'
}).map().once(function (msg, key) {
  console.log(msg.author + ': ' + msg.text);
});
```

### Social Feed

```javascript
var gun = Gun();

// Post to a user's feed
function createPost(userId, content) {
  var timestamp = new Date().toISOString();
  var postId = timestamp + '/' + userId;

  var post = gun.get('post/' + postId).put({
    author: userId,
    content: content,
    createdAt: timestamp,
    likeCount: 0
  });

  // Add to the user's personal feed
  gun.get('user/' + userId + '/posts').get(postId).put(post);

  // Add to the global feed
  gun.get('feed/global').get(postId).put(post);

  return post;
}

// Follow a user: copy their posts into your timeline
function followUser(myId, theirId) {
  gun.get('user/' + myId + '/following').set(
    gun.get('user/' + theirId)
  );

  // Subscribe to their future posts
  gun.get('user/' + theirId + '/posts').map().on(function (post, key) {
    if (!post) { return; }
    // Add to my timeline
    gun.get('user/' + myId + '/timeline').get(key).put(post);
  });
}

// Read your timeline
gun.get('user/alice/timeline').map().once(function (post) {
  if (!post) { return; }
  console.log(post.author + ': ' + post.content);
});
```

### Todo List

```javascript
var gun = Gun();

var todos = gun.get('todos/mylist');

function addTodo(title) {
  var id = new Date().toISOString() + '/' + Math.random().toString(36).slice(2, 6);
  todos.get(id).put({
    title: title,
    completed: false,
    createdAt: new Date().toISOString()
  });
  return id;
}

function toggleTodo(id) {
  todos.get(id).once(function (todo) {
    if (todo) {
      todos.get(id).put({ completed: !todo.completed });
    }
  });
}

function removeTodo(id) {
  // "Remove" by nullifying -- the key remains with a null value
  todos.get(id).put(null);
}

// Subscribe to all active todos
todos.map(function (todo) {
  if (todo && !todo.completed && todo.title) {
    return todo;
  }
}).on(function (todo, key) {
  console.log('[' + key + '] ' + todo.title);
});

// Usage
var id1 = addTodo('Buy groceries');
var id2 = addTodo('Write documentation');
toggleTodo(id1);
removeTodo(id2);
```

### Game State

```javascript
var gun = Gun();

// Shared game state -- all players see the same data in realtime
var game = gun.get('game/room123');

// Initialize game
game.put({
  status: 'waiting',
  maxPlayers: 4,
  round: 0
});

// Player joins
function joinGame(gameId, playerId, name) {
  var player = gun.get('player/' + playerId).put({
    name: name,
    score: 0,
    ready: false,
    lastAction: null
  });

  gun.get('game/' + gameId).get('players').set(player);
  return player;
}

// Update player state (each player only updates their own node)
function playerAction(playerId, action) {
  gun.get('player/' + playerId).put({
    lastAction: action,
    lastActionAt: new Date().toISOString()
  });
}

// Update score using the distributed counter pattern
function incrementScore(gameId, playerId, points) {
  var scoreEntry = new Date().toISOString();
  gun.get('game/' + gameId + '/scores/' + playerId)
    .get(scoreEntry)
    .put(points);
}

function getPlayerScore(gameId, playerId, cb) {
  var total = 0;
  gun.get('game/' + gameId + '/scores/' + playerId)
    .map().once(function (points) {
      if (typeof points === 'number') {
        total += points;
        cb(total);
      }
    });
}

// Subscribe to game state changes
game.on(function (state) {
  console.log('Game status:', state.status, 'Round:', state.round);
});

// Subscribe to all players
game.get('players').map().on(function (player) {
  if (!player) { return; }
  console.log(player.name, 'ready:', player.ready, 'action:', player.lastAction);
});

// Start the game when all players are ready
function checkAllReady(gameId) {
  var allReady = true;
  gun.get('game/' + gameId).get('players').map().once(function (player) {
    if (player && !player.ready) {
      allReady = false;
    }
  });
  // Note: due to async nature, you'd use a timeout/debounce in practice
  return allReady;
}
```

### Collaborative Document (Simple)

```javascript
var gun = Gun();

// Simple collaborative key-value document
var doc = gun.get('doc/meeting-notes');

doc.put({
  title: 'Sprint Planning - Week 3',
  updatedBy: null,
  updatedAt: null
});

// Each section is its own node to minimize conflicts
doc.get('section/agenda').put({
  content: '1. Review last sprint\n2. Plan next sprint',
  editedBy: 'alice',
  editedAt: new Date().toISOString()
});

doc.get('section/notes').put({
  content: '',
  editedBy: null,
  editedAt: null
});

// Subscribe to changes on a section
doc.get('section/agenda').on(function (section) {
  if (!section) { return; }
  console.log('Agenda updated by', section.editedBy + ':');
  console.log(section.content);
});

// Edit a section
function editSection(docId, sectionId, content, userId) {
  gun.get('doc/' + docId).get('section/' + sectionId).put({
    content: content,
    editedBy: userId,
    editedAt: new Date().toISOString()
  });
}
```

---

## Summary of Key Principles

1. **Partials by default** -- `.put()` merges, never replaces.
2. **Graphs, not trees** -- Circular references work naturally.
3. **Sets for collections** -- `.set()` adds, `.map()` iterates.
4. **No atomic increment** -- Use distributed counter patterns.
5. **No true delete** -- Nullify (tombstone) instead.
6. **LEX for queries** -- Design keys for lexicographic access.
7. **Time as keys** -- ISO timestamps enable time-range queries.
8. **Content addressing** -- Hash-based souls for immutability.
9. **One node per concern** -- Split data to minimize merge conflicts.
10. **Bidirectional links are explicit** -- GUN graphs are directed by default.
