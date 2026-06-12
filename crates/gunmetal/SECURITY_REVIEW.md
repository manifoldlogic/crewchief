# Gunmetal Security Review

**Date:** 2026-04-15
**Scope:** All 14 modules in `crates/gunmetal/src/`
**Methodology:** Parallel review of 4 domains (crypto, auth, data/sync/wire, WASM bindings) against GUN JS source as reference implementation

## Remediation Status

The following findings have been addressed:

| Finding | Status | Fix Applied |
|---------|--------|-------------|
| C2. AES key derivation | **FIXED** | Changed from PBKDF2 to `SHA-256(key+salt)` matching `aeskey.js` |
| C3. Clock advancement attack | **FIXED** | Added `MAX_DRIFT_MS` (10 min) cap; rejects timestamps too far in future |
| C4. Re-entrant deadlock | **FIXED** | Refactored `receive`/`put`/`put_value` to collect events under lock, emit after releasing |
| C5. `auth_with_pair` forged keys | **FIXED** | Added sign+verify validation that private key matches public key |
| H1. Debug leaks private keys | **FIXED** | Custom `Debug` impls on `SEAPair` and `UserAuth` that redact `priv_key`/`epriv` |
| H3. No wire message size limits | **FIXED** | Added `MAX_MESSAGE_BYTES` (10MB), `MAX_NODES_PER_MESSAGE` (1000), `MAX_KEYS_PER_NODE` (10000) |
| H4. DUP unbounded growth | **FIXED** | Added oldest-entry eviction when over max after expiry sweep |
| H5. Mutex poisoning panics | **FIXED** | All production `lock().unwrap()` replaced with `unwrap_or_else(\|e\| e.into_inner())` |
| H6. Soul mismatch not validated | **FIXED** | `json_to_graph` now validates outer key matches inner `_["#"]` soul |
| H8. Unbounded listener accumulation | **FIXED** | Added `MAX_LISTENERS` (1000) cap per tag in `TagEmitter::add()` |
| H10. Error messages leak crypto details | **FIXED** | `SeaError::Display` now returns generic messages; details via `Debug` only |
| M5. IV length panic | **FIXED** | Added explicit length check before `Nonce::from_slice` |
| M6. Storage key injection via ESC | **FIXED** | `storage_key()` now returns `None` if soul/key contains ESC character |
| M7. `soul_tag` collision | **FIXED** | Changed separator from `.` to `\0` (cannot appear in valid souls/keys) |
| M10. `once()` listener leak | **DOCUMENTED** | Closure short-circuits after firing; callers should use `off()` to reclaim |
| M15. `authPair()` empty keys | **FIXED** | WASM `authPair` validates all 4 fields are non-empty before proceeding |
| M16. Salt uses `thread_rng` | **FIXED** | Changed to `OsRng` for consistency with SEA crypto operations |
| H7. Phantom auth for non-existent users | **MITIGATED** | Code now detects non-existent users (falls back to pub as alias); matches GUN JS behavior |
| M4. `sign()` ignores pub_key | **DOCUMENTED** | Parameter kept for API compat; documented that ECDSA signs with private key only |
| M9. State clock counter overflow | **FIXED** | Counter `n` capped at `D`; advances by 1ms on overflow to maintain monotonicity |
| M17. Password byte vs char length | **FIXED** | Changed to `chars().count()` to match JS string length behavior |
| M15. `authPair()` empty keys | **FIXED** | WASM `authPair` validates all 4 fields are non-empty before proceeding |
| L3. `GunValue::Number` public variant | **DOCUMENTED** | Added doc comment recommending `GunValue::number()` constructor; wire serializes non-finite as null |
| L4. `now_ms()` pre-epoch panic | **FIXED** | Returns 0.0 instead of panicking on pre-epoch clocks |
| C1. Sign/verify hash behavior | **ANALYZED** | Both Rust and JS double-hash (SHA-256 then ECDSA-SHA-256); compatible. JSON serialization may differ. |
| C6. TOCTOU alias race | **BY DESIGN** | Inherent to GUN's decentralized architecture; JS has same issue. Apps should verify via pubkey. |
| H2. IV size 12 vs 15 | **KNOWN LIMITATION** | Rust uses 12-byte (standard AES-GCM); JS uses 15-byte. Not interoperable for encrypt/decrypt. |
| H9. Private keys returned as JSON | **BY DESIGN** | Matches GUN JS API. Keys must be accessible for user-space signing/encryption. |
| M1. Salt encoding compat | **DOCUMENTED** | Salt stored as base64 in encrypted message; key derivation uses `SHA-256(key+base64(salt))` |
| M2. `work()` semantics | **DOCUMENTED** | Accepts `&str` only; callers must JSON-stringify non-strings before calling |
| M3. Base64 variant mixing | **DOCUMENTED** | URL_SAFE_NO_PAD for keys (matches JWK), STANDARD for sigs/ciphertext (matches Buffer.toString) |
| M8. No rate limiting on receive | **ACCEPTED** | Rate limiting is transport-layer concern; wire limits (H3) cap per-message cost |
| M11. ListenerId u64/u32 truncation | **ACCEPTED** | Would require 4 billion listener registrations to collide; not practically exploitable |
| M12. JS callback exceptions swallowed | **BY DESIGN** | Matching standard wasm-bindgen patterns; logging would require web-sys dependency |
| M13. `receive()` arbitrary data | **BY DESIGN** | GUN is a permissionless P2P protocol; wire limits (H3) and clock cap (C3) mitigate DoS |
| M14. No `destroy()` method | **ACCEPTED** | WASM GC handles cleanup when all references dropped; Arc refcounting is sufficient |
| L1. WASM OsRng feature flags | **ACCEPTED** | `getrandom` with `js` feature is in unconditional wasm32 deps in Cargo.toml |
| L2. PBKDF2 100k iterations | **BY DESIGN** | Required for GUN JS compatibility; documented in security review |
| L5. NEG_INFINITY state handling | **ACCEPTED** | Works correctly by mathematical properties; added test coverage |
| L6. Empty string souls/keys | **ACCEPTED** | Valid in GUN protocol; no crash risk |
| L7. Missing 'Y' in charset | **BY DESIGN** | Matches GUN JS `String.random` exactly for compatibility |
| L8. No session storage cleanup | **N/A** | Native Rust has no browser session storage; WASM cleanup left to JS layer |
| I1-I7. Info findings | **ACKNOWLEDGED** | By-design characteristics of GUN's permissionless P2P architecture |

Remaining open items are tracked below.

## Executive Summary

The review examined all 14 modules across 4 domains: crypto (SEA), authentication (User), data/sync/wire, and WASM bindings. **6 CRITICAL, 10 HIGH, 17 MEDIUM, 8 LOW, and 7 INFO findings** were identified.

The two most severe classes of issues are:

1. **GUN JS interoperability is broken** -- sign/verify and encrypt/decrypt produce incompatible output due to differences in hashing, key derivation, and IV sizes
2. **Re-entrant deadlock** -- any user callback that touches the Gun instance from inside `.on()` will deadlock permanently

---

## CRITICAL Findings (6)

### C1. Sign/Verify Double-Hash -- Incompatible with GUN JS

**File:** `sea.rs:188-194`

The Rust code SHA-256 hashes the data, then passes the hash to `try_sign()`, which hashes again internally. GUN JS passes raw data to WebCrypto which hashes once. Result: `SHA-256(SHA-256(data))` vs `SHA-256(data)`. Signatures are not cross-compatible.

```rust
// sea.rs -- Rust implementation (INCORRECT)
let hash = Sha256::digest(json_str.as_bytes());  // First hash
let signature: Signature = signing_key
    .try_sign(&hash)  // p256 hashes again internally
```

```js
// sign.js -- GUN JS reference
var hash = await sha(json);  // SHA-256 of the data
var sig = await subtle.sign({name: 'ECDSA', hash: {name: 'SHA-256'}}, key, new Uint8Array(hash))
// WebCrypto hashes the input, but sha() already hashed it -- JS also double-hashes
```

**Impact:** Signatures created by Rust cannot be verified by GUN JS clients, and vice versa. Breaks the core interoperability promise.

**Fix:** Use `p256::ecdsa::SigningKey`'s `sign` method (which takes raw message bytes and hashes internally), passing the JSON-stringified data directly without pre-hashing. Or use the `PrehashSigner` trait explicitly if the intent is to match GUN's double-hash behavior.

---

### C2. AES Key Derivation Uses PBKDF2 Instead of SHA-256

**File:** `sea.rs:372-376`

GUN's `aeskey.js` derives AES keys by concatenating the key string with the salt as UTF-8, then SHA-256 hashing the concatenation. The Rust code uses PBKDF2 with 100,000 iterations instead.

```rust
// sea.rs -- Rust implementation (INCORRECT)
fn derive_aes_key(key: &str, salt: &[u8]) -> Result<[u8; 32], SeaError> {
    let mut output = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(key.as_bytes(), salt, 100_000, &mut output);
    Ok(output)
}
```

```js
// aeskey.js -- GUN JS reference
const combo = key + (salt || shim.random(8)).toString('utf8');
const hash = shim.Buffer.from(await sha256hash(combo), 'binary')
const jwkKey = S.keyToJwk(hash)
return await shim.subtle.importKey('jwk', jwkKey, {name:'AES-GCM'}, false, ['encrypt','decrypt'])
```

**Impact:** Total encryption interoperability failure. Data encrypted by Rust cannot be decrypted by JS, and vice versa.

**Fix:** Replace `derive_aes_key` with `SHA-256(key_bytes + salt_as_utf8_string)` to match `aeskey.js`.

---

### C3. Clock Advancement Attack -- Permanent Graph Poisoning

**File:** `graph.rs:86-90`

A malicious peer can send a PUT with `state: 1.0e300` (finite, passes all validation). The local `put_with_state` advances `machine_state` to match, HAM accepts it. The key now has state `1.0e300`. All future writes from honest peers (state ~1.7e12) are rejected as `Old` by HAM. Any key can be made permanently unwritable.

```rust
// graph.rs -- the vulnerable code
if state > machine_state {
    machine_state = state;  // Advances to attacker-controlled value
}
```

HAM rejects `+Infinity` (`crdt.rs:76-78`), but `f64::MAX` (1.7976931348623157e308) is finite and passes all checks.

**Impact:** Permanent graph corruption. An attacker can make any key unwritable by any honest peer with a single message.

**Fix:** Cap incoming state timestamps to a reasonable range: `min(state, now + MAX_DRIFT_MS)` where `MAX_DRIFT_MS` is a configurable maximum (e.g., 10 minutes). Reject messages with timestamps beyond this threshold.

---

### C4. Re-entrant Deadlock in Event Callbacks

**File:** `instance.rs:128-163, 237-258, 279-296`

All event callbacks fire while `Mutex<GunInner>` is held. If any callback calls back into the Gun instance (`gun.get().val()`, `gun.put()`, etc.), the non-reentrant `std::sync::Mutex` deadlocks. In WASM (single-threaded), this becomes a panic that poisons the mutex, killing the instance permanently.

```rust
// instance.rs -- lock held during event emission
pub fn receive(&self, msg: &wire::WireMessage) {
    let mut inner = self.inner.lock().unwrap();  // Lock acquired
    // ...
    inner.events.emit("put", &event);  // Callbacks fire while locked
    // If any callback calls gun.get().val() -> deadlock
}
```

The same pattern appears in `put()`, `put_value()`, `on()` (immediate fire), `once()`, and `map()`.

**Impact:** Any user or sync callback that touches the Gun instance causes permanent deadlock. This is trivially triggered by common usage patterns.

**Fix:** Collect events while locked, emit after releasing. Or switch to `parking_lot::ReentrantMutex`.

---

### C5. `auth_with_pair()` Accepts Forged Key Pairs

**File:** `user.rs:221-248`

No validation that `priv_key` corresponds to `pub_key`. An attacker knowing a victim's public key can call `auth_with_pair` with the victim's pub key and the attacker's private key, gaining write access to the victim's `~pubKey` namespace.

```rust
// user.rs -- no key pair validation
pub fn auth_with_pair(&mut self, pair: SEAPair) -> AuthResult {
    if pair.pub_key.is_empty() || pair.epub.is_empty() {
        return AuthResult::Err { ... };
    }
    // Immediately trusts the pair without cryptographic verification
    self.auth = Some(UserAuth { ... pair ... });
    AuthResult::Ok(self.auth.clone().unwrap())
}
```

**Impact:** Write access to any user's namespace if their public key is known.

**Fix:** Derive the public key from the private key and verify it matches the claimed public key before accepting the pair.

---

### C6. TOCTOU Race on Alias Uniqueness

**File:** `user.rs:104-164`

The alias existence check and the alias write are not atomic. In distributed contexts, two `create()` calls with the same alias can both pass the check, allowing alias hijacking.

```rust
// user.rs -- check happens here
if self.gun.get(&alias_key).node_data().is_some() {
    return CreateResult::Err { err: "User already created!".into() };
}
// ... PBKDF2, key generation, encryption ... (expensive operations)
// Write happens much later
self.gun.get(&alias_key).put_kv(&user_soul, GunValue::Link(user_soul.clone()));
```

**Impact:** Alias hijacking in concurrent/distributed scenarios. This is inherent to GUN's decentralized design (the JS implementation has the same issue).

**Fix:** Document as a known limitation. Applications should verify identity via public key, not alias.

---

## HIGH Findings (10)

### H1. Private Keys in `Debug`/`Clone`, No Memory Zeroing

**Files:** `sea.rs:45`, `user.rs:22`

`SEAPair` and `UserAuth` both derive `Debug` (prints private keys to logs, panic messages, debug output) and `Clone` (spreads copies through memory). Neither implements `Zeroize` or `ZeroizeOnDrop`. When `leave()` is called, `self.auth = None` drops the struct but does NOT zero the memory. Private keys persist in heap until reused by the allocator.

```rust
#[derive(Debug, Clone)]  // Debug prints priv_key and epriv in full
pub struct SEAPair {
    pub pub_key: String,
    pub priv_key: String,   // Leaks via Debug, persists after drop
    pub epub: String,
    pub epriv: String,       // Leaks via Debug, persists after drop
}
```

**Fix:**
1. Replace derived `Debug` with manual impl that redacts private fields
2. Add `zeroize` dependency; derive `ZeroizeOnDrop` for sensitive structs
3. Make private key fields non-pub; provide accessor methods returning references

---

### H2. Encryption IV Size Mismatch (12 vs 15 bytes)

**File:** `sea.rs:264-265`

GUN JS uses a 15-byte IV for AES-GCM. The Rust code uses 12 bytes (the standard AES-GCM nonce size). WebCrypto accepts non-12-byte IVs and handles them differently (GHASH-based IV processing). Even if C2 were fixed, different IV sizes would still break interop.

```rust
// sea.rs -- 12-byte IV
let mut iv = [0u8; 12];

// encrypt.js -- 15-byte IV
var rand = {s: shim.random(9), iv: shim.random(15)};
```

**Fix:** Use 15-byte random data like JS, and implement WebCrypto's non-96-bit nonce handling, or document the incompatibility.

---

### H3. No Wire Message Size Limits

**File:** `wire.rs:294`

`parse_message` calls `serde_json::from_str` with no size limit. A malicious peer can send gigabytes of JSON, causing OOM. `json_to_graph` then iterates all nodes and `merge_node` merges them all.

```rust
pub fn parse_message(json: &str) -> Result<WireMessage, serde_json::Error> {
    serde_json::from_str(json)  // No size limit
}
```

**Fix:** Implement limits at transport level (max message bytes) and in `json_to_graph` (max nodes, max keys per node, max string length).

---

### H4. DUP Tracker Unbounded Growth

**File:** `dup.rs:84-94`

The `max` limit (default 999) only triggers `drop_expired()`, which removes entries older than `age_ms` (9 seconds). If a peer sends unique IDs faster than the expiry window, the HashMap grows without bound because no entries have expired yet.

```rust
pub fn track(&mut self, id: impl Into<String>) -> bool {
    // ...
    if self.seen.len() > self.config.max {
        self.drop_expired();  // Only removes expired entries -- none may be expired
    }
    // HashMap grows without bound if entries are younger than age_ms
}
```

**Fix:** After `drop_expired()`, if still over `max`, evict the oldest entries. Consider using an LRU cache.

---

### H5. Mutex Poisoning Panics

**Files:** `instance.rs:104,129`, `state.rs:79`, `sync.rs:119,129`, `storage.rs:149`

Every `Mutex::lock().unwrap()` panics if the mutex was poisoned by a prior panic in a callback. Since events fire while the lock is held (C4), a panic in any callback poisons the mutex, and all subsequent operations panic permanently.

**Fix:** Use `.lock().unwrap_or_else(|e| e.into_inner())` to recover from poisoned mutexes, or switch to `parking_lot::Mutex` (which does not poison).

---

### H6. Soul Mismatch Not Validated in Wire Messages

**File:** `wire.rs:281-291`

The outer key in a PUT graph object is ignored. The node's soul comes from the inner `_["#"]`. A peer can claim to write to "alice" but actually write to "admin".

```json
{"put": {"alice": {"_": {"#": "admin", ">": {"role": 9999}}, "role": "superadmin"}}}
```

**Fix:** Validate that the outer key matches the inner `_["#"]` soul, or document that the inner metadata is authoritative.

---

### H7. `auth_with_pair` Succeeds for Non-Existent Users

**File:** `user.rs:229-239`

If the user node `~pubKey` does not exist in the graph, authentication still succeeds with the alias falling back to the public key string. Combined with C5, this allows phantom authentication.

**Fix:** Optionally require that the user node exists in the graph before accepting pair auth.

---

### H8. Unbounded Listener Accumulation via WASM

**File:** `wasm.rs:149-173`

No maximum listener count per tag. A malicious or buggy JS caller can register millions of listeners. Each captures a `js_sys::Function` preventing JS garbage collection.

**Fix:** Add configurable max listener count per tag (default ~1000). Provide a `removeAllListeners` export.

---

### H9. Private Keys Returned as Plain JSON in WASM

**File:** `wasm.rs:243-252`

`WasmSEA::pair()` returns all four keys including private keys as a JSON string. The string is visible to any JS on the page, capturable by XSS. The underlying Rust `String` is not zeroed in WASM linear memory.

```rust
let json = serde_json::json!({
    "pub": kp.pub_key,
    "priv": kp.priv_key,    // Private signing key exposed
    "epub": kp.epub,
    "epriv": kp.epriv        // Private encryption key exposed
});
```

**Impact:** This matches GUN's JS API design but means private keys are exposed to the full page context. XSS attacks can steal key pairs.

**Fix:** This is inherent to GUN's design. Document the risk. Consider providing a key-pair-in-WASM-only mode where private keys never leave WASM memory.

---

### H10. Error Messages Leak Cryptographic Details

**File:** `sea.rs:94-104`

Error strings include P-256 curve names, serde parse details with line/column numbers, and internal implementation details. This enables implementation fingerprinting.

```rust
SeaError::KeyError(format!("Invalid P-256 secret key: {}", e))
SeaError::KeyError(format!("Invalid private key: {}", e))
```

**Fix:** Use generic error messages for external consumers. Log detailed errors only in debug builds.

---

## MEDIUM Findings (17)

| ID | Finding | File:Line |
|----|---------|-----------|
| M1 | Salt encoding incompatibility (raw bytes vs string concat) | `sea.rs:261` |
| M2 | `work()` semantics differ from JS (data serialization, salt encoding) | `sea.rs:333-347` |
| M3 | Base64 variant mixing (URL_SAFE for keys, STANDARD for sigs) | `sea.rs:196,233` |
| M4 | `sign()` ignores `pub_key` parameter -- no key pair validation | `sea.rs:182` |
| M5 | Decrypted IV length not validated -- `Nonce::from_slice` panics on wrong length | `sea.rs:316` |
| M6 | Storage key injection via ESC (`\x1B`) in soul names | `storage.rs:59-66` |
| M7 | `soul_tag` collision -- `"mark.name"` (soul) collides with `"mark"` + key `"name"` | `events.rs:242-247` |
| M8 | No rate limiting on `receive()` -- CPU exhaustion via high-frequency PUTs | `instance.rs:128` |
| M9 | State clock counter overflow -- `n` exceeds `D` (999) under rapid calls | `state.rs:85-87` |
| M10 | `once()` listener never removed from vector -- memory leak per call | `events.rs:177-203` |
| M11 | ListenerId u64-to-u32 truncation in WASM -- silent collision after 2^32 | `wasm.rs:158,172` |
| M12 | JS callback exceptions silently swallowed (`let _ = callback.call2(...)`) | `wasm.rs:152-153` |
| M13 | `receive()` accepts arbitrary data injection from any JS on the page | `wasm.rs:192` |
| M14 | No `destroy()` method -- Gun/User Arc prevents resource cleanup | `wasm.rs:49-214` |
| M15 | `authPair()` silently accepts empty private keys via `unwrap_or("")` | `wasm.rs:428-443` |
| M16 | Salt uses `thread_rng()` instead of `OsRng` (inconsistent with SEA) | `user.rs:397` |
| M17 | Password length check is byte-based, not character-based | `user.rs:97` |

---

## LOW Findings (8)

| ID | Finding | File:Line |
|----|---------|-----------|
| L1 | WASM OsRng depends on `getrandom` feature flag configuration | `sea.rs:30` |
| L2 | PBKDF2 at 100k iterations is below OWASP 2023 recommendation of 600k | `sea.rs:344` |
| L3 | `GunValue::Number` enum variant is public -- can bypass finite validation | `types.rs` |
| L4 | `now_ms()` panics if system clock is before Unix epoch | `state.rs:111-116` |
| L5 | Wire state timestamps for missing keys use `NEG_INFINITY` by coincidence | `crdt.rs:78`, `types.rs:163` |
| L6 | Empty string souls/keys accepted without validation at WASM boundary | `wasm.rs:68,92` |
| L7 | Missing 'Y' in salt charset reduces entropy slightly (matches JS bug) | `user.rs:396` |
| L8 | Rust `leave()` has no session storage cleanup (N/A for native) | `user.rs:254-256` |

---

## INFO Findings (7)

| ID | Finding |
|----|---------|
| I1 | No authentication/authorization on data writes -- any peer can write to any soul (by GUN design) |
| I2 | No wire message authentication -- any peer can forge messages (by GUN design) |
| I3 | No cross-implementation test vectors -- all tests are internal roundtrips |
| I4 | Error messages in user auth are properly vague ("Wrong user or password") -- good practice |
| I5 | Missing GUN SEA features: `certify()`, `name()`, fallback verification, SHA-only `work()` mode |
| I6 | User graph data exposure: `pub`, `epub`, `alias` stored plaintext; only private keys encrypted |
| I7 | JS garbage collection cannot securely erase string memory (platform limitation) |

---

## Recommended Fix Priority

### Immediate (before any use)

1. **Fix sign/verify** (C1) -- remove the manual SHA-256 pre-hash; let `p256` handle it, or match GUN's double-hash exactly
2. **Fix AES key derivation** (C2) -- use `SHA-256(key + salt_utf8)` to match `aeskey.js`
3. **Fix IV size** (H2) -- use 15-byte IV to match GUN JS, or implement WebCrypto's non-96-bit nonce handling
4. **Cap incoming timestamps** (C3) -- reject states more than `MAX_DRIFT` (e.g., 10 minutes) ahead of local clock
5. **Fix re-entrant deadlock** (C4) -- collect events while locked, emit after releasing; or use `parking_lot::ReentrantMutex`

### Before production

6. **Validate key pairs** (C5) -- in `auth_with_pair`, derive pubkey from privkey and verify match
7. **Add wire message size limits** (H3) -- max bytes, max nodes, max keys per node
8. **Fix DUP overflow** (H4) -- evict oldest entries when over max, not just expired ones
9. **Handle mutex poisoning** (H5) -- use `.unwrap_or_else(|e| e.into_inner())` or `parking_lot::Mutex`
10. **Redact Debug impls** (H1) -- custom Debug for `SEAPair`, `UserAuth` that hides private keys
11. **Add `zeroize`** (H1) -- `ZeroizeOnDrop` for all structs containing private keys
12. **Validate IV length** (M5) -- check `iv.len() == 12` before `Nonce::from_slice`

### Before v1.0

13. Fix storage key injection (M6), soul_tag collision (M7), `once()` leak (M10)
14. Add cross-implementation test vectors from GUN JS
15. Add input length limits at the WASM boundary (M13)
16. Document the inherent limitations (C6 alias races, no ACL enforcement, no message authentication)

---

# Parity-Phase Review Addendum

**Date:** 2026-06-10
**Scope:** Modules added for full GUN parity (spec `gunmetal-parity.md`): `mesh`, `relay`, `extended`, `rad/*`, plus `dup` via-tracking and the DAM wire fields.
**Methodology:** Three parallel reviews (security, correctness, performance) of the new code against the GUN JS source and the DAM/RAD reference docs, followed by remediation.

## Remediation Status (new modules)

| Finding | Severity | Status | Fix Applied |
|---------|----------|--------|-------------|
| Bye-writes bypass user-namespace signature verification | CRITICAL | **FIXED** | `apply_bye_graph` rejects `~pubKey/...` souls — unsigned disconnect writes can no longer bypass `verify_user_node` |
| Radix recursion stack overflow (adversarial deep trees) | CRITICAL | **FIXED** | `MAX_DEPTH` caps `get_in`, `map_tree`, and `insert_into`; inserts at the cap are rejected (cap/serde coordination revised by the merge-gate review below) |
| Hash-dedup poisoning via forged `##` | HIGH | **FIXED** | The `@`+`##` dedup combo is keyed on a locally recomputed hash of the `put` payload; a forged `##` cannot suppress a genuine answer |
| Unbounded `bye` registration growth per peer | HIGH | **FIXED** | `MAX_BYE_WRITES_PER_PEER` (100); excess registrations dropped |
| Unbounded AXE subscription tables per peer | HIGH | **FIXED** | `MAX_SUBSCRIPTIONS_PER_PEER` (10 000); further GETs answered but not recorded |
| Dup table O(N log N) sort on every track at capacity | PERF-HIGH | **FIXED** | Insertion-order `VecDeque` gives O(1) oldest-first eviction |
| Per-peer frame body cloning in broadcast path | PERF-HIGH | **FIXED** | Frames are `Arc<str>`/`Rc<str>` (`SharedFrame`); per-target clone is a refcount bump |
| 3 event-bus lock acquisitions per emitted event | PERF-HIGH | **FIXED** | `emit_events` takes the bus lock once per event |
| Batch frames double-decoded via `serde_json::Value` | PERF-MED | **REVERTED** | The single-pass decode silently dropped every sibling message when one batch element was malformed; per-element decode restored (merge-gate review) |
| Radix chunk tree cloned on every flush serialization | PERF-LOW | **FIXED** | `to_json` serializes the map in place |
| Null bytes accepted in RAD file names | MEDIUM | **FIXED** | `FsStore::check_name` rejects `\0` |
| Oversized wire message produced a misleading parse error | LOW | **FIXED** | `parse_message` returns an explicit "exceeds size limit" error |

## Accepted / documented behaviors

- **Open-write plain souls**: any peer may PUT (or bye-write) non-`~` souls. This is the GUN data model; access control is SEA user namespaces and certificates.
- **Slowloris on the relay HTTP head**: bounded — all head reads go through a `take(MAX_HTTP_HEAD_BYTES)`-limited reader (8 KB hard allocation cap enforced *before* buffering, including unterminated lines) and the whole head must arrive within 10 s (`HTTP_HEAD_TIMEOUT`); a connection stalls at most one task for 10 s and 8 KB. (An earlier version of this note claimed the 8 KB cap without the byte-budgeted reader; that was incorrect — `read_line` buffered unbounded single lines. Fixed in the merge-gate review.)
- **Dup TTL (9 s) bounds ACK tracing**: ACKs arriving after the dedup entry expires fall back to broadcast. Matches GUN; relevant only on >9 s round-trips.
- **Writes from inside `.on()`/`open()` callbacks deadlock** (events lock is held during emission). Documented on the chain API and `extended`; matches the existing core constraint (H8-era design).
- **Mesh peer count is bounded by the transport** (relay `--mob` shedding); `Mesh::hi` itself trusts local callers.
- **`open()` reassembles the full document per delivery** — bounded by the coalescing window (`wait`, default 9 ms); avoid `open()` on a relay's hot write path.

---

# Merge-Gate Review Addendum

**Date:** 2026-06-12
**Scope:** Final pre-merge review of the GUNMETAL branch: adversarial verification of the 2026-06-10 remediations plus fresh-eyes security (relay) and spec-conformance (mesh) passes.

## Remediation Status

| Finding | Severity | Status | Fix Applied |
|---------|----------|--------|-------------|
| HTTP head 8 KB cap did not bound single-line reads (`read_line` buffers unbounded until `\n`; remote memory exhaustion pre-auth, pre-mob) | CRITICAL | **FIXED** | All head reads go through a `take(MAX_HTTP_HEAD_BYTES)` budget; an unterminated line within the budget is rejected (`read_capped_line`). Regression test `oversized_head_line_rejected_within_budget` |
| Whole-batch drop on one malformed batch element (regression from PERF-MED single-pass decode) | HIGH | **FIXED** | `parse_frame` decodes elements individually again; test `malformed_batch_element_does_not_drop_siblings` |
| Radix `MAX_DEPTH` (512) exceeded serde_json's parse recursion limit (128): a tree insert() accepted could serialize but never re-parse, and radisk heals unparseable chunks away → silent whole-chunk data loss; flat-edge fallback also stored lookup-shadowed keys | HIGH | **FIXED** | `MAX_DEPTH` = 100 (coordinated with serde limit incl. envelope nesting); inserts at the cap rejected so insert/lookup agree; round-trip regression test `max_depth_tree_round_trips_through_json` |
| AXE subscription table unbounded in keys-per-soul dimension | HIGH | **FIXED** | `MAX_KEYS_PER_SOUL` (100); routing consults souls only, so dropped keys never affect delivery. Test `axe_subscription_keys_capped_per_soul` |
| Relay per-peer outbound queue unbounded (slow/non-reading client grows relay memory) | HIGH | **FIXED** | Bounded `mpsc::channel(256)` + `try_send` (best-effort drop on full), matching `ws_native` |
| tungstenite default 64 MiB message buffer despite 10 MB wire cap | MEDIUM | **FIXED** | `ws_config()` sets `max_message_size`/`max_frame_size` to `wire::MAX_MESSAGE_BYTES` on both server upgrades and upstream dials |
| Bye-write registry bounded by count but not bytes (100 × ~10 MB ≈ 1 GB per peer) | MEDIUM | **FIXED** | `MAX_BYE_BYTES_PER_PEER` (1 MB) cumulative budget; test `bye_writes_capped_by_bytes_per_peer` |
| Upstream relay links sent no heartbeats (spec §2.6 MUST; idle `up` links depended on remote traffic) | MEDIUM | **FIXED** | `dial_upstream` select loop gained the same 20 s heartbeat arm as `serve_ws` |
| Untested MUSTs: timed gap flush (§2.3), multi-hop ACK via-trace (§2.5), dup-connection same-direction + their-pid-higher branches (§5.4) | TEST | **FIXED** | Tests `timed_gap_window_flushes_without_explicit_flush`, `multi_hop_ack_routed_to_requester_via_dup_trace`, `duplicate_same_direction_connection_keeps_newer_link`, `duplicate_connection_their_pid_higher_drops_our_outbound` |

## Accepted / documented (merge-gate)

- **`@`+`##` dedup deviates from the parity-spec letter** (recomputes the hash locally instead of trusting `##`): intentional anti-poisoning hardening, recorded as a spec-deviation note in `gunmetal-parity.md` §2.2.
- **Same-direction duplicate links resolve by local arrival order**, which can differ between the two ends under racing handshakes (each end may drop a different link, costing one reconnect). Matches GUN's practical behavior; revisit if relay-to-relay meshes grow.
- **Mesh⇄Gun reference cycle + heartbeat task have no teardown path** (`Mesh::new` registers a listener whose id is discarded; `start_heartbeat` loops forever). Irrelevant for the one-relay-per-process and one-instance-per-page (wasm) lifecycles shipped today; add `Mesh::close()` when embedders need teardown.
- **EventBus holds its lock while listeners run** — superseded (see the web-catalog review addendum): emission is now snapshot-and-release; listeners may freely subscribe, unsubscribe, and write back.

---

# Web-Catalog Review Addendum

**Date:** 2026-06-12
**Scope:** Crate changes on the `gunmetal-web-catalog` branch (wasm networking/persistence/taps, EventBus snapshot-and-release, mesh duplicate-connection change), reviewed by a 3-agent final adversarial pass before merge.

| Finding | Severity | Status | Fix Applied |
|---------|----------|--------|-------------|
| Snapshot-and-release `try_lock` silently dropped events under CROSS-THREAD contention (relay persists via a listener -> silent data loss; empirically ~50% loss with 2 threads + slow listener) | CRITICAL | **FIXED** | Re-entrancy-only skip via owner `ThreadId`; other threads take a blocking lock. Panic-safe owner reset. Regression test `concurrent_emits_never_drop_events` |
| wasm `enablePersistence` built IndexedDB row keys by raw ESC concatenation, bypassing the M6 `storage_key()` validation (remote soul/key containing ESC -> cross-soul write spoofing after hydration) | HIGH | **FIXED** | Persist and hydrate paths share the native `storage_key`/`parse_storage_key` validation with an exact round-trip check |
| `fire_wire_tap`/`fire_status` held the handler RefCell borrow across the user JS callback (callback re-registering itself -> BorrowMutError abort) | MEDIUM | **FIXED** | Functions are cloned out of the borrow before invocation |
| `enablePersistence` double-call registered duplicate listeners + IDB handles | LOW | **FIXED** | Idempotent guard (retry allowed after a failed open) |
| `off()` no longer barriers against in-flight listener execution; snapshots hold strong refs | DOC | **DOCUMENTED** | Caveats added to `off()` and the `SharedListener` docs |
| Same-direction duplicate resolution (newer wins) + a CONFIGURED static pid = connection-eviction primitive for anyone knowing the pid | DOC | **DOCUMENTED** | Caveat in the resolution comment; default random pid unaffected |
| Writes/`registerBye` between `connect()` and socket open target zero peers (not queued) | DOC | **DOCUMENTED** | Caveat on `connect()`: wait for `onStatus("open")`/`peerPid` |
