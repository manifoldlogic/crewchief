# GUN User API

The User API provides authentication, identity management, and user-scoped data storage built on top of GUN's Security, Encryption, and Authorization (SEA) module. Every user gets a cryptographic key pair, and their data lives in a graph namespaced under their public key.

## Table of Contents

- [Getting Started](#getting-started)
- [gun.user(pub?)](#gunuserpub)
- [user.create(alias, password, callback)](#usercreatealias-password-callback)
- [user.auth(alias, password, callback?, options?)](#userauthalias-password-callback-options)
- [user.is](#useris)
- [user.leave()](#userleave)
- [user.recall(options, callback?)](#userrecalloptions-callback)
- [user.delete(alias, password, callback)](#userdeletealias-password-callback)
- [user.pair()](#userpair)
- [Finding Users](#finding-users)
- [Authentication Events](#authentication-events)
- [User Graph and Data Storage](#user-graph-and-data-storage)
- [Password Changes](#password-changes)
- [Certificates (SEA.certify)](#certificates-seacertify)
- [Deprecated and Experimental Methods](#deprecated-and-experimental-methods)
- [TypeScript Types](#typescript-types)
- [Common Patterns](#common-patterns)

---

## Getting Started

The User API requires the SEA module. In Node.js, require it explicitly. In the browser, include `gun/sea.js` after `gun.js`.

```js
// Node.js
var Gun = require('gun');
require('gun/sea');

var gun = Gun();
var user = gun.user();
```

```html
<!-- Browser -->
<script src="https://cdn.jsdelivr.net/npm/gun/gun.js"></script>
<script src="https://cdn.jsdelivr.net/npm/gun/sea.js"></script>
<script>
  var gun = Gun();
  var user = gun.user();
</script>
```

Only one user can be logged in at a time per GUN instance. The `user` reference is a chain that extends `IGunChain` with authentication methods.

---

## gun.user(pub?)

Initialize or retrieve the user chain. This is the entry point for all user operations.

### Without arguments -- get the current user chain

```js
var user = gun.user();
```

Returns the user chain for the current GUN instance. If a user chain already exists on this instance, it returns the existing one. Otherwise, it creates a new `User` instance and attaches it to the root.

### With a public key -- look up another user

```js
var alice = gun.user(alicePubKey);
```

Returns a chain reference to `~<publicKey>`, allowing you to read another user's public data. This is equivalent to `gun.get('~' + pub)`.

```js
// Read another user's profile
gun.user(alicePubKey).get('profile').once(function (data) {
  console.log('Alice profile:', data);
});
```

---

## user.create(alias, password, callback)

Create a new user account with a username (alias) and password.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `alias` | `string` | Yes | Username or alias for the account |
| `password` | `string` | Yes | Password (must be at least 8 characters). Extended internally with PBKDF2 for key derivation |
| `callback` | `function` | No | Invoked when creation completes or fails |

You can also pass an existing key pair as the first argument instead of alias/password, to register a pre-generated identity.

### Callback

The callback receives an acknowledgment object:

**Success:**
```js
{ ok: 0, pub: 'publickey...' }
```
The `ok: 0` value indicates the user was saved to the graph but disk acknowledgment was not waited for.

**Error -- user already being created or authenticated:**
```js
{ err: 'User is already being created or authenticated!', wait: true }
```

**Error -- alias already taken:**
```js
{ err: 'User already created!' }
```

**Error -- missing alias:**
```js
{ err: 'No user.' }
```

**Error -- password too short:**
```js
{ err: 'Password too short!' }
```

### How It Works

1. Checks if the alias already exists by looking up `~@alias` in the graph.
2. Generates a random 64-character salt.
3. Derives a proof-of-work from the password and salt using PBKDF2 (via `SEA.work`).
4. Generates a new ECDSA/ECDH key pair (via `SEA.pair`), or uses a provided pair.
5. Encrypts the private keys (`priv`, `epriv`) with the proof-of-work using AES.
6. Stores the user record at `~<publicKey>` with fields: `pub`, `alias`, `epub`, `auth`.
7. Creates an alias-to-public-key link at `~@<alias>`.

### Alias Uniqueness

GUN is a decentralized database with no central authority. The alias uniqueness check (`~@alias` lookup) is a best-effort mechanism -- it relies on the data your peers have synced. In practice, two users could create the same alias simultaneously on disconnected peers. If strict uniqueness matters, validate manually:

```js
gun.get('~@myAlias').once(function (data, key) {
  if (data) {
    console.log('Alias already taken');
  } else {
    console.log('Alias available');
  }
});
```

### Auto-Login Behavior

If no callback is passed to `create`, the user is automatically authenticated after creation:

```js
// No callback -- auto-login happens
user.create('alice', 'password123');

// With callback -- no auto-login, you must call auth() yourself
user.create('alice', 'password123', function (ack) {
  if (ack.err) {
    console.error(ack.err);
    return;
  }
  console.log('Created user with pub:', ack.pub);
  // Now authenticate
  user.auth('alice', 'password123');
});
```

### Examples

**Basic account creation:**

```js
var user = gun.user();

user.create('alice', 'securePassword1', function (ack) {
  if (ack.err) {
    console.error('Creation failed:', ack.err);
    return;
  }
  console.log('User created! Public key:', ack.pub);
});
```

**Create with an existing key pair:**

```js
var SEA = Gun.SEA;

SEA.pair(function (pair) {
  console.log('Generated pair:', pair.pub);

  user.create(pair, function (ack) {
    if (ack.err) {
      console.error(ack.err);
      return;
    }
    console.log('Registered with pub:', ack.pub);
  });
});
```

**Guard against duplicate aliases:**

```js
var alias = 'bob';

gun.get('~@' + alias).once(function (data) {
  if (data) {
    console.log('Username taken, pick another');
    return;
  }
  user.create(alias, 'strongPassword1', function (ack) {
    if (ack.err) {
      console.error(ack.err);
    } else {
      console.log('Account ready:', ack.pub);
    }
  });
});
```

---

## user.auth(alias, password, callback?, options?)

Authenticate an existing user by alias and password, or by key pair.

### Signature Variants

```js
// By alias and password
user.auth(alias, password, callback?, options?)

// By key pair (ISEAPair)
user.auth(pair, callback?, options?)
```

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `alias` | `string` | Yes (variant 1) | Username to authenticate |
| `password` | `string` | Yes (variant 1) | Password for the account |
| `pair` | `ISEAPair` | Yes (variant 2) | Key pair object with `pub`, `epub`, `priv`, `epriv` |
| `callback` | `function` | No | Receives the authenticated user reference or an error |
| `options` | `object` | No | `{ change: 'newPassword' }` to change password on auth |

### Callback

**Success** -- the callback receives the user's internal state object:

```js
{
  ack: 2,
  soul: '~publicKeyOfUser',
  get: '~publicKeyOfUser',
  put: { alias: 'alice', epub: '...', pub: '...' },
  sea: { pub: '...', epub: '...', priv: '...', epriv: '...' }
}
```

The `sea` property contains the full key pair. The `put` property contains the user's public data.

**Error:**
```js
{ err: 'Wrong user or password.' }
```

**Error -- concurrent auth attempt:**
```js
{ err: 'User is already being created or authenticated!', wait: true }
```

### How It Works (Alias/Password)

1. Looks up `~@<alias>` to find the user's public key.
2. Retrieves the user's `auth` field (contains encrypted private keys and salt).
3. Derives proof-of-work from the password and stored salt using PBKDF2.
4. Decrypts the private keys using the proof.
5. If decryption succeeds, sets the user's `is` and `sea` properties.
6. Emits the `auth` event on the GUN root.

### How It Works (Key Pair)

When a full key pair (with `priv` and `epriv`) is provided, the keys are used directly -- no password derivation is needed. When only `pub` and `epub` are provided, GUN looks up the user's data and still requires password derivation.

### Retry Behavior

Authentication retries up to 9 times by default when looking up the user's public key. This handles eventual consistency in peer-to-peer sync. Control this with `options.retries`:

```js
user.auth('alice', 'password', callback, { retries: 3 });
```

### Examples

**Authenticate by alias and password:**

```js
var user = gun.user();

user.auth('alice', 'securePassword1', function (ack) {
  if (ack.err) {
    console.error('Login failed:', ack.err);
    return;
  }
  console.log('Logged in as:', user.is.alias);
  console.log('Public key:', user.is.pub);
});
```

**Authenticate by key pair:**

```js
// If you saved the pair from a previous session
var savedPair = {
  pub: 'xxxxxxxx.yyyyyyyy',
  epub: 'xxxxxxxx.yyyyyyyy',
  priv: 'base64PrivateKey',
  epriv: 'base64EncPrivateKey'
};

user.auth(savedPair, function (ack) {
  if (ack.err) {
    console.error('Key pair auth failed:', ack.err);
    return;
  }
  console.log('Authenticated via key pair');
});
```

**Authenticate and immediately read/write:**

```js
user.auth('alice', 'securePassword1', function (ack) {
  if (ack.err) return;

  // Write to user's graph
  user.get('profile').put({
    name: 'Alice',
    bio: 'Decentralized developer'
  });

  // Read it back
  user.get('profile').once(function (data) {
    console.log(data.name); // "Alice"
  });
});
```

---

## user.is

A property (not a method) that indicates the current authentication status.

### When Authenticated

Returns an object with the user's public identity:

```js
{
  alias: 'alice',        // string username, or ISEAPair if auth'd by pair
  epub: 'xxxxxxxx.yyyy', // public encryption key (ECDH P-256)
  pub: 'xxxxxxxx.yyyy'   // public signing key (ECDSA P-256)
}
```

### When Not Authenticated

Returns `undefined`.

### Examples

**Check login status:**

```js
if (user.is) {
  console.log('Logged in as:', user.is.alias);
  console.log('Public key:', user.is.pub);
  console.log('Encryption pub:', user.is.epub);
} else {
  console.log('Not logged in');
}
```

**Guard a write operation:**

```js
function saveProfile(name, bio) {
  if (!user.is) {
    console.error('Must be logged in to save profile');
    return;
  }
  user.get('profile').put({ name: name, bio: bio });
}
```

**Display login state in UI:**

```js
function updateUI() {
  var status = document.getElementById('status');
  if (user.is) {
    status.textContent = 'Welcome, ' + user.is.alias;
  } else {
    status.textContent = 'Please log in';
  }
}
```

---

## user.leave()

Log out the currently authenticated user. Clears the in-memory credentials and session storage.

### Returns

The GUN root chain reference (not the user chain), allowing you to continue chaining.

### What It Does

1. Deletes `user.is` and `user._.is` (public identity).
2. Deletes `user._.sea` (key pair from memory).
3. Clears `sessionStorage.recall` and `sessionStorage.pair` (browser persistence).

### No Callback

There is no callback to confirm logout. To verify the user was logged out, check the internal state.

### Examples

**Basic logout:**

```js
user.leave();

console.log(user.is); // undefined
```

**Logout with verification:**

```js
user.leave();

if (!user.is) {
  console.log('Successfully logged out');
} else {
  console.log('Logout may not have completed');
}
```

**Logout and redirect (browser):**

```js
function logout() {
  user.leave();
  // Verify credentials are wiped
  var sea = (user._ || {}).sea;
  if (!sea) {
    console.log('Credentials cleared from memory');
  }
  window.location.href = '/login';
}
```

---

## user.recall(options, callback?)

Persist authentication across page refreshes using the browser's `sessionStorage`. The session lasts until the tab is closed.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `options` | `object` | Yes | `{ sessionStorage: true }` to enable session persistence |
| `callback` | `function` | No | Passed internally to `user.auth()` -- receives the same ack as auth |

### How It Works

1. Sets `remember: true` on the GUN instance options.
2. Checks `sessionStorage` for a stored key pair (`sessionStorage.pair`).
3. If found, calls `user.auth(pair, callback)` with the stored pair.
4. On subsequent successful authentications, the key pair is automatically saved to `sessionStorage`.

### Browser-Only

This method relies on `window.sessionStorage` and has no effect in Node.js or environments without a `window` object.

### Examples

**Enable session persistence on app startup:**

```js
var gun = Gun('https://relay.example.com/gun');
var user = gun.user().recall({ sessionStorage: true });

// On page load, if the user was previously logged in during this tab session,
// they will be automatically re-authenticated.
```

**With a callback to know when session is restored:**

```js
var user = gun.user().recall({ sessionStorage: true }, function (ack) {
  if (ack.err) {
    console.log('No active session or session expired');
    showLoginForm();
    return;
  }
  console.log('Session restored for:', user.is.alias);
  showDashboard();
});
```

**Typical app initialization pattern:**

```js
var gun = Gun(['https://relay1.example.com/gun', 'https://relay2.example.com/gun']);
var user = gun.user().recall({ sessionStorage: true });

// Listen for auth to know when the user is ready
gun.on('auth', function (ack) {
  console.log('User authenticated:', user.is.alias);
  initApp();
});

function login(alias, pass) {
  user.auth(alias, pass, function (ack) {
    if (ack.err) {
      alert(ack.err);
    }
    // The 'auth' event handler above will fire on success
  });
}
```

---

## user.delete(alias, password, callback)

Delete a user account. This method re-authenticates the user, sets all their data to `null`, then logs out.

> **Deprecation Warning:** The source code logs `"user.delete() IS DEPRECATED AND WILL BE MOVED TO A MODULE!!!"`. Use with caution -- this API may move to a separate module in future versions.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `alias` | `string` | Yes | Username of the account to delete |
| `password` | `string` | Yes | Password for the account |
| `callback` | `function` | No | Called on completion |

### Callback

**Success:**
```js
{ ok: 0 }
```

**Errors** are logged to the console rather than passed to the callback.

### How It Works

1. Calls `user.auth(alias, pass, ...)` to authenticate.
2. Iterates over all user data with `user.map().once()` and sets each value to `null`.
3. Calls `user.leave()` to clear credentials from memory.
4. Invokes the callback with `{ ok: 0 }`.

### Important Caveats

- In a decentralized system, deletion propagates as `null` values. Peers that have already synced the data will see the nullification, but there is no guarantee that all copies are removed from every peer.
- The alias-to-public-key mapping (`~@alias`) is not explicitly cleaned up.
- Errors during deletion are caught and logged with `Gun.log`, not passed to the callback.

### Examples

```js
user.delete('alice', 'securePassword1', function (ack) {
  if (ack.ok === 0) {
    console.log('Account deleted');
  }
});
```

---

## user.pair()

Access the current user's key pair through a safety proxy.

### Returns

A `Proxy` object that allows reading the key pair properties (`pub`, `epub`, `priv`, `epriv`) only if the user is authenticated. If not authenticated, all property accesses return `undefined`.

The proxy has a `DANGER` property set to a skull-and-crossbones character as a reminder that leaking private keys compromises the account.

### Examples

```js
user.auth('alice', 'securePassword1', function (ack) {
  if (ack.err) return;

  var pair = user.pair();
  console.log(pair.pub);   // public signing key
  console.log(pair.epub);  // public encryption key
  console.log(pair.priv);  // PRIVATE signing key -- do not share!
  console.log(pair.epriv); // PRIVATE encryption key -- do not share!
});

// Before auth, pair properties are undefined
var pair = user.pair();
console.log(pair.pub); // undefined
```

---

## Finding Users

GUN uses two namespacing conventions for users:
- `~<publicKey>` -- a user's data node, keyed by public key
- `~@<alias>` -- an alias lookup node that maps to one or more public keys

### Find by Alias

```js
gun.get('~@alice').once(function (data, key) {
  if (!data) {
    console.log('User not found');
    return;
  }
  // data contains references to public keys
  // Each key in data (except '_') is a ~pubKey reference
  Object.keys(data).forEach(function (k) {
    if (k === '_') return;
    console.log('Found public key reference:', k);
  });
});
```

### Find by Public Key

```js
var pubKey = 'xxxxxxxx.yyyyyyyy';

gun.user(pubKey).once(function (data) {
  console.log('User data:', data);
  console.log('Alias:', data.alias);
  console.log('Public key:', data.pub);
  console.log('Encryption key:', data.epub);
});
```

### Read Another User's Data

```js
// Read alice's profile (assuming you know her public key)
gun.user(alicePubKey).get('profile').once(function (data) {
  console.log('Name:', data.name);
});

// Subscribe to real-time updates on alice's status
gun.user(alicePubKey).get('status').on(function (data) {
  console.log('Alice status changed:', data);
});
```

---

## Authentication Events

GUN emits an `auth` event on the root instance whenever a user successfully authenticates. This fires after `user.auth()` completes and after session recall.

```js
gun.on('auth', function (ack) {
  console.log('User authenticated!');
  console.log('Alias:', user.is.alias);
  console.log('Public key:', user.is.pub);

  // Safe to start reading/writing user data here
  user.get('lastLogin').put(Gun.state());
});
```

### Timing Note

If no `auth` event listener is registered at the time authentication completes, GUN uses a 1ms `setTimeout` to re-emit the event. This means the listener will still fire even if registered slightly after `user.auth()` is called, but you should register it early to avoid race conditions.

```js
// Register listener BEFORE calling auth
gun.on('auth', function (ack) {
  console.log('Ready');
});

user.auth('alice', 'securePassword1');
```

---

## User Graph and Data Storage

Each authenticated user has a graph namespace under `~<publicKey>`. All data written through the `user` chain is automatically placed in this namespace and signed with the user's private key.

### Writing Data

```js
user.auth('alice', 'securePassword1', function (ack) {
  if (ack.err) return;

  // These all write to ~alicePubKey/profile, ~alicePubKey/settings, etc.
  user.get('profile').put({
    name: 'Alice',
    bio: 'Building the decentralized web'
  });

  user.get('settings').put({
    theme: 'dark',
    notifications: true
  });

  // Nested data
  user.get('posts').get('first-post').put({
    title: 'Hello World',
    content: 'My first decentralized post',
    timestamp: Gun.state()
  });
});
```

### Reading Your Own Data

```js
user.get('profile').once(function (data) {
  console.log(data.name); // "Alice"
});

// Subscribe to changes
user.get('settings').on(function (data) {
  applyTheme(data.theme);
});
```

### Reading Another User's Data

```js
// Anyone can read public user data if they know the public key
gun.user(alicePubKey).get('profile').once(function (data) {
  console.log(data.name); // "Alice"
});

// Subscribe to another user's posts
gun.user(alicePubKey).get('posts').map().once(function (post, id) {
  console.log(id, post.title);
});
```

### Data Signing (Automatic)

When you write data through an authenticated `user` chain, SEA automatically signs every value with the user's private key. When other peers receive this data, they verify the signature against the user's public key before accepting it. This prevents tampering -- only the key holder can write to their own graph.

### UUID Generation

When authenticated, any new nodes created under the user chain get UUIDs prefixed with `~<pub>/`, ensuring they are namespaced to the user:

```js
// After auth, set() creates IDs like: ~pubKey/lk3j2f9
user.get('todos').set({ text: 'Buy milk', done: false });
```

---

## Password Changes

Change a user's password by passing `{ change: 'newPassword' }` in the options parameter of `user.auth()`. You must authenticate with the current password first.

### How It Works

1. Authenticates with the current alias/password.
2. Generates a new random salt.
3. Derives a new proof-of-work from the new password and new salt.
4. Re-encrypts the private keys with the new proof.
5. Updates the `auth` field on the user's node.

### Example

```js
user.auth('alice', 'oldPassword1', function (ack) {
  if (ack.err) {
    console.error('Current password incorrect:', ack.err);
    return;
  }
  console.log('Password changed successfully');
}, { change: 'newSecurePassword2' });
```

After a password change, the old password will no longer work for authentication. The key pair itself does not change -- only the encryption of the stored private keys is updated.

---

## Certificates (SEA.certify)

Certificates allow a user to grant write access to specific parts of their graph to other users, without sharing private keys. This is the mechanism for collaborative data in GUN.

> **Experimental:** The source code warns that `SEA.certify()` is an early experimental community-supported method. The API may change without warning.

### Creating a Certificate

```js
var SEA = Gun.SEA;

// Alice grants Bob write access to her "inbox" path
var certificate = await SEA.certify(
  bobPubKey,                    // who gets access
  { '*': 'inbox' },            // where they can write (LEX path pattern)
  aliceKeyPair,                 // authority (Alice's pair or priv)
  null,                         // callback (optional)
  { expiry: Gun.state() + (60 * 60 * 1000) }  // expires in 1 hour
);
```

### Certificate Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `certificants` | `'*'` or `string` or `string[]` or `{pub}` or `{pub}[]` | Who gets write access. `'*'` means everyone |
| `policy` | `string` or `LEX` or `Array` | Path/key rules defining where they can write |
| `authority` | `ISEAPair` or `{priv, pub}` | The certificate issuer's key pair |
| `callback` | `function` | Called with the signed certificate string |
| `options` | `object` | `{ expiry: timestamp }` for time-limited certificates. `{ block: string }` for revocation |

### Using a Certificate to Write to Another User's Graph

```js
// Bob writes to Alice's graph using the certificate
gun.user(alicePubKey).get('inbox').get('msg1').put(
  { from: 'Bob', text: 'Hello Alice!' },
  null,
  { opt: { cert: certificate } }
);
```

### Certificate with Block List (Revocation)

```js
// Alice creates a cert with a block list path
var cert = await SEA.certify(
  '*',                         // everyone
  { '*': 'comments' },        // can write to comments
  aliceKeyPair,
  null,
  {
    expiry: Gun.state() + (24 * 60 * 60 * 1000),
    block: 'blocked-users'    // path in Alice's graph for blocked users
  }
);

// Later, Alice blocks a spammer
user.get('blocked-users').get(spammerPubKey).put(true);
```

---

## Deprecated and Experimental Methods

These methods exist in the source code but carry explicit deprecation or instability warnings.

### user.trust(user) -- BROKEN

```js
// DO NOT USE -- source code warns:
// "`.trust` API MAY BE DELETED OR CHANGED OR RENAMED, DO NOT USE!"
```

Was intended for granting read/write access to another user's data. The implementation is incomplete and references undefined variables. It has been effectively broken since at least 2020.

### user.grant(to, callback) -- EXPERIMENTAL

```js
// DO NOT USE -- source code warns:
// "`.grant` API MAY BE DELETED OR CHANGED OR RENAMED, DO NOT USE!"
```

Intended for granting encrypted read access to specific paths of your data. It derives a shared secret using `SEA.secret()`, encrypts a content key with it, and stores the encrypted key at `grant/<theirPub>/<path>`. The API may change or be removed.

### user.secret(data, callback) -- EXPERIMENTAL

```js
// DO NOT USE -- source code warns:
// "`.secret` API MAY BE DELETED OR CHANGED OR RENAMED, DO NOT USE!"
```

Intended for storing encrypted data that only trusted users can read. Uses `SEA.encrypt()` with a shared content key stored in the `trust` path. The API may change or be removed.

### user.alive() -- DEPRECATED

```js
// Source code logs: "user.alive() IS DEPRECATED!!!"
```

Was intended to check if a recalled session is still valid. Throws `{ err: 'No session!' }` if no session exists.

---

## TypeScript Types

The User API has full TypeScript definitions. Here are the key types.

### ISEAPair

```typescript
interface ISEAPair {
  /** private key for signing (ECDSA) */
  priv: string;
  /** public key for signing (ECDSA) */
  pub: string;
  /** private key for encryption (ECDH) */
  epriv: string;
  /** public key for encryption (ECDH) */
  epub: string;
}
```

### user.is Type

```typescript
is?: {
  alias: string | ISEAPair;
  /** public key for encryption */
  epub: string;
  /** public key */
  pub: string;
}
```

### GunCallbackUserCreate

```typescript
type GunCallbackUserCreate = (
  ack: { ok: 0; pub: string } | { err: string }
) => void;
```

### GunCallbackUserAuth

```typescript
type GunCallbackUserAuth = (
  ack:
    | {
        ack: 2;
        /** ~publicKeyOfUser */
        soul: string;
        /** ~publicKeyOfUser */
        get: string;
        put: GunUser;
        sea: ISEAPair;
      }
    | { err: string }
) => void;
```

### OptionsUserAuth

```typescript
type OptionsUserAuth = { change: string };
```

### OptionsUserRecall

```typescript
type OptionsUserRecall = { sessionStorage: boolean };
```

### IGunUserInstance (simplified)

```typescript
interface IGunUserInstance extends IGunInstanceRoot {
  create(alias: string, password: string, callback: GunCallbackUserCreate): IGunUserInstance;
  auth(pair: ISEAPair, callback?: GunCallbackUserAuth, options?: OptionsUserAuth): IGunUserInstance;
  auth(alias: string, password: string, callback?: GunCallbackUserAuth, options?: OptionsUserAuth): IGunUserInstance;
  is?: { alias: string | ISEAPair; epub: string; pub: string };
  leave(): IGunInstanceRoot;
  recall(options: OptionsUserRecall, callback?: GunCallbackUserAuth): IGunUserInstance;
}
```

---

## Common Patterns

### Full Registration and Login Flow

```js
var gun = Gun('https://relay.example.com/gun');
var user = gun.user().recall({ sessionStorage: true });

// Check if already logged in from session
gun.on('auth', function () {
  console.log('Authenticated as:', user.is.alias);
  showApp();
});

function register(alias, pass, cb) {
  // Check alias availability first
  gun.get('~@' + alias).once(function (data) {
    if (data) {
      cb({ err: 'Username already taken' });
      return;
    }
    user.create(alias, pass, function (ack) {
      if (ack.err) {
        cb(ack);
        return;
      }
      // Created -- now log in
      user.auth(alias, pass, function (authAck) {
        cb(authAck);
      });
    });
  });
}

function login(alias, pass, cb) {
  user.auth(alias, pass, function (ack) {
    cb(ack);
  });
}

function logout() {
  user.leave();
  showLogin();
}
```

### Encrypted Private Messaging

```js
var SEA = Gun.SEA;

// Alice sends an encrypted message to Bob
async function sendMessage(bobEpub, message) {
  // Derive shared secret from Bob's epub and Alice's key pair
  var secret = await SEA.secret(bobEpub, user.pair());

  // Encrypt the message with the shared secret
  var encrypted = await SEA.encrypt(message, secret);

  // Store in Alice's graph (Bob reads it from there)
  user.get('messages').get('to-bob').get(Gun.state().toString()).put(encrypted);
}

// Bob decrypts a message from Alice
async function readMessage(aliceEpub, encryptedMessage) {
  var secret = await SEA.secret(aliceEpub, user.pair());
  var decrypted = await SEA.decrypt(encryptedMessage, secret);
  return decrypted;
}
```

### Social Graph with Public Profiles

```js
// After authentication, set up a public profile
user.get('profile').put({
  name: 'Alice',
  bio: 'Decentralized developer',
  avatar: 'https://example.com/alice.jpg'
});

// Follow another user
function follow(pubKey) {
  user.get('following').get(pubKey).put(true);
}

// Get a user's followers (they each added your pubkey under their "following")
function listFollowing(cb) {
  user.get('following').map().once(function (val, pubKey) {
    if (!val) return; // unfollowed
    gun.user(pubKey).get('profile').once(function (profile) {
      cb(pubKey, profile);
    });
  });
}
```

### Signed and Verified Public Data

```js
var SEA = Gun.SEA;

// Publish a signed statement
async function publishStatement(text) {
  var signed = await SEA.sign(text, user.pair());
  user.get('statements').set({ text: signed, timestamp: Gun.state() });
}

// Anyone can verify the statement came from the user
async function verifyStatement(pubKey, signedText) {
  var data = await SEA.verify(signedText, pubKey);
  if (data) {
    console.log('Verified message:', data);
  } else {
    console.log('Signature verification failed');
  }
}
```

### Checking Authentication Before Operations

```js
function requireAuth(fn) {
  return function () {
    if (!user.is) {
      console.error('Authentication required');
      return;
    }
    return fn.apply(this, arguments);
  };
}

var savePost = requireAuth(function (title, content) {
  var post = {
    title: title,
    content: content,
    author: user.is.pub,
    timestamp: Gun.state()
  };
  user.get('posts').set(post);
});
```
