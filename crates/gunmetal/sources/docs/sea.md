# SEA -- Security, Encryption, and Authorization

SEA is the cryptographic layer of GUN. It provides user authentication, data
signing, encryption, and certificate-based authorization for the decentralized
graph database. SEA wraps the browser's WebCrypto API (and Node.js equivalents)
behind a small, consistent interface so that every read and write in the graph
can be cryptographically verified.

## Two Parts of SEA

| Layer | What it does |
|---|---|
| `gun.user()` chain | High-level user management -- create accounts, authenticate, read/write user-scoped data. SEA hooks automatically sign and verify every value written under a user's graph. |
| `Gun.SEA` utility | Low-level static functions for key generation, signing, verification, encryption, decryption, hashing, shared secrets, and certificates. Usable independently of the GUN database. |

## Default Cryptographic Primitives

| Purpose | Algorithm | Curve / Parameters |
|---|---|---|
| Signing / Verification | ECDSA | P-256, SHA-256 hash |
| Key Agreement (encryption) | ECDH | P-256 |
| Symmetric Encryption | AES-GCM | 256-bit derived key |
| Password Stretching | PBKDF2 | SHA-256, 100 000 iterations, 64-byte key |
| Hashing | SHA-256 | -- |

---

## Setup

### Browser

```html
<script src="https://cdn.jsdelivr.net/npm/gun/gun.js"></script>
<script src="https://cdn.jsdelivr.net/npm/gun/sea.js"></script>
<script>
  // SEA is now available as a global and on the Gun constructor
  var SEA = Gun.SEA;
</script>
```

SEA requires HTTPS in the browser. If loaded over plain HTTP (and the host is
not `localhost` or `127.x.x.x`), it will automatically redirect to `https:`.

### Node.js

```javascript
const Gun = require('gun');
require('gun/sea');

// -- or import SEA directly --
const SEA = require('gun/sea');
```

Node.js does not ship with the WebCrypto API that SEA relies on. Install the
polyfill:

```bash
npm install @peculiar/webcrypto --save
```

SEA's shim layer (`sea/shim.js`) detects the Node environment and loads
`@peculiar/webcrypto` automatically. If the package is missing, you will see a
console warning at startup.

---

## Error Handling

SEA is designed for production stability. By default it does **not** throw
exceptions. Instead, every function returns `undefined` on failure and stores
the last error on `SEA.err`.

```javascript
const signed = await SEA.sign(data, pair);
if (!signed) {
  console.error('Signing failed:', SEA.err);
}
```

During development you can opt into thrown errors:

```javascript
SEA.throw = true; // functions will throw on failure
```

> **Warning:** Do not enable `SEA.throw` in production. A thrown exception
> inside a real-time sync pipeline will crash your application.

---

## ISEAPair -- The Key Pair Object

Every identity in SEA is represented by a key pair with four fields:

```typescript
interface ISEAPair {
  pub: string;   // ECDSA public signing key  (base64url x.y)
  priv: string;  // ECDSA private signing key (base64url d)
  epub: string;  // ECDH public encryption key (base64url x.y)
  epriv: string; // ECDH private encryption key (base64url d)
}
```

`pub` and `epub` are safe to share. **Never expose `priv` or `epriv`.**

The public keys are encoded as the JWK `x` and `y` coordinates joined by a
period (`.`). This format is URL-safe and avoids base64 padding issues.

---

## SEA.pair([callback])

Generate a cryptographically secure key pair.

**Signature:**

```typescript
SEA.pair(callback?: (pair: ISEAPair) => void): Promise<ISEAPair>
```

**Returns:** `{ pub, priv, epub, epriv }`

**Implementation details:**

1. Generates an ECDSA P-256 key pair for signing and verification.
2. Generates an ECDH P-256 key pair for key agreement and encryption.
3. Exports both key pairs as JWK and assembles the `ISEAPair` object.

**Example -- async/await:**

```javascript
const pair = await SEA.pair();
console.log(pair.pub);   // signing public key
console.log(pair.epub);  // encryption public key
// pair.priv and pair.epriv are SECRET -- never log or transmit them.
```

**Example -- callback:**

```javascript
SEA.pair(function (pair) {
  console.log('Keys ready:', pair.pub);
});
```

---

## SEA.sign(data, pair [, callback, options])

Attach a cryptographic signature to data, proving that the holder of the
private key authorized it.

**Signature:**

```typescript
SEA.sign(
  data: any,
  pair: { priv: string; pub: string },
  callback?: (signed: string | undefined) => void,
  options?: { raw?: boolean; encode?: string; check?: any }
): Promise<string>
```

**Parameters:**

| Name | Description |
|---|---|
| `data` | Any JSON-serializable value. `undefined` is not allowed. |
| `pair` | An object containing at least `{ pub, priv }` from `SEA.pair()`. |

**Returns:** A `string` prefixed with `"SEA"` that encodes `{ m, s }` -- the
original message and its base64-encoded ECDSA signature over the SHA-256 hash
of the message.

**How it works:**

1. Serializes `data` to a JSON string.
2. Computes a SHA-256 digest of the serialized data.
3. Signs the digest with the ECDSA private key (`pair.priv`).
4. Returns `"SEA" + JSON.stringify({ m: <data>, s: <signature> })`.

If the data is already signed and verifiable with the same key, SEA returns the
existing signature without re-signing.

**Example:**

```javascript
const pair = await SEA.pair();
const message = { text: 'Hello, world!', ts: Date.now() };

const signed = await SEA.sign(message, pair);
console.log(signed);
// "SEA{"m":{"text":"Hello, world!","ts":1700000000000},"s":"base64..."}"
```

---

## SEA.verify(message, pair [, callback, options])

Verify a signed message and extract the original data.

**Signature:**

```typescript
SEA.verify<T = any>(
  message: string,
  pair: string | { pub: string },
  callback?: (data: T | undefined) => void,
  options?: { encode?: string }
): Promise<T | undefined>
```

**Parameters:**

| Name | Description |
|---|---|
| `message` | The signed string returned by `SEA.sign()`. |
| `pair` | The signer's public key as a string, or an object `{ pub }`. |

**Returns:** The original data if the signature is valid; `undefined` otherwise.

**Special case:** Passing `false` as `pair` skips verification entirely and
returns the raw message payload. Use this only when you have already verified
the data through another channel.

**Security note:** Only verify against public keys you already trust. If an
attacker supplies both the message and the public key, the verification is
meaningless.

**Example:**

```javascript
const pair = await SEA.pair();
const signed = await SEA.sign('important data', pair);

// Verify with the full pair object
const data = await SEA.verify(signed, pair);
console.log(data); // "important data"

// Verify with just the public key string
const data2 = await SEA.verify(signed, pair.pub);
console.log(data2); // "important data"

// Tampered or wrong key
const other = await SEA.pair();
const bad = await SEA.verify(signed, other);
console.log(bad); // undefined
```

**Fallback behavior:** SEA includes backward-compatible verification for older
data formats (UTF-8 encoded signatures and legacy SHA-256 hashing). The
`SEA.opt.fallback` setting (default `2`) controls how many legacy rounds to
attempt before giving up.

---

## SEA.encrypt(data, pair_or_passphrase [, callback, options])

Encrypt data for confidentiality using AES-GCM.

**Signature (key pair overload):**

```typescript
SEA.encrypt(
  data: any,
  pair: { epriv: string },
  callback?: (encrypted: string | undefined) => void,
  options?: { name?: string; encode?: string; raw?: boolean }
): Promise<string>
```

**Signature (passphrase overload):**

```typescript
SEA.encrypt(
  data: any,
  passphrase: string,
  callback?: (encrypted: string | undefined) => void,
  options?: { name?: string; encode?: string; raw?: boolean }
): Promise<string>
```

**Parameters:**

| Name | Description |
|---|---|
| `data` | Any JSON-serializable value. `undefined` is not allowed. |
| `pair` / `passphrase` | Either an object with `epriv` (the encryption private key from `SEA.pair()`) or a plain string passphrase. |

**Returns:** A `string` prefixed with `"SEA"` that encodes `{ ct, iv, s }` --
the ciphertext, initialization vector, and salt, all base64-encoded.

**How it works:**

1. Serializes `data` to a JSON string.
2. Generates a random 9-byte salt and 15-byte IV.
3. Derives an AES-256-GCM key from the passphrase/epriv + salt via SHA-256.
4. Encrypts the serialized data with AES-GCM using the derived key and IV.

**Example -- encrypt with key pair:**

```javascript
const pair = await SEA.pair();
const secret = { ssn: '123-45-6789' };

const encrypted = await SEA.encrypt(secret, pair);
console.log(encrypted);
// "SEA{"ct":"base64...","iv":"base64...","s":"base64..."}"
```

**Example -- encrypt with passphrase:**

```javascript
const encrypted = await SEA.encrypt('my secret', 'strong-passphrase');
console.log(encrypted);
```

**Example -- encrypt with a shared secret (for two-party communication):**

```javascript
const alice = await SEA.pair();
const bob = await SEA.pair();

// Alice derives the shared secret
const sharedKey = await SEA.secret(bob.epub, alice);

// Alice encrypts with the shared secret
const encrypted = await SEA.encrypt('Hello Bob', sharedKey);
```

---

## SEA.decrypt(message, pair_or_passphrase [, callback, options])

Decrypt data previously encrypted with `SEA.encrypt`.

**Signature (key pair overload):**

```typescript
SEA.decrypt<T = any>(
  message: string,
  pair: { epriv: string },
  callback?: (data: T | undefined) => void,
  options?: { name?: string; encode?: string }
): Promise<T>
```

**Signature (passphrase overload):**

```typescript
SEA.decrypt<T = any>(
  message: string,
  passphrase: string,
  callback?: (data: T | undefined) => void,
  options?: { name?: string; encode?: string }
): Promise<T>
```

**Parameters:**

| Name | Description |
|---|---|
| `message` | The encrypted string from `SEA.encrypt()`. |
| `pair` / `passphrase` | The same key or passphrase used to encrypt. |

**Returns:** The original decrypted data, or `undefined` on failure.

**Example:**

```javascript
const pair = await SEA.pair();

const encrypted = await SEA.encrypt({ answer: 42 }, pair);
const decrypted = await SEA.decrypt(encrypted, pair);
console.log(decrypted); // { answer: 42 }
```

**Example -- passphrase:**

```javascript
const encrypted = await SEA.encrypt('secret', 'my-password');
const decrypted = await SEA.decrypt(encrypted, 'my-password');
console.log(decrypted); // "secret"
```

---

## SEA.work(data [, pair, callback, options])

Proof of Work and hashing. Primarily used to derive a password hash that is
resistant to brute-force attacks.

**Signature:**

```typescript
SEA.work(
  data: any,
  pair?: ISEAPair | null,
  callback?: (hash: string | undefined) => void,
  options?: {
    name?: 'SHA-256' | 'PBKDF2';
    encode?: 'base64' | 'utf8' | 'hex';
    salt?: any;
    hash?: string;
    length?: number;
    iterations?: number;
  }
): Promise<string | undefined>
```

**Parameters:**

| Name | Description |
|---|---|
| `data` | The content to hash or derive a key from. |
| `pair` | (Optional) Used as salt. Specifically, `pair.epub` is extracted. If omitted, a random salt is generated, making the result non-deterministic. |
| `options.name` | `'PBKDF2'` (default) or `'SHA-256'` for a straight hash. |
| `options.encode` | Output encoding: `'base64'` (default), `'utf8'`, or `'hex'`. |
| `options.iterations` | PBKDF2 iteration count. Default: 100 000. |
| `options.length` | Derived key length in bits. Default: 512 (64 bytes). |

**How it works:**

- If `options.name` starts with `"sha"` (case-insensitive), it performs a
  single SHA-256 hash and returns the result.
- Otherwise it imports the data as a PBKDF2 key and derives bits using the
  configured iterations, salt, and hash algorithm.
- After derivation, the input data buffer is overwritten with random bytes to
  protect passphrases in memory.

**Example -- deterministic password hash (with known salt):**

```javascript
const pair = await SEA.pair();

// Using pair as salt makes the hash deterministic and reproducible
const hash = await SEA.work('my-password', pair);
console.log(hash); // base64 string, same every time for same inputs
```

**Example -- simple SHA-256 hash:**

```javascript
const hash = await SEA.work('hello', null, null, { name: 'SHA-256' });
console.log(hash); // base64-encoded SHA-256 digest
```

**Example -- hex-encoded hash:**

```javascript
const hash = await SEA.work('data', null, null, {
  name: 'SHA-256',
  encode: 'hex'
});
console.log(hash); // hex string
```

**Example -- non-deterministic hash (random salt):**

```javascript
// Omitting pair means a random salt is used each time
const hash1 = await SEA.work('password');
const hash2 = await SEA.work('password');
console.log(hash1 === hash2); // false -- different random salts
```

**Use case in authentication:** When a user creates an account, GUN calls
`SEA.work(password, salt)` to produce a proof. This proof is then used as the
AES key to encrypt the user's private keys. On login, the same proof is
re-derived from the password + stored salt and used to decrypt the private keys.
The 100 000-iteration PBKDF2 makes brute-force attacks computationally
expensive.

---

## SEA.secret(key, pair [, callback, options])

Derive a shared secret between two parties using Elliptic Curve Diffie-Hellman
(ECDH).

**Signature:**

```typescript
SEA.secret(
  key: string | { epub: string },
  pair: { epriv: string; epub: string },
  callback?: (secret: string | undefined) => void
): Promise<string | undefined>
```

**Parameters:**

| Name | Description |
|---|---|
| `key` | The **other** user's public encryption key (`epub`), as a string or `{ epub }`. |
| `pair` | **Your** encryption key pair containing `epub` and `epriv`. |

**Returns:** A base64url-encoded string representing the derived AES-256-GCM
key, or `undefined` on failure.

**How it works:**

1. Imports the other party's public ECDH key and your private ECDH key.
2. Derives 256 shared bits via ECDH `deriveBits`.
3. Imports those bits as a raw AES-GCM-256 key.
4. Exports the AES key as JWK and returns the `k` field (the symmetric key
   material).

The resulting secret is symmetric: both parties derive the same value.

**Example -- Alice and Bob:**

```javascript
const alice = await SEA.pair();
const bob = await SEA.pair();

// Alice derives the shared secret using Bob's epub and her own pair
const aliceSecret = await SEA.secret(bob.epub, alice);

// Bob derives the same shared secret using Alice's epub and his own pair
const bobSecret = await SEA.secret(alice.epub, bob);

console.log(aliceSecret === bobSecret); // true

// Now they can encrypt/decrypt messages that only they can read
const encrypted = await SEA.encrypt('Hello Bob!', aliceSecret);
const decrypted = await SEA.decrypt(encrypted, bobSecret);
console.log(decrypted); // "Hello Bob!"
```

**Example -- using object form:**

```javascript
const secret = await SEA.secret({ epub: otherUser.epub }, myPair);
```

---

## SEA.certify(who, policy, authority [, callback, options])

Create a cryptographically signed certificate that grants write permission to
other users on your graph.

> **Note:** `SEA.certify()` is marked as an early experimental community
> method. The API may change without warning in future versions.

**Signature:**

```typescript
SEA.certify(
  who: '*' | string | string[] | { pub: string } | { pub: string }[],
  policy: Policy,
  authority: { priv: string; pub: string },
  callback?: (cert: string | undefined) => void,
  options?: { expiry?: number; block?: string | { read?: string; write?: string } }
): Promise<string>
```

**Parameters:**

| Name | Description |
|---|---|
| `who` | The certificant(s): `'*'` for everyone, a pub key string, an array of pub key strings, or objects with `{ pub }`. |
| `policy` | Rules defining where the certificant can write. See Policy section below. |
| `authority` | Your key pair (the certificate issuer). Must contain `priv` and `pub`. |
| `options.expiry` | A Unix timestamp (milliseconds) after which the certificate expires. **If not set, the certificate is permanent.** |
| `options.block` | Path to a block list in your graph. If a certificant is found in the block list, their writes are rejected. |

**Returns:** A `"SEA"`-prefixed signed certificate string.

### Policy Types

```typescript
type Policy = string | IPolicy | (string | IPolicy)[];

interface IPolicy {
  '#'?: string;  // Path pattern (LEX match against soul)
  '.'?: string;  // Key pattern (LEX match against property name)
  '+'?: '*';     // Require the certificant's pub to appear in path or key
}
```

- A **string** policy matches against `path/key` using GUN's LEX/text.match.
- An **IPolicy** object lets you separately constrain the path (`#`) and key
  (`.`).
- The `+` field set to `'*'` forces the certificant's public key to appear
  somewhere in the path or key, ensuring users can only write to their own
  namespaced area.
- An **array** of policies grants permission if any single policy matches.

### Certificate Verification at Runtime

When a peer receives a `put` operation on another user's graph, the SEA
firewall (`sea/index.js`) checks:

1. Does the put contain a certificate (`+`) and the putter's pub (`*`)?
2. Is the certificate validly signed by the graph owner?
3. Has the certificate expired (`e` field vs. current state)?
4. Is the putter in the certificants list (`c` field)?
5. Does the path and key match at least one write policy (`w` field)?
6. If a block path is specified (`wb`), is the putter blocked?

If all checks pass, the write is allowed. Otherwise it is rejected with an
error.

### Important Constraints

- Certificates **must not be encrypted**. They need to be plain text so every
  peer in the network can enforce the same rules.
- If `options.expiry` is **not** set, the certificate never expires. This is
  dangerous for production systems -- always set an expiry.
- The certificate data format uses reserved keys: `c` (certificants), `e`
  (expiry), `r` (read policy), `w` (write policy), `rb` (read block), `wb`
  (write block).

**Example -- grant a single user write access to your inbox:**

```javascript
const alice = await SEA.pair();
const bobPub = 'bob-public-key-here';

const cert = await SEA.certify(bobPub, { '#': 'inbox' }, alice, null, {
  expiry: Date.now() + (1000 * 60 * 60 * 24) // 24 hours
});

console.log(cert);
// Bob can now write to alice's graph at paths matching "inbox"
```

**Example -- grant everyone write access with namespace isolation:**

```javascript
const alice = await SEA.pair();

// Everyone can write, but only under a path that contains their own pub
const cert = await SEA.certify('*', [
  { '#': 'chat', '.': 'msg', '+': '*' }
], alice, null, {
  expiry: Date.now() + (1000 * 60 * 60) // 1 hour
});
```

**Example -- grant multiple users:**

```javascript
const cert = await SEA.certify(
  [bob.pub, charlie.pub],
  'documents',
  alice,
  null,
  { expiry: Date.now() + (1000 * 60 * 60 * 24 * 7) } // 1 week
);
```

**Example -- using a certificate when writing to another user's graph:**

```javascript
// Bob writes to Alice's graph using the certificate
const gun = Gun();
const user = gun.user();
user.auth(bob); // authenticate as Bob

// When putting data, pass the certificate in the options
gun.get('~' + alice.pub).get('inbox').get('msg1').put('Hello Alice', null, {
  opt: { cert: cert }
});
```

---

## gun.user() -- High-Level User Chain

The `gun.user()` chain provides account management on top of SEA. Only one
user can be logged in per GUN instance at a time.

### user.create(alias, password [, callback, options])

Create a new user account.

```javascript
const gun = Gun();
const user = gun.user();

user.create('alice', 'password123', function (ack) {
  if (ack.err) {
    console.error(ack.err); // "User already created!" or "Password too short!"
  } else {
    console.log('User created, pub:', ack.pub);
  }
});
```

**What happens internally:**

1. Checks if the alias already exists in `~@alias`.
2. Generates a random 64-character salt.
3. Runs `SEA.work(password, salt)` to produce a PBKDF2 proof.
4. Calls `SEA.pair()` to generate fresh keys (or uses a provided pair).
5. Encrypts `{ priv, epriv }` with the proof using `SEA.encrypt()`.
6. Stores `{ pub, epub, alias, auth: { ek, s } }` at `~<pub>` in the graph.
7. Creates an alias mapping at `~@<alias>` pointing to `~<pub>`.

Password must be at least 8 characters (unless `opt.check` is `false`).

### user.auth(alias, password [, callback, options])

Authenticate with username and password.

```javascript
user.auth('alice', 'password123', function (ack) {
  if (ack.err) {
    console.error(ack.err);
  } else {
    console.log('Logged in as:', user.is.alias);
    console.log('Public key:', user.is.pub);
  }
});
```

**What happens internally:**

1. Looks up the alias in `~@alias` to find the public key.
2. Retrieves the auth data (`{ ek, s }`) from `~<pub>.auth`.
3. Re-derives the PBKDF2 proof from the password + stored salt.
4. Decrypts the encrypted private keys using the proof.
5. Assembles the full `ISEAPair` and stores it in memory.

### user.auth(pair [, callback, options])

Authenticate directly with a key pair (skips password derivation).

```javascript
const pair = await SEA.pair();
user.auth(pair, function (ack) {
  console.log('Authenticated with key pair');
});
```

### user.leave()

Log out the current user. Clears the in-memory key pair and session storage.

```javascript
user.leave();
console.log(user.is); // undefined
```

### user.recall(options [, callback])

Restore a session from browser sessionStorage.

```javascript
user.recall({ sessionStorage: true }, function (ack) {
  if (ack.err) {
    console.log('No session to restore');
  }
});
```

When `remember` is enabled (via recall or auth options), the key pair is saved
to `sessionStorage` and automatically restored on page reload.

### user.is

After authentication, `user.is` contains:

```javascript
{
  alias: 'alice',      // or the pub key if no alias
  pub: '...',          // signing public key
  epub: '...'          // encryption public key
}
```

Returns `undefined` when not authenticated.

---

## Automatic Data Signing and Verification

When SEA is loaded, it installs a GUN adapter (`Gun.on('opt', ...)`) that
intercepts every `put` operation. The adapter in `sea/index.js` enforces
security rules at the HAM (Hypothetical Amnesia Machine) diff level:

### Write Path (outgoing puts)

When an authenticated user writes data to their own graph (`~<pub>/...`):

1. The data is packed with graph metadata (soul, key, value, state).
2. `SEA.sign()` is called with the user's key pair.
3. The signed value `{ ':', <value>, '~': <signature> }` replaces the raw
   value.

When writing to another user's graph with a certificate:

1. The certificate (`+`) and putter's pub (`*`) are injected into the signed
   payload.
2. The receiving peer verifies both the signature and the certificate.

### Read Path (incoming puts)

For every incoming put, the SEA firewall checks:

- **Alias nodes** (`~@`): Value must exactly equal its key (self-enforced).
- **Public key lists** (`~@alias`): Each entry's key must match its link ID.
- **User data** (`~<pub>/...`): Signature must verify against the soul's
  public key. If a certificate is present, it must also be valid.
- **Content-addressed data** (`#hash`): The SHA-256 hash of the value must
  match the key.
- **Other data**: Passes through unless `opt.secure` is enabled.

If verification fails, the data is rejected and does not propagate.

---

## Complete Quickstart Example

```javascript
// -- Setup --
const Gun = require('gun');
require('gun/sea');
const SEA = Gun.SEA;

async function quickstart() {
  // 1. Generate key pairs for two users
  const alice = await SEA.pair();
  const bob = await SEA.pair();

  console.log('Alice pub:', alice.pub);
  console.log('Bob pub:', bob.pub);

  // 2. Sign and verify
  const signed = await SEA.sign('Hello from Alice', alice);
  console.log('Signed:', signed);

  const verified = await SEA.verify(signed, alice.pub);
  console.log('Verified:', verified); // "Hello from Alice"

  const tampered = await SEA.verify(signed, bob.pub);
  console.log('Wrong key:', tampered); // undefined

  // 3. Encrypt and decrypt with own key pair
  const encrypted = await SEA.encrypt({ secret: 'data' }, alice);
  console.log('Encrypted:', encrypted);

  const decrypted = await SEA.decrypt(encrypted, alice);
  console.log('Decrypted:', decrypted); // { secret: "data" }

  // 4. Encrypt and decrypt with passphrase
  const encPass = await SEA.encrypt('message', 'shared-password');
  const decPass = await SEA.decrypt(encPass, 'shared-password');
  console.log('Passphrase decrypt:', decPass); // "message"

  // 5. Shared secret between Alice and Bob (ECDH)
  const aliceSecret = await SEA.secret(bob.epub, alice);
  const bobSecret = await SEA.secret(alice.epub, bob);

  console.log('Secrets match:', aliceSecret === bobSecret); // true

  // Alice encrypts for Bob
  const forBob = await SEA.encrypt('Private message for Bob', aliceSecret);

  // Bob decrypts
  const fromAlice = await SEA.decrypt(forBob, bobSecret);
  console.log('Bob reads:', fromAlice); // "Private message for Bob"

  // 6. Hashing / Proof of Work
  const hash = await SEA.work('some data', null, null, { name: 'SHA-256' });
  console.log('SHA-256 hash:', hash);

  const pbkdf2 = await SEA.work('password', alice);
  console.log('PBKDF2 derived:', pbkdf2);

  // 7. Certificates -- Alice grants Bob write access
  const cert = await SEA.certify(bob.pub, { '#': 'messages' }, alice, null, {
    expiry: Date.now() + (1000 * 60 * 60) // expires in 1 hour
  });
  console.log('Certificate:', cert);

  // 8. Full user flow with GUN
  const gun = Gun();
  const user = gun.user();

  // Create account
  user.create('alice2', 'secure-pass-123', function (ack) {
    if (ack.err) return console.error(ack.err);

    // Authenticate
    user.auth('alice2', 'secure-pass-123', function (ack) {
      if (ack.err) return console.error(ack.err);

      console.log('Logged in as:', user.is.alias);
      console.log('Public key:', user.is.pub);

      // Write data (automatically signed by SEA)
      user.get('profile').put({ name: 'Alice', age: 30 });

      // Read it back (automatically verified by SEA)
      user.get('profile').once(function (data) {
        console.log('Profile:', data);
      });

      // Log out
      user.leave();
    });
  });
}

quickstart().catch(console.error);
```

---

## SEA Internal Format Reference

SEA-encoded strings always start with the prefix `"SEA"` followed by a JSON
object. The prefix is stripped during parsing.

| Function | Encoded format |
|---|---|
| `SEA.sign` | `SEA{"m": <data>, "s": "<base64 signature>"}` |
| `SEA.encrypt` | `SEA{"ct": "<base64 ciphertext>", "iv": "<base64 IV>", "s": "<base64 salt>"}` |
| `SEA.certify` | `SEA{"m": "<JSON cert data>", "s": "<base64 signature>"}` |

Signed data stored in the graph uses the internal format:

```json
{ ":": <plain value>, "~": "<signature>" }
```

When a certificate is involved, two additional fields are added:

```json
{ ":": <plain value>, "~": "<signature>", "+": <certificate>, "*": "<putter pub>" }
```

---

## Configuration (SEA.opt / Settings)

SEA's internal settings are exposed on `SEA.opt` (defined in `sea/settings.js`):

| Setting | Default | Description |
|---|---|---|
| `pbkdf2.hash` | `{ name: 'SHA-256' }` | PBKDF2 hash algorithm |
| `pbkdf2.iter` | `100000` | PBKDF2 iteration count |
| `pbkdf2.ks` | `64` | PBKDF2 key size in bytes |
| `ecdsa.pair` | `{ name: 'ECDSA', namedCurve: 'P-256' }` | ECDSA key generation params |
| `ecdsa.sign` | `{ name: 'ECDSA', hash: { name: 'SHA-256' } }` | ECDSA signing params |
| `ecdh` | `{ name: 'ECDH', namedCurve: 'P-256' }` | ECDH key agreement params |
| `recall.validity` | `43200` (12 hours in seconds) | Session recall validity |
| `shuffle_attack` | `1546329600000` (Jan 1, 2019) | Timestamp threshold for legacy format migration |
| `fallback` | `2` | Number of legacy verification rounds to attempt |
| `check(t)` | -- | Returns `true` if string starts with `"SEA{"` |
| `parse(t)` | -- | Strips `"SEA"` prefix and parses JSON |

---

## Troubleshooting

### "ReferenceError: CryptoKey is not defined" (Node.js)

The `@peculiar/webcrypto` package is missing or failed to load. Install it:

```bash
npm install @peculiar/webcrypto --save
```

If `Gun.SEA` is `undefined` after requiring `gun/sea`, you may need to
manually wire it:

```javascript
const SEA = require('gun/sea');
const Gun = require('gun');
Gun.SEA = SEA;
```

### HTTPS Required in Browser

SEA's WebCrypto dependency requires a secure context (HTTPS). On non-localhost
HTTP pages, SEA will attempt to redirect to HTTPS automatically. For local
development, use `localhost` or `127.0.0.1`.

### Node.js Buffer Shim

SEA provides its own Buffer implementation (`sea/buffer.js`) compatible with
Node's `safe-buffer`. In environments where `Buffer` is not available, SEA
polyfills `btoa` and `atob` using the `buffer` npm package:

```bash
npm install buffer
```

### React Native

SEA requires a WebCrypto-compatible environment. React Native does not provide
one by default. Consult the GUN React Native documentation for platform-specific
shim installation.

### Signature Verification Failures on Legacy Data

If you are verifying data that was signed with an older version of GUN/SEA, the
signature encoding may be different (UTF-8 vs. base64). SEA's fallback
mechanism automatically tries legacy formats. If you still encounter issues,
check `SEA.opt.fallback` and ensure it is set to `2` (the default).

### Password Changes

To change a user's password, call `user.auth()` with the current credentials
and pass `{ change: 'new-password' }` in the options:

```javascript
user.auth('alice', 'old-password', function (ack) {
  // password changed
}, { change: 'new-password' });
```

This re-encrypts the private keys with a new PBKDF2 proof derived from the new
password and a fresh salt.

---

## Security Considerations

1. **Private keys stay in memory.** After authentication, `priv` and `epriv`
   exist only in the GUN instance's memory (`user._.sea`). They are never
   written to the graph in plaintext -- only the AES-encrypted form is stored.

2. **PBKDF2 brute-force resistance.** The 100 000-iteration PBKDF2 derivation
   makes offline password attacks expensive. However, weak passwords are still
   vulnerable. Enforce strong passwords at the application level.

3. **Certificate expiry.** Certificates without an `expiry` are permanent.
   Always set an expiry for production certificates and implement a renewal
   flow.

4. **Trust model.** SEA verifies signatures against public keys, but it does
   not inherently establish trust. Your application must decide which public
   keys to trust (e.g., through a friend list, certificate authority, or
   out-of-band verification).

5. **No forward secrecy.** ECDH shared secrets are static for a given key pair
   combination. If a private key is compromised, all past messages encrypted
   with that shared secret are exposed. Consider rotating key pairs for
   sensitive applications.

6. **Peer enforcement.** Every peer in the GUN network independently verifies
   signatures and certificates. A malicious peer can relay unsigned data, but
   honest peers will reject it. The security model depends on at least some
   peers being honest.
