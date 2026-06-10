//! WASM bindings for gunmetal via wasm-bindgen.
//!
//! Exposes the complete GUN API to JavaScript: data operations, SEA crypto,
//! and User authentication.
//!
//! # Usage from JavaScript
//!
//! ```js
//! import init, { WasmGun, WasmSEA, WasmUser } from './gunmetal.js';
//!
//! await init();
//! const gun = new WasmGun();
//! const sea = new WasmSEA();
//! const user = new WasmUser(gun);
//!
//! // Create account & login
//! const result = user.create("alice", "password123");
//! console.log("Created:", result);
//!
//! // Write to user namespace
//! user.put("profile", JSON.stringify("Alice"));
//!
//! // SEA crypto
//! const pair = sea.pair();
//! const signed = sea.sign(JSON.stringify("hello"), pair.priv, pair.pub);
//! const verified = sea.verify(signed, pair.pub);
//! ```

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::cert::{CertWhat, CertWho, Certificate};
#[cfg(target_arch = "wasm32")]
use crate::events::ListenerId;
#[cfg(target_arch = "wasm32")]
use crate::instance::{Gun, GunOptions};
#[cfg(target_arch = "wasm32")]
use crate::sea;
#[cfg(target_arch = "wasm32")]
use crate::types::GunValue;
#[cfg(target_arch = "wasm32")]
use crate::user::{AuthResult, CreateResult, User};
#[cfg(target_arch = "wasm32")]
use crate::wire;

// ═══════════════════════════════════════════════════════════════════════
// WasmGun — Core database operations
// ═══════════════════════════════════════════════════════════════════════

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmGun {
    inner: Gun,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmGun {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Gun::new(GunOptions::default()),
        }
    }

    // ── Write operations ────────────────────────────────────────────

    #[wasm_bindgen(js_name = "putText")]
    pub fn put_text(&self, soul: &str, key: &str, value: &str) {
        self.inner
            .get(soul)
            .put_kv(key, GunValue::Text(value.to_string()));
    }

    #[wasm_bindgen(js_name = "putNumber")]
    pub fn put_number(&self, soul: &str, key: &str, value: f64) {
        if let Some(v) = GunValue::number(value) {
            self.inner.get(soul).put_kv(key, v);
        }
    }

    #[wasm_bindgen(js_name = "putBool")]
    pub fn put_bool(&self, soul: &str, key: &str, value: bool) {
        self.inner.get(soul).put_kv(key, GunValue::Bool(value));
    }

    #[wasm_bindgen(js_name = "putNull")]
    pub fn put_null(&self, soul: &str, key: &str) {
        self.inner.get(soul).put_kv(key, GunValue::Null);
    }

    #[wasm_bindgen(js_name = "putLink")]
    pub fn put_link(&self, soul: &str, key: &str, target_soul: &str) {
        self.inner
            .get(soul)
            .put_kv(key, GunValue::Link(target_soul.to_string()));
    }

    /// Write a JSON object. Each key becomes a property on the node.
    #[wasm_bindgen(js_name = "putObject")]
    pub fn put_object(&self, soul: &str, json: &str) -> Result<(), JsValue> {
        let parsed: serde_json::Value =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;

        if let Some(obj) = parsed.as_object() {
            let data: Vec<(String, GunValue)> = obj
                .iter()
                .filter_map(|(k, v)| wire::json_to_value(v).map(|gv| (k.clone(), gv)))
                .collect();
            self.inner.get(soul).put(data);
            Ok(())
        } else {
            Err(JsValue::from_str("Expected a JSON object"))
        }
    }

    // ── Read operations ─────────────────────────────────────────────

    /// Read a value. Returns JSON string or null.
    #[wasm_bindgen(js_name = "get")]
    pub fn get(&self, soul: &str, key: &str) -> JsValue {
        match self.inner.get(soul).get(key).val() {
            Some(val) => {
                let json = wire::value_to_json(&val);
                JsValue::from_str(&json.to_string())
            }
            None => JsValue::NULL,
        }
    }

    /// Read a full node as JSON.
    #[wasm_bindgen(js_name = "getNode")]
    pub fn get_node(&self, soul: &str) -> JsValue {
        match self.inner.get(soul).node_data() {
            Some(data) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in data {
                    obj.insert(k, wire::value_to_json(&v));
                }
                JsValue::from_str(&serde_json::Value::Object(obj).to_string())
            }
            None => JsValue::NULL,
        }
    }

    // ── Subscriptions ───────────────────────────────────────────────

    /// Subscribe to a key. Callback: `(jsonValue: string, key: string)`.
    #[wasm_bindgen(js_name = "on")]
    pub fn on(&self, soul: &str, key: &str, callback: js_sys::Function) -> u32 {
        let id = self.inner.get(soul).get(key).on(move |val, k| {
            let json_val = wire::value_to_json(&val).to_string();
            let _ = callback.call2(
                &JsValue::NULL,
                &JsValue::from_str(&json_val),
                &JsValue::from_str(&k),
            );
        });
        id.0 as u32
    }

    /// Subscribe to all changes on a node.
    #[wasm_bindgen(js_name = "onNode")]
    pub fn on_node(&self, soul: &str, callback: js_sys::Function) -> u32 {
        let id = self.inner.get(soul).on(move |val, k| {
            let json_val = wire::value_to_json(&val).to_string();
            let _ = callback.call2(
                &JsValue::NULL,
                &JsValue::from_str(&json_val),
                &JsValue::from_str(&k),
            );
        });
        id.0 as u32
    }

    #[wasm_bindgen(js_name = "off")]
    pub fn off(&self, soul: &str, key: &str, listener_id: u32) {
        self.inner
            .get(soul)
            .get(key)
            .off(ListenerId(listener_id as u64));
    }

    // ── Sync / Wire ─────────────────────────────────────────────────

    #[wasm_bindgen(js_name = "state")]
    pub fn state(&self) -> f64 {
        self.inner.state()
    }

    /// Process incoming wire message from a peer.
    #[wasm_bindgen(js_name = "receive")]
    pub fn receive(&self, json: &str) -> Result<(), JsValue> {
        let msg =
            wire::parse_message(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.inner.receive(&msg);
        Ok(())
    }

    /// Serialize a node for sending to peers.
    #[wasm_bindgen(js_name = "outgoing")]
    pub fn outgoing(&self, soul: &str) -> JsValue {
        self.inner.graph(|graph| match graph.get_node(soul) {
            Some(node) => {
                let msg_id = format!("gm{}", self.inner.state() as u64);
                let msg = wire::put_message(&msg_id, &[node]);
                match wire::serialize_message(&msg) {
                    Ok(json) => JsValue::from_str(&json),
                    Err(_) => JsValue::NULL,
                }
            }
            None => JsValue::NULL,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WasmGun — Extended chain API (gun/lib/* equivalents)
// ═══════════════════════════════════════════════════════════════════════

#[cfg(all(target_arch = "wasm32", feature = "extended-api"))]
#[wasm_bindgen]
impl WasmGun {
    /// Promise-based read (`gun/lib/then.js`): resolves with the JSON value
    /// at `soul.key` (or `null` if missing), using `.once()` semantics.
    #[wasm_bindgen(js_name = "then")]
    pub fn then_promise(&self, soul: &str, key: &str) -> js_sys::Promise {
        let chain = self.inner.get(soul).get(key);
        wasm_bindgen_futures::future_to_promise(async move {
            Ok(match chain.then().await {
                Some(val) => JsValue::from_str(&wire::value_to_json(&val).to_string()),
                None => JsValue::NULL,
            })
        })
    }

    /// One-shot deep document load (`gun/lib/load.js`): fires `callback`
    /// once with the full document tree as a JSON string.
    #[wasm_bindgen(js_name = "load")]
    pub fn load(&self, soul: &str, callback: js_sys::Function) {
        self.inner
            .get(soul)
            .load(crate::extended::OpenOptions::default(), move |doc| {
                let _ = callback.call1(&JsValue::NULL, &JsValue::from_str(&doc.to_string()));
            });
    }

    /// Dot-notation path read (`gun/lib/path.js`): resolves `soul` then the
    /// dot-delimited `path`, returning the value as JSON or `null`.
    #[wasm_bindgen(js_name = "pathVal")]
    pub fn path_val(&self, soul: &str, path: &str) -> JsValue {
        match self.inner.get(soul).path(path).val() {
            Some(val) => JsValue::from_str(&wire::value_to_json(&val).to_string()),
            None => JsValue::NULL,
        }
    }

    /// Remove a node from a set (`gun/lib/unset.js`): nulls the link to
    /// `item_soul` inside the set at `set_soul`.
    #[wasm_bindgen(js_name = "unset")]
    pub fn unset(&self, set_soul: &str, item_soul: &str) {
        let item = self.inner.get(item_soul);
        self.inner.get(set_soul).unset(&item);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WasmSEA — Cryptographic operations
// ═══════════════════════════════════════════════════════════════════════

/// SEA cryptographic utilities exposed to JavaScript.
///
/// ```js
/// const sea = new WasmSEA();
/// const pair = sea.pair();
/// console.log(pair); // { pub, priv, epub, epriv }
/// ```
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmSEA;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmSEA {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self
    }

    /// Generate a new key pair.
    ///
    /// Returns JSON: `{ pub, priv, epub, epriv }`
    #[wasm_bindgen(js_name = "pair")]
    pub fn pair(&self) -> Result<JsValue, JsValue> {
        let kp = sea::pair().map_err(|e| JsValue::from_str(&e.to_string()))?;
        let json = serde_json::json!({
            "pub": kp.pub_key,
            "priv": kp.priv_key,
            "epub": kp.epub,
            "epriv": kp.epriv
        });
        Ok(JsValue::from_str(&json.to_string()))
    }

    /// Sign data with a private key.
    ///
    /// - `data`: JSON-encoded data to sign
    /// - `priv_key`: private signing key
    /// - `pub_key`: public signing key
    ///
    /// Returns: `SEA{...}` signed message string
    #[wasm_bindgen(js_name = "sign")]
    pub fn sign(
        &self,
        data: &str,
        priv_key: &str,
        pub_key: &str,
    ) -> Result<JsValue, JsValue> {
        let json_data: serde_json::Value =
            serde_json::from_str(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let signed = sea::sign(&json_data, priv_key, pub_key)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(JsValue::from_str(&signed))
    }

    /// Verify a signed message.
    ///
    /// - `message`: `SEA{...}` signed message from `sign()`
    /// - `pub_key`: public signing key
    ///
    /// Returns: the original JSON data, or throws on verification failure
    #[wasm_bindgen(js_name = "verify")]
    pub fn verify(&self, message: &str, pub_key: &str) -> Result<JsValue, JsValue> {
        let data =
            sea::verify(message, pub_key).map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(JsValue::from_str(&data.to_string()))
    }

    /// Encrypt data.
    ///
    /// - `data`: JSON-encoded data to encrypt
    /// - `key`: encryption key (epriv, shared secret, or passphrase)
    ///
    /// Returns: `SEA{...}` encrypted message string
    #[wasm_bindgen(js_name = "encrypt")]
    pub fn encrypt(&self, data: &str, key: &str) -> Result<JsValue, JsValue> {
        let json_data: serde_json::Value =
            serde_json::from_str(data).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let encrypted = sea::encrypt(&json_data, key)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(JsValue::from_str(&encrypted))
    }

    /// Decrypt data.
    ///
    /// - `message`: `SEA{...}` encrypted message from `encrypt()`
    /// - `key`: same key used to encrypt
    ///
    /// Returns: the original JSON data string
    #[wasm_bindgen(js_name = "decrypt")]
    pub fn decrypt(&self, message: &str, key: &str) -> Result<JsValue, JsValue> {
        let data =
            sea::decrypt(message, key).map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(JsValue::from_str(&data.to_string()))
    }

    /// Proof of Work / hash via PBKDF2.
    ///
    /// - `data`: data to hash
    /// - `salt`: optional salt (null for random)
    ///
    /// Returns: base64-encoded hash string
    #[wasm_bindgen(js_name = "work")]
    pub fn work(&self, data: &str, salt: Option<String>) -> Result<JsValue, JsValue> {
        let result = sea::work(data, salt.as_deref())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(JsValue::from_str(&result))
    }

    /// Derive shared secret via ECDH.
    ///
    /// - `their_epub`: other user's public encryption key
    /// - `my_epriv`: your private encryption key
    ///
    /// Returns: shared secret string (usable as encryption key)
    #[wasm_bindgen(js_name = "secret")]
    pub fn secret(&self, their_epub: &str, my_epriv: &str) -> Result<JsValue, JsValue> {
        let result = sea::secret(their_epub, my_epriv)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(JsValue::from_str(&result))
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WasmUser — Decentralized authentication
// ═══════════════════════════════════════════════════════════════════════

/// User authentication exposed to JavaScript.
///
/// ```js
/// const gun = new WasmGun();
/// const user = new WasmUser(gun);
///
/// // Create account
/// const result = JSON.parse(user.create("alice", "password123"));
/// if (result.ok !== undefined) {
///     console.log("Created! Public key:", result.pub);
/// }
///
/// // Auth
/// const auth = JSON.parse(user.auth("alice", "password123"));
///
/// // Write to user space
/// user.put("profile", '"Alice"');
///
/// // Check auth
/// const is = user.isAuthenticated(); // JSON or null
///
/// // Logout
/// user.leave();
/// ```
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmUser {
    inner: User,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmUser {
    /// Create a WasmUser tied to a WasmGun instance.
    #[wasm_bindgen(constructor)]
    pub fn new(gun: &WasmGun) -> Self {
        Self {
            inner: User::new(gun.inner.clone()),
        }
    }

    /// Create a new user account.
    ///
    /// Returns JSON: `{ ok: 0, pub: "..." }` or `{ err: "..." }`
    #[wasm_bindgen(js_name = "create")]
    pub fn create(&mut self, alias: &str, password: &str) -> JsValue {
        match self.inner.create(alias, password) {
            CreateResult::Ok { pub_key } => {
                let json = serde_json::json!({ "ok": 0, "pub": pub_key });
                JsValue::from_str(&json.to_string())
            }
            CreateResult::Err { err } => {
                let json = serde_json::json!({ "err": err });
                JsValue::from_str(&json.to_string())
            }
        }
    }

    /// Authenticate with alias and password.
    ///
    /// Returns JSON: `{ pub, epub, alias }` or `{ err: "..." }`
    #[wasm_bindgen(js_name = "auth")]
    pub fn auth(&mut self, alias: &str, password: &str) -> JsValue {
        match self.inner.auth_with_password(alias, password) {
            AuthResult::Ok(auth) => {
                let json = serde_json::json!({
                    "pub": auth.pub_key,
                    "epub": auth.epub,
                    "alias": auth.alias
                });
                JsValue::from_str(&json.to_string())
            }
            AuthResult::Err { err } => {
                let json = serde_json::json!({ "err": err });
                JsValue::from_str(&json.to_string())
            }
        }
    }

    /// Authenticate with a key pair (JSON: `{ pub, priv, epub, epriv }`).
    #[wasm_bindgen(js_name = "authPair")]
    pub fn auth_pair(&mut self, pair_json: &str) -> JsValue {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(pair_json);
        let parsed = match parsed {
            Ok(v) => v,
            Err(e) => {
                let json = serde_json::json!({ "err": e.to_string() });
                return JsValue::from_str(&json.to_string());
            }
        };

        // M15: Validate all four fields are present and non-empty
        let pub_key = parsed["pub"].as_str().unwrap_or("").to_string();
        let priv_key = parsed["priv"].as_str().unwrap_or("").to_string();
        let epub = parsed["epub"].as_str().unwrap_or("").to_string();
        let epriv = parsed["epriv"].as_str().unwrap_or("").to_string();

        if pub_key.is_empty() || priv_key.is_empty() || epub.is_empty() || epriv.is_empty() {
            let json = serde_json::json!({ "err": "Missing required key pair fields (pub, priv, epub, epriv)" });
            return JsValue::from_str(&json.to_string());
        }

        let pair = sea::SEAPair { pub_key, priv_key, epub, epriv };

        match self.inner.auth_with_pair(pair) {
            AuthResult::Ok(auth) => {
                let json = serde_json::json!({
                    "pub": auth.pub_key,
                    "epub": auth.epub,
                    "alias": auth.alias
                });
                JsValue::from_str(&json.to_string())
            }
            AuthResult::Err { err } => {
                let json = serde_json::json!({ "err": err });
                JsValue::from_str(&json.to_string())
            }
        }
    }

    /// Check if authenticated. Returns JSON `{ pub, epub, alias }` or null.
    #[wasm_bindgen(js_name = "isAuthenticated")]
    pub fn is_authenticated(&self) -> JsValue {
        match self.inner.is_authenticated() {
            Some(auth) => {
                let json = serde_json::json!({
                    "pub": auth.pub_key,
                    "epub": auth.epub,
                    "alias": auth.alias
                });
                JsValue::from_str(&json.to_string())
            }
            None => JsValue::NULL,
        }
    }

    /// Log out.
    #[wasm_bindgen(js_name = "leave")]
    pub fn leave(&mut self) {
        self.inner.leave();
    }

    /// Write to the user's namespace. Key is the property name.
    /// Value is a JSON-encoded string.
    #[wasm_bindgen(js_name = "put")]
    pub fn put(&self, key: &str, json_value: &str) -> Result<(), JsValue> {
        let chain = self
            .inner
            .get(key)
            .ok_or_else(|| JsValue::from_str("Not authenticated"))?;

        let parsed: serde_json::Value = serde_json::from_str(json_value)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        match wire::json_to_value(&parsed) {
            Some(val) => {
                chain.put_value(val);
                Ok(())
            }
            None => Err(JsValue::from_str("Invalid GUN value")),
        }
    }

    /// Read from the user's namespace.
    #[wasm_bindgen(js_name = "get")]
    pub fn get(&self, key: &str) -> JsValue {
        match self.inner.get(key) {
            Some(chain) => match chain.val() {
                Some(val) => JsValue::from_str(&wire::value_to_json(&val).to_string()),
                None => JsValue::NULL,
            },
            None => JsValue::NULL,
        }
    }

    /// Get the user's public key (or null if not authenticated).
    #[wasm_bindgen(js_name = "pubKey")]
    pub fn pub_key(&self) -> JsValue {
        match self.inner.is_authenticated() {
            Some(auth) => JsValue::from_str(&auth.pub_key),
            None => JsValue::NULL,
        }
    }

    /// Write a signed value to the user's namespace.
    ///
    /// The value is automatically signed with the user's private key.
    /// Metadata keys (pub, epub, alias, auth) are stored unsigned.
    #[wasm_bindgen(js_name = "putSigned")]
    pub fn put_signed(&self, key: &str, json_value: &str) -> Result<(), JsValue> {
        let signed_chain = self
            .inner
            .get_signed(key)
            .ok_or_else(|| JsValue::from_str("Not authenticated"))?;

        let parsed: serde_json::Value = serde_json::from_str(json_value)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        match wire::json_to_value(&parsed) {
            Some(val) => {
                signed_chain.put_value(val);
                Ok(())
            }
            None => Err(JsValue::from_str("Invalid GUN value")),
        }
    }

    /// Read and verify a signed value from the user's namespace.
    ///
    /// Returns the verified value (with signature stripped), or null if
    /// the value doesn't exist or verification fails.
    #[wasm_bindgen(js_name = "getSigned")]
    pub fn get_signed(&self, key: &str) -> JsValue {
        match self.inner.get_signed(key) {
            Some(signed_chain) => match signed_chain.val() {
                Some(val) => JsValue::from_str(&wire::value_to_json(&val).to_string()),
                None => JsValue::NULL,
            },
            None => JsValue::NULL,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WasmCert — Certificate management
// ═══════════════════════════════════════════════════════════════════════

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmCert;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmCert {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self
    }

    /// Create a certificate granting write access.
    ///
    /// - `who`: public key of grantee, or `"*"` for anyone
    /// - `what`: path (exact), `"path/*"` (prefix), or `"*"` (all)
    /// - `expiry`: ms since epoch, or null/0 for no expiry
    /// - `issuer_pub`: issuer's public signing key
    /// - `issuer_priv`: issuer's private signing key
    ///
    /// Returns JSON: `{ who, what, expiry, issuer, signature }`
    #[wasm_bindgen(js_name = "create")]
    pub fn create(
        &self,
        who: &str,
        what: &str,
        expiry: Option<f64>,
        issuer_pub: &str,
        issuer_priv: &str,
    ) -> Result<JsValue, JsValue> {
        let cert_who = if who == "*" {
            CertWho::Anyone
        } else {
            CertWho::PubKey(who.to_string())
        };

        let cert_what = if what == "*" {
            CertWhat::All
        } else if what.ends_with('*') {
            CertWhat::Prefix(what.trim_end_matches('*').to_string())
        } else {
            CertWhat::Exact(what.to_string())
        };

        let exp = expiry.filter(|&e| e > 0.0);

        let cert = Certificate::create(cert_who, cert_what, exp, issuer_pub, issuer_priv)
            .map_err(|e| JsValue::from_str(&e))?;

        let json = serde_json::json!({
            "who": match &cert.who {
                CertWho::PubKey(pk) => pk.as_str(),
                CertWho::Anyone => "*",
            },
            "what": match &cert.what {
                CertWhat::Exact(p) => p.clone(),
                CertWhat::Prefix(p) => format!("{}*", p),
                CertWhat::All => "*".to_string(),
            },
            "expiry": cert.expiry,
            "issuer": cert.issuer,
            "signature": cert.signature,
        });
        Ok(JsValue::from_str(&json.to_string()))
    }

    /// Verify a certificate's signature.
    ///
    /// Takes the JSON returned by `create()`.
    /// Returns `true` if valid, throws on error.
    #[wasm_bindgen(js_name = "verify")]
    pub fn verify(&self, cert_json: &str) -> Result<bool, JsValue> {
        let parsed: serde_json::Value = serde_json::from_str(cert_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let who_str = parsed["who"].as_str().unwrap_or("");
        let what_str = parsed["what"].as_str().unwrap_or("");
        let issuer = parsed["issuer"].as_str().unwrap_or("");
        let signature = parsed["signature"].as_str().unwrap_or("");

        let who = if who_str == "*" {
            CertWho::Anyone
        } else {
            CertWho::PubKey(who_str.to_string())
        };

        let what = if what_str == "*" {
            CertWhat::All
        } else if what_str.ends_with('*') {
            CertWhat::Prefix(what_str.trim_end_matches('*').to_string())
        } else {
            CertWhat::Exact(what_str.to_string())
        };

        let expiry = parsed["expiry"].as_f64();

        let cert = Certificate {
            who,
            what,
            expiry,
            issuer: issuer.to_string(),
            signature: signature.to_string(),
        };

        cert.verify().map_err(|e| JsValue::from_str(&e))
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WasmUUID — Time-sortable UUID generation
// ═══════════════════════════════════════════════════════════════════════

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = "generateUUID")]
pub fn wasm_generate_uuid() -> String {
    crate::uuid::generate_uuid()
}

// ═══════════════════════════════════════════════════════════════════════
// WasmEvictionConfig — Graph eviction configuration
// ═══════════════════════════════════════════════════════════════════════

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmEvictionConfig {
    max_nodes: usize,
    max_keys: usize,
    eviction_fraction: f64,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmEvictionConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(max_nodes: usize, max_keys: usize, eviction_fraction: f64) -> Self {
        Self {
            max_nodes,
            max_keys,
            eviction_fraction,
        }
    }

    #[wasm_bindgen(js_name = "default")]
    pub fn default_config() -> Self {
        let d = crate::graph::EvictionConfig::default();
        Self {
            max_nodes: d.max_nodes,
            max_keys: d.max_keys,
            eviction_fraction: d.eviction_fraction,
        }
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> JsValue {
        let json = serde_json::json!({
            "maxNodes": self.max_nodes,
            "maxKeys": self.max_keys,
            "evictionFraction": self.eviction_fraction,
        });
        JsValue::from_str(&json.to_string())
    }
}
