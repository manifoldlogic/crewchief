//! User — decentralized authentication via SEA.
//!
//! Implements `gun.user()` with create/auth/leave for decentralized
//! identity management. Users are identified by their public key and
//! authenticated via password-derived proof of work.
//!
//! ## Data Model
//!
//! - User node stored at `~<pubKey>` with: `{pub, epub, alias, auth}`
//! - `auth` contains `{ek: encrypted_private_keys, s: salt}`
//! - Alias lookup at `~@<alias>` → link to `~<pubKey>`
//! - User data lives under the `~<pubKey>` namespace

use crate::instance::Gun;
use crate::sea::{self, SEAPair};
use crate::types::GunValue;

/// Minimum password length (matches GUN's check).
const MIN_PASSWORD_LEN: usize = 8;

/// Authentication state for a logged-in user.
#[derive(Clone)]
pub struct UserAuth {
    /// The user's public signing key.
    pub pub_key: String,
    /// The user's public encryption key.
    pub epub: String,
    /// The user's alias (username).
    pub alias: String,
    /// The full key pair (available only while authenticated).
    pub pair: SEAPair,
}

// Custom Debug that redacts the key pair to prevent private key leakage.
impl std::fmt::Debug for UserAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserAuth")
            .field("pub_key", &self.pub_key)
            .field("epub", &self.epub)
            .field("alias", &self.alias)
            .field("pair", &"[REDACTED]")
            .finish()
    }
}

/// Result of a user.create() operation.
#[derive(Debug)]
pub enum CreateResult {
    /// User created successfully.
    Ok { pub_key: String },
    /// Error during creation.
    Err { err: String },
}

/// Result of a user.auth() operation.
#[derive(Debug)]
pub enum AuthResult {
    /// Authentication successful.
    Ok(UserAuth),
    /// Authentication failed.
    Err { err: String },
}

/// Encrypted auth data stored in the graph.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AuthData {
    /// Encrypted private keys (SEA encrypted string).
    ek: String,
    /// Salt used for PBKDF2.
    s: String,
}

/// A user instance tied to a Gun database.
///
/// Equivalent to `gun.user()` in JavaScript.
pub struct User {
    gun: Gun,
    auth: Option<UserAuth>,
}

impl User {
    /// Create a new User instance from a Gun database.
    ///
    /// The user starts unauthenticated. Call `create()` or `auth()` to log in.
    pub fn new(gun: Gun) -> Self {
        Self { gun, auth: None }
    }

    /// Check if a user is currently authenticated.
    ///
    /// Equivalent to `user.is` in JavaScript.
    pub fn is_authenticated(&self) -> Option<&UserAuth> {
        self.auth.as_ref()
    }

    /// Create a new user account.
    ///
    /// Generates a key pair, encrypts private keys with a password-derived
    /// proof, and stores the user data in the graph.
    ///
    /// Equivalent to `user.create(alias, password, cb)`.
    pub fn create(&mut self, alias: &str, password: &str) -> CreateResult {
        // Validate inputs
        if alias.is_empty() {
            return CreateResult::Err {
                err: "No user.".into(),
            };
        }
        // M17: use char count, not byte count, to match JS string length behavior
        if password.chars().count() < MIN_PASSWORD_LEN {
            return CreateResult::Err {
                err: "Password too short!".into(),
            };
        }

        // Check if alias already exists
        let alias_key = format!("~@{}", alias);
        if self.gun.get(&alias_key).node_data().is_some() {
            return CreateResult::Err {
                err: "User already created!".into(),
            };
        }

        // Generate key pair
        let pair = match sea::pair() {
            Ok(p) => p,
            Err(e) => {
                return CreateResult::Err {
                    err: format!("Key generation failed: {}", e),
                }
            }
        };

        // Generate salt and derive proof via PBKDF2
        let salt = random_string(64);
        let proof = match sea::work(password, Some(&salt)) {
            Ok(p) => p,
            Err(e) => {
                return CreateResult::Err {
                    err: format!("Work failed: {}", e),
                }
            }
        };

        // Encrypt private keys with proof
        let priv_data = serde_json::json!({
            "priv": pair.priv_key,
            "epriv": pair.epriv
        });
        let encrypted = match sea::encrypt(&priv_data, &proof) {
            Ok(e) => e,
            Err(e) => {
                return CreateResult::Err {
                    err: format!("Encrypt failed: {}", e),
                }
            }
        };

        // Build auth data
        let auth_data = AuthData {
            ek: encrypted,
            s: salt,
        };
        let auth_json = serde_json::to_string(&auth_data).unwrap();

        // Store user node at ~pubKey
        let user_soul = format!("~{}", pair.pub_key);
        self.gun.get(&user_soul).put(vec![
            ("pub".into(), GunValue::Text(pair.pub_key.clone())),
            ("epub".into(), GunValue::Text(pair.epub.clone())),
            ("alias".into(), GunValue::Text(alias.to_string())),
            ("auth".into(), GunValue::Text(auth_json)),
        ]);

        // Create alias lookup: ~@alias -> link to ~pubKey
        self.gun
            .get(&alias_key)
            .put_kv(&user_soul, GunValue::Link(user_soul.clone()));

        // Auto-login
        self.auth = Some(UserAuth {
            pub_key: pair.pub_key.clone(),
            epub: pair.epub.clone(),
            alias: alias.to_string(),
            pair,
        });

        CreateResult::Ok {
            pub_key: self.auth.as_ref().unwrap().pub_key.clone(),
        }
    }

    /// Authenticate an existing user by alias and password.
    ///
    /// Looks up the user by alias, derives the proof from the password,
    /// decrypts the private keys, and sets the authentication state.
    ///
    /// Equivalent to `user.auth(alias, password, cb)`.
    pub fn auth_with_password(&mut self, alias: &str, password: &str) -> AuthResult {
        // Look up alias → ~pubKey
        let alias_key = format!("~@{}", alias);
        let alias_data = match self.gun.get(&alias_key).node_data() {
            Some(data) => data,
            None => {
                return AuthResult::Err {
                    err: "Wrong user or password.".into(),
                }
            }
        };

        // Find the ~pubKey link
        let user_soul = alias_data
            .iter()
            .find_map(|(_, v)| match v {
                GunValue::Link(soul) if soul.starts_with('~') => Some(soul.clone()),
                _ => None,
            });

        let user_soul = match user_soul {
            Some(s) => s,
            None => {
                return AuthResult::Err {
                    err: "Wrong user or password.".into(),
                }
            }
        };

        self.auth_from_soul(&user_soul, alias, password)
    }

    /// Authenticate using a key pair directly.
    ///
    /// Validates that the private keys correspond to the claimed public keys
    /// before accepting the pair. Equivalent to `user.auth(pair, cb)`.
    pub fn auth_with_pair(&mut self, pair: SEAPair) -> AuthResult {
        if pair.pub_key.is_empty() || pair.epub.is_empty()
            || pair.priv_key.is_empty() || pair.epriv.is_empty()
        {
            return AuthResult::Err {
                err: "Invalid key pair.".into(),
            };
        }

        // Validate that private keys match public keys by signing and verifying
        let test_data = serde_json::json!("gunmetal_key_validation");
        match sea::sign(&test_data, &pair.priv_key, &pair.pub_key) {
            Ok(signed) => {
                if sea::verify(&signed, &pair.pub_key).is_err() {
                    return AuthResult::Err {
                        err: "Invalid key pair.".into(),
                    };
                }
            }
            Err(_) => {
                return AuthResult::Err {
                    err: "Invalid key pair.".into(),
                };
            }
        }

        // H7: Look up user data — warn if user doesn't exist in graph
        let user_soul = format!("~{}", pair.pub_key);
        let user_exists = self.gun.get(&user_soul).get("pub").val().is_some();
        let alias = if user_exists {
            self.gun
                .get(&user_soul)
                .get("alias")
                .val()
                .and_then(|v| match v {
                    GunValue::Text(s) => Some(s),
                    _ => None,
                })
                .unwrap_or_else(|| pair.pub_key.clone())
        } else {
            // User node doesn't exist — this is a "phantom" auth.
            // Still allow it (matching GUN JS behavior) but use pub as alias.
            pair.pub_key.clone()
        };

        self.auth = Some(UserAuth {
            pub_key: pair.pub_key.clone(),
            epub: pair.epub.clone(),
            alias,
            pair,
        });

        AuthResult::Ok(self.auth.clone().unwrap())
    }

    /// Log out the current user.
    ///
    /// Clears authentication state. Equivalent to `user.leave()`.
    pub fn leave(&mut self) {
        self.auth = None;
    }

    /// Get a chain scoped to the current user's graph.
    ///
    /// Returns None if not authenticated.
    /// Writes here go to `~<pubKey>/<key>`.
    pub fn get(&self, key: &str) -> Option<crate::instance::GunChain> {
        self.auth.as_ref().map(|auth| {
            let user_soul = format!("~{}", auth.pub_key);
            self.gun.get(user_soul).get(key)
        })
    }

    /// Get a signed chain scoped to the current user's graph.
    ///
    /// Returns None if not authenticated.
    /// All writes through this chain are automatically signed with
    /// the user's private key. Reads verify and unwrap signatures.
    pub fn get_signed(&self, key: &str) -> Option<SignedChain> {
        self.auth.as_ref().map(|auth| {
            let user_soul = format!("~{}", auth.pub_key);
            let chain = self.gun.get(&user_soul).get(key);
            SignedChain {
                chain,
                priv_key: auth.pair.priv_key.clone(),
                pub_key: auth.pair.pub_key.clone(),
            }
        })
    }

    /// Get the underlying Gun instance.
    pub fn gun(&self) -> &Gun {
        &self.gun
    }

    // ── Internal helpers ────────────────────────────────────────────

    fn auth_from_soul(&mut self, user_soul: &str, alias: &str, password: &str) -> AuthResult {
        // Get user node data
        let user_data = match self.gun.get(user_soul).node_data() {
            Some(data) => data,
            None => {
                return AuthResult::Err {
                    err: "Wrong user or password.".into(),
                }
            }
        };

        // Extract fields
        let mut pub_key = None;
        let mut epub = None;
        let mut auth_json = None;

        for (k, v) in &user_data {
            match (k.as_str(), v) {
                ("pub", GunValue::Text(s)) => pub_key = Some(s.clone()),
                ("epub", GunValue::Text(s)) => epub = Some(s.clone()),
                ("auth", GunValue::Text(s)) => auth_json = Some(s.clone()),
                _ => {}
            }
        }

        let pub_key = match pub_key {
            Some(p) => p,
            None => {
                return AuthResult::Err {
                    err: "Wrong user or password.".into(),
                }
            }
        };
        let epub = match epub {
            Some(e) => e,
            None => {
                return AuthResult::Err {
                    err: "Wrong user or password.".into(),
                }
            }
        };
        let auth_str = match auth_json {
            Some(a) => a,
            None => {
                return AuthResult::Err {
                    err: "Wrong user or password.".into(),
                }
            }
        };

        // Parse auth data
        let auth_data: AuthData = match serde_json::from_str(&auth_str) {
            Ok(a) => a,
            Err(_) => {
                return AuthResult::Err {
                    err: "Wrong user or password.".into(),
                }
            }
        };

        // Derive proof from password + salt
        let proof = match sea::work(password, Some(&auth_data.s)) {
            Ok(p) => p,
            Err(_) => {
                return AuthResult::Err {
                    err: "Wrong user or password.".into(),
                }
            }
        };

        // Decrypt private keys
        let decrypted = match sea::decrypt(&auth_data.ek, &proof) {
            Ok(d) => d,
            Err(_) => {
                return AuthResult::Err {
                    err: "Wrong user or password.".into(),
                }
            }
        };

        // Extract priv and epriv
        let priv_key = decrypted
            .get("priv")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let epriv = decrypted
            .get("epriv")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let (priv_key, epriv) = match (priv_key, epriv) {
            (Some(p), Some(e)) => (p, e),
            _ => {
                return AuthResult::Err {
                    err: "Wrong user or password.".into(),
                }
            }
        };

        let pair = SEAPair {
            pub_key: pub_key.clone(),
            priv_key,
            epub: epub.clone(),
            epriv,
        };

        self.auth = Some(UserAuth {
            pub_key,
            epub,
            alias: alias.to_string(),
            pair,
        });

        AuthResult::Ok(self.auth.clone().unwrap())
    }
}

/// Metadata keys that are never signed (stored in plaintext).
const METADATA_KEYS: &[&str] = &["pub", "epub", "alias", "auth"];

/// A chain wrapper that auto-signs writes and verifies reads.
///
/// Returned by `User::get_signed()`. Wraps a `GunChain` and signs all
/// values with the user's private key before storing them as
/// `GunValue::Text("SEA{m:...,s:...}")`.
///
/// Metadata keys (`pub`, `epub`, `alias`, `auth`) are stored unsigned.
pub struct SignedChain {
    chain: crate::instance::GunChain,
    priv_key: String,
    pub_key: String,
}

impl SignedChain {
    /// Write a signed value at the current chain position.
    ///
    /// The value is signed with the user's private key and stored as
    /// `GunValue::Text("SEA{...}")`. Metadata keys are stored unsigned.
    pub fn put_value(&self, value: GunValue) -> &Self {
        let key = self.chain.key().unwrap_or("");

        // Skip signing for metadata keys
        if METADATA_KEYS.contains(&key) {
            self.chain.put_value(value);
            return self;
        }

        // Sign the value
        match sea::sign_value(&value, &self.priv_key, &self.pub_key) {
            Ok(signed) => {
                self.chain.put_value(GunValue::Text(signed));
            }
            Err(_) => {
                // If signing fails, store unsigned (matches GUN JS behavior)
                self.chain.put_value(value);
            }
        }
        self
    }

    /// Read and verify the value at the current chain position.
    ///
    /// If the value is a `SEA{...}` signed string, verifies the signature
    /// and returns the unwrapped value. Returns None if verification fails
    /// or if the value doesn't exist.
    pub fn val(&self) -> Option<GunValue> {
        let raw = self.chain.val()?;

        match &raw {
            GunValue::Text(text) if text.starts_with("SEA{") => {
                // Try to verify and unwrap
                // None on verification failure
                sea::verify_signed_value(text, &self.pub_key).ok()
            }
            _ => Some(raw), // not signed, return as-is (metadata keys)
        }
    }

    /// Get the underlying chain for direct (unsigned) access.
    pub fn chain(&self) -> &crate::instance::GunChain {
        &self.chain
    }

    /// Get the soul this chain is scoped to.
    pub fn soul(&self) -> &str {
        self.chain.soul()
    }

    /// Get the key this chain is scoped to (if any).
    pub fn key(&self) -> Option<&str> {
        self.chain.key()
    }
}

/// Generate a random alphanumeric string (matches GUN's String.random).
/// Uses OsRng for consistency with SEA crypto operations (M16 fix).
fn random_string(len: usize) -> String {
    use rand::Rng;
    use rand::rngs::OsRng;
    // Note: charset matches GUN JS exactly (missing 'Y' is intentional for JS compat)
    const CHARSET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXZabcdefghijklmnopqrstuvwxyz";
    let mut rng = OsRng;
    (0..len)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::GunOptions;

    fn new_gun() -> Gun {
        Gun::new(GunOptions::default())
    }

    #[test]
    fn create_user() {
        let gun = new_gun();
        let mut user = User::new(gun.clone());

        match user.create("alice", "password123") {
            CreateResult::Ok { pub_key } => {
                assert!(!pub_key.is_empty());
                assert!(pub_key.contains('.'));
            }
            CreateResult::Err { err } => panic!("Failed to create user: {}", err),
        }
    }

    #[test]
    fn create_user_auto_login() {
        let gun = new_gun();
        let mut user = User::new(gun);

        user.create("bob", "password123");
        let auth = user.is_authenticated().expect("should be auto-logged in");
        assert_eq!(auth.alias, "bob");
        assert!(!auth.pub_key.is_empty());
    }

    #[test]
    fn create_user_stores_in_graph() {
        let gun = new_gun();
        let mut user = User::new(gun.clone());

        let result = user.create("charlie", "password123");
        let pub_key = match result {
            CreateResult::Ok { pub_key } => pub_key,
            _ => panic!("Failed"),
        };

        let user_soul = format!("~{}", pub_key);
        assert_eq!(
            gun.get(&user_soul).get("alias").val(),
            Some(GunValue::Text("charlie".into()))
        );
        assert_eq!(
            gun.get(&user_soul).get("pub").val(),
            Some(GunValue::Text(pub_key.clone()))
        );
        assert!(gun.get(&user_soul).get("auth").val().is_some());
    }

    #[test]
    fn create_user_alias_lookup() {
        let gun = new_gun();
        let mut user = User::new(gun.clone());

        let result = user.create("dave", "password123");
        let pub_key = match result {
            CreateResult::Ok { pub_key } => pub_key,
            _ => panic!("Failed"),
        };

        // Alias lookup should have a link to the user soul
        let alias_data = gun.get("~@dave").node_data().expect("alias should exist");
        let user_soul = format!("~{}", pub_key);
        assert!(alias_data
            .iter()
            .any(|(_, v)| *v == GunValue::Link(user_soul.clone())));
    }

    #[test]
    fn create_duplicate_alias_fails() {
        let gun = new_gun();
        let mut user = User::new(gun);

        user.create("eve", "password123");
        user.leave();

        match user.create("eve", "password456") {
            CreateResult::Err { err } => {
                assert!(err.contains("already created"));
            }
            _ => panic!("Should have failed for duplicate alias"),
        }
    }

    #[test]
    fn create_short_password_fails() {
        let gun = new_gun();
        let mut user = User::new(gun);

        match user.create("frank", "short") {
            CreateResult::Err { err } => {
                assert!(err.contains("too short"));
            }
            _ => panic!("Should have failed for short password"),
        }
    }

    #[test]
    fn create_empty_alias_fails() {
        let gun = new_gun();
        let mut user = User::new(gun);

        match user.create("", "password123") {
            CreateResult::Err { err } => {
                assert!(err.contains("No user"));
            }
            _ => panic!("Should have failed for empty alias"),
        }
    }

    #[test]
    fn auth_with_password() {
        let gun = new_gun();
        let mut user = User::new(gun.clone());

        // Create user
        user.create("grace", "password123");
        let pub_key = user.is_authenticated().unwrap().pub_key.clone();
        user.leave();

        // Re-authenticate
        let mut user2 = User::new(gun);
        match user2.auth_with_password("grace", "password123") {
            AuthResult::Ok(auth) => {
                assert_eq!(auth.pub_key, pub_key);
                assert_eq!(auth.alias, "grace");
                assert!(!auth.pair.priv_key.is_empty());
                assert!(!auth.pair.epriv.is_empty());
            }
            AuthResult::Err { err } => panic!("Auth failed: {}", err),
        }
    }

    #[test]
    fn auth_wrong_password_fails() {
        let gun = new_gun();
        let mut user = User::new(gun.clone());

        user.create("hank", "correct_password");
        user.leave();

        let mut user2 = User::new(gun);
        match user2.auth_with_password("hank", "wrong_password") {
            AuthResult::Err { err } => {
                assert!(err.contains("Wrong user or password"));
            }
            _ => panic!("Should have failed with wrong password"),
        }
    }

    #[test]
    fn auth_nonexistent_user_fails() {
        let gun = new_gun();
        let mut user = User::new(gun);

        match user.auth_with_password("nobody", "password123") {
            AuthResult::Err { err } => {
                assert!(err.contains("Wrong user or password"));
            }
            _ => panic!("Should have failed"),
        }
    }

    #[test]
    fn auth_with_pair() {
        let gun = new_gun();
        let mut user = User::new(gun.clone());

        user.create("irene", "password123");
        let pair = user.is_authenticated().unwrap().pair.clone();
        user.leave();

        let mut user2 = User::new(gun);
        match user2.auth_with_pair(pair.clone()) {
            AuthResult::Ok(auth) => {
                assert_eq!(auth.pub_key, pair.pub_key);
            }
            AuthResult::Err { err } => panic!("Auth failed: {}", err),
        }
    }

    #[test]
    fn leave_clears_auth() {
        let gun = new_gun();
        let mut user = User::new(gun);

        user.create("jack", "password123");
        assert!(user.is_authenticated().is_some());

        user.leave();
        assert!(user.is_authenticated().is_none());
    }

    #[test]
    fn user_get_writes_to_namespace() {
        let gun = new_gun();
        let mut user = User::new(gun.clone());

        user.create("karen", "password123");
        let pub_key = user.is_authenticated().unwrap().pub_key.clone();

        // Write to user namespace
        user.get("profile")
            .unwrap()
            .put_value(GunValue::Text("hello".into()));

        // Should be stored under ~pubKey/profile
        let user_soul = format!("~{}", pub_key);
        assert_eq!(
            gun.get(&user_soul).get("profile").val(),
            Some(GunValue::Text("hello".into()))
        );
    }

    #[test]
    fn user_get_returns_none_when_not_authed() {
        let gun = new_gun();
        let user = User::new(gun);
        assert!(user.get("anything").is_none());
    }

    #[test]
    fn full_lifecycle() {
        let gun = new_gun();
        let mut user = User::new(gun.clone());

        // Create
        user.create("lifecycle_user", "securepass1");
        let pub_key = user.is_authenticated().unwrap().pub_key.clone();

        // Write user data
        user.get("settings")
            .unwrap()
            .put_value(GunValue::Text("dark_mode".into()));

        // Leave
        user.leave();
        assert!(user.is_authenticated().is_none());

        // Re-auth
        let mut user2 = User::new(gun.clone());
        user2.auth_with_password("lifecycle_user", "securepass1");
        assert!(user2.is_authenticated().is_some());
        assert_eq!(user2.is_authenticated().unwrap().pub_key, pub_key);

        // Read user data back
        let user_soul = format!("~{}", pub_key);
        assert_eq!(
            gun.get(&user_soul).get("settings").val(),
            Some(GunValue::Text("dark_mode".into()))
        );
    }

    // ── SignedChain tests ───────────────────────────────────────────

    #[test]
    fn signed_chain_put_and_val() {
        let gun = new_gun();
        let mut user = User::new(gun.clone());
        user.create("signed_alice", "password123");
        let pub_key = user.is_authenticated().unwrap().pub_key.clone();

        // Write through signed chain
        let signed = user.get_signed("profile").unwrap();
        signed.put_value(GunValue::Text("hello signed".into()));

        // Read through signed chain — should verify and unwrap
        let val = signed.val();
        assert_eq!(val, Some(GunValue::Text("hello signed".into())));

        // Raw value in graph should be a SEA{...} string
        let user_soul = format!("~{}", pub_key);
        let raw = gun.get(&user_soul).get("profile").val().unwrap();
        match raw {
            GunValue::Text(s) => assert!(s.starts_with("SEA{"), "expected SEA prefix, got: {}", s),
            _ => panic!("expected Text, got {:?}", raw),
        }
    }

    #[test]
    fn signed_chain_returns_none_for_tampered_value() {
        let gun = new_gun();
        let mut user = User::new(gun.clone());
        user.create("signed_bob", "password123");
        let pub_key = user.is_authenticated().unwrap().pub_key.clone();

        // Write signed value
        let signed = user.get_signed("secret").unwrap();
        signed.put_value(GunValue::Text("original".into()));

        // Tamper with the stored value directly (bypass signing)
        let user_soul = format!("~{}", pub_key);
        gun.get(&user_soul)
            .get("secret")
            .put_value(GunValue::Text("SEA{\"m\":\"tampered\",\"s\":\"invalid\"}".into()));

        // Reading through signed chain should fail verification
        let val = signed.val();
        assert!(val.is_none(), "tampered value should not verify");
    }

    #[test]
    fn get_signed_returns_none_when_not_authed() {
        let gun = new_gun();
        let user = User::new(gun);
        assert!(user.get_signed("anything").is_none());
    }

    #[test]
    fn signed_chain_metadata_keys_stored_unsigned() {
        let gun = new_gun();
        let mut user = User::new(gun.clone());
        user.create("signed_charlie", "password123");
        let pub_key = user.is_authenticated().unwrap().pub_key.clone();

        // The create() method stores metadata keys (pub, epub, alias, auth) unsigned
        let user_soul = format!("~{}", pub_key);
        let pub_val = gun.get(&user_soul).get("pub").val().unwrap();

        // Metadata should NOT be SEA-wrapped
        match pub_val {
            GunValue::Text(s) => {
                assert!(!s.starts_with("SEA{"), "metadata should not be signed");
            }
            _ => panic!("expected Text"),
        }
    }
}
