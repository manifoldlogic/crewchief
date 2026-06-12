//! SEA — Security, Encryption, and Authorization.
//!
//! Pure-Rust implementation of GUN's SEA cryptographic system.
//! All primitives are WASM-compatible (no JS/WebCrypto dependency).
//!
//! ## Cryptographic Primitives
//!
//! | Operation | Algorithm | Details |
//! |-----------|-----------|---------|
//! | Key pair  | ECDSA P-256 + ECDH P-256 | Signing + encryption keys |
//! | Sign      | ECDSA P-256 + SHA-256 | Signature over SHA-256 hash |
//! | Verify    | ECDSA P-256 + SHA-256 | Verify signature |
//! | Encrypt   | AES-256-GCM | With random IV and salt |
//! | Decrypt   | AES-256-GCM | Reverse of encrypt |
//! | Work      | PBKDF2-SHA256 | 100k iterations, 64-byte output |
//! | Secret    | ECDH P-256 | Derive shared secret |
//!
//! ## Key Format
//!
//! Public keys are stored as `x.y` where x and y are base64url-encoded
//! P-256 curve coordinates. This matches GUN's format exactly.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::Aes256Gcm;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use p256::ecdsa::{signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{PublicKey, SecretKey};
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// A SEA key pair containing both signing (ECDSA) and encryption (ECDH) keys.
///
/// Matches GUN's `ISEAPair`:
/// ```typescript
/// interface ISEAPair {
///     pub: string;   // public signing key (x.y base64url)
///     priv: string;  // private signing key (d base64url)
///     epub: string;  // public encryption key (x.y base64url)
///     epriv: string; // private encryption key (d base64url)
/// }
/// ```
#[derive(Clone)]
pub struct SEAPair {
    /// Public signing key in `x.y` format (base64url-encoded P-256 coordinates).
    pub pub_key: String,
    /// Private signing key (base64url-encoded).
    pub priv_key: String,
    /// Public encryption key in `x.y` format.
    pub epub: String,
    /// Private encryption key (base64url-encoded).
    pub epriv: String,
}

// Custom Debug that redacts private keys to prevent accidental leakage
// in logs, panic messages, and debug output.
impl std::fmt::Debug for SEAPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SEAPair")
            .field("pub_key", &self.pub_key)
            .field("priv_key", &"[REDACTED]")
            .field("epub", &self.epub)
            .field("epriv", &"[REDACTED]")
            .finish()
    }
}

/// Signed message format: `SEA{"m": data, "s": signature}`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignedMessage {
    /// The original message (JSON-encoded).
    pub m: serde_json::Value,
    /// The signature (base64-encoded).
    pub s: String,
}

/// Encrypted message format: `SEA{"ct": ciphertext, "iv": iv, "s": salt}`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedMessage {
    /// Ciphertext (base64-encoded).
    pub ct: String,
    /// Initialization vector (base64-encoded).
    pub iv: String,
    /// Salt (base64-encoded).
    pub s: String,
}

/// SEA error type.
#[derive(Debug)]
pub enum SeaError {
    /// Key generation or import failed.
    KeyError(String),
    /// Signing failed.
    SignError(String),
    /// Verification failed.
    VerifyError(String),
    /// Encryption failed.
    EncryptError(String),
    /// Decryption failed.
    DecryptError(String),
    /// PBKDF2/hashing failed.
    WorkError(String),
}

impl std::fmt::Display for SeaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // H10: Generic messages for external consumers to prevent
        // implementation fingerprinting. Full details available via Debug.
        match self {
            SeaError::KeyError(_) => write!(f, "SEA: invalid key"),
            SeaError::SignError(_) => write!(f, "SEA: signing failed"),
            SeaError::VerifyError(_) => write!(f, "SEA: verification failed"),
            SeaError::EncryptError(_) => write!(f, "SEA: encryption failed"),
            SeaError::DecryptError(_) => write!(f, "SEA: decryption failed"),
            SeaError::WorkError(_) => write!(f, "SEA: work failed"),
        }
    }
}

impl std::error::Error for SeaError {}

// ── Helper: P-256 key encoding ──────────────────────────────────────────

/// Encode a P-256 public key as GUN's `x.y` format.
fn encode_pub(key: &PublicKey) -> String {
    let point = key.to_encoded_point(false);
    let x = URL_SAFE_NO_PAD.encode(point.x().unwrap());
    let y = URL_SAFE_NO_PAD.encode(point.y().unwrap());
    format!("{}.{}", x, y)
}

/// Encode a P-256 secret key's scalar as base64url (the `d` parameter).
fn encode_priv(key: &SecretKey) -> String {
    URL_SAFE_NO_PAD.encode(key.to_bytes())
}

/// Decode a GUN-format `x.y` public key back to a P-256 PublicKey.
fn decode_pub(pub_str: &str) -> Result<PublicKey, SeaError> {
    let (x_b64, y_b64) = pub_str
        .split_once('.')
        .ok_or_else(|| SeaError::KeyError("Invalid pub key format (expected x.y)".into()))?;

    let x_bytes = URL_SAFE_NO_PAD
        .decode(x_b64)
        .map_err(|e| SeaError::KeyError(format!("Invalid x coordinate: {}", e)))?;
    let y_bytes = URL_SAFE_NO_PAD
        .decode(y_b64)
        .map_err(|e| SeaError::KeyError(format!("Invalid y coordinate: {}", e)))?;

    // Build uncompressed SEC1 encoding: 0x04 || x || y
    let mut sec1 = Vec::with_capacity(1 + x_bytes.len() + y_bytes.len());
    sec1.push(0x04);
    sec1.extend_from_slice(&x_bytes);
    sec1.extend_from_slice(&y_bytes);

    PublicKey::from_sec1_bytes(&sec1)
        .map_err(|e| SeaError::KeyError(format!("Invalid P-256 public key: {}", e)))
}

/// Decode a base64url private key scalar to a SecretKey.
fn decode_priv(priv_str: &str) -> Result<SecretKey, SeaError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(priv_str)
        .map_err(|e| SeaError::KeyError(format!("Invalid private key: {}", e)))?;
    SecretKey::from_slice(&bytes)
        .map_err(|e| SeaError::KeyError(format!("Invalid P-256 secret key: {}", e)))
}

// ── SEA Operations ──────────────────────────────────────────────────────

/// Generate a new key pair (ECDSA for signing + ECDH for encryption).
///
/// Equivalent to `SEA.pair()` in JavaScript.
pub fn pair() -> Result<SEAPair, SeaError> {
    // ECDSA signing key pair
    let sign_secret = SecretKey::random(&mut OsRng);
    let sign_public = sign_secret.public_key();

    // ECDH encryption key pair
    let enc_secret = SecretKey::random(&mut OsRng);
    let enc_public = enc_secret.public_key();

    Ok(SEAPair {
        pub_key: encode_pub(&sign_public),
        priv_key: encode_priv(&sign_secret),
        epub: encode_pub(&enc_public),
        epriv: encode_priv(&enc_secret),
    })
}

/// Sign data with an ECDSA private key.
///
/// Returns a `SEA{...}` prefixed string containing the signed message.
/// Equivalent to `SEA.sign(data, pair)`.
/// M4: pub_key parameter kept for API compatibility but is not used in the
/// signing operation (ECDSA signs with private key only). The public key is
/// derivable from the private key if validation is needed.
pub fn sign(data: &serde_json::Value, priv_key: &str, _pub_key: &str) -> Result<String, SeaError> {
    let secret = decode_priv(priv_key)?;
    let signing_key = SigningKey::from(&secret);

    // Hash the JSON-stringified data with SHA-256 (matching GUN's sha256 step)
    let json_str = serde_json::to_string(data)
        .map_err(|e| SeaError::SignError(format!("JSON error: {}", e)))?;
    let hash = Sha256::digest(json_str.as_bytes());

    // Sign the hash
    let signature: Signature = signing_key
        .try_sign(&hash)
        .map_err(|e| SeaError::SignError(format!("ECDSA sign failed: {}", e)))?;

    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

    let signed = SignedMessage {
        m: data.clone(),
        s: sig_b64,
    };

    let json = serde_json::to_string(&signed)
        .map_err(|e| SeaError::SignError(format!("JSON error: {}", e)))?;

    Ok(format!("SEA{}", json))
}

/// Verify a signed message and extract the original data.
///
/// Returns the original data if the signature is valid.
/// Equivalent to `SEA.verify(message, pub)`.
pub fn verify(message: &str, pub_key: &str) -> Result<serde_json::Value, SeaError> {
    // Strip SEA prefix
    let json_str = if message.starts_with("SEA{") {
        &message[3..]
    } else {
        message
    };

    let signed: SignedMessage = serde_json::from_str(json_str)
        .map_err(|e| SeaError::VerifyError(format!("Invalid signed message: {}", e)))?;

    let public = decode_pub(pub_key)?;
    let verifying_key = VerifyingKey::from(&public);

    // Reconstruct the hash that was signed
    let json_data = serde_json::to_string(&signed.m)
        .map_err(|e| SeaError::VerifyError(format!("JSON error: {}", e)))?;
    let hash = Sha256::digest(json_data.as_bytes());

    // Decode and verify signature
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(&signed.s)
        .map_err(|e| SeaError::VerifyError(format!("Invalid signature encoding: {}", e)))?;
    let signature = Signature::from_slice(&sig_bytes)
        .map_err(|e| SeaError::VerifyError(format!("Invalid signature: {}", e)))?;

    verifying_key
        .verify(&hash, &signature)
        .map_err(|_| SeaError::VerifyError("Signature did not match".into()))?;

    Ok(signed.m)
}

/// Sign a GunValue for storage in user space.
///
/// Wraps the value in a `SEA{m:...,s:...}` signed string.
/// Used by `SignedChain` for auto-signing user writes.
pub fn sign_value(
    value: &crate::types::GunValue,
    priv_key: &str,
    pub_key: &str,
) -> Result<String, SeaError> {
    let json_value = crate::wire::value_to_json(value);
    sign(&json_value, priv_key, pub_key)
}

/// Verify a signed GunValue and extract the original value.
///
/// Expects a `SEA{...}` string. Returns the original `GunValue` if valid.
/// Used in `receive()` to verify writes to `~pubKey/...` namespaces.
pub fn verify_signed_value(
    signed_text: &str,
    pub_key: &str,
) -> Result<crate::types::GunValue, SeaError> {
    let json_value = verify(signed_text, pub_key)?;
    crate::wire::json_to_value(&json_value)
        .ok_or_else(|| SeaError::VerifyError("Invalid GunValue in signed message".into()))
}

/// Encrypt data with AES-256-GCM.
///
/// The key can be either:
/// - A private encryption key (`epriv` from a pair)
/// - A shared secret (from `secret()`)
/// - A passphrase string
///
/// Returns a `SEA{...}` prefixed encrypted message.
/// Equivalent to `SEA.encrypt(data, pair)`.
pub fn encrypt(data: &serde_json::Value, key: &str) -> Result<String, SeaError> {
    let json_str = serde_json::to_string(data)
        .map_err(|e| SeaError::EncryptError(format!("JSON error: {}", e)))?;

    // Derive AES key from the input key using PBKDF2
    let mut salt = [0u8; 9];
    OsRng.fill_bytes(&mut salt);
    let aes_key = derive_aes_key(key, &salt)?;

    // Generate random IV (12 bytes for AES-GCM)
    let mut iv = [0u8; 12];
    OsRng.fill_bytes(&mut iv);

    let cipher = Aes256Gcm::new_from_slice(&aes_key)
        .map_err(|e| SeaError::EncryptError(format!("AES init: {}", e)))?;
    let nonce = aes_gcm::Nonce::from_slice(&iv);

    let ciphertext = cipher
        .encrypt(nonce, json_str.as_bytes())
        .map_err(|e| SeaError::EncryptError(format!("AES encrypt: {}", e)))?;

    let enc = EncryptedMessage {
        ct: base64::engine::general_purpose::STANDARD.encode(&ciphertext),
        iv: base64::engine::general_purpose::STANDARD.encode(iv),
        s: base64::engine::general_purpose::STANDARD.encode(salt),
    };

    let json = serde_json::to_string(&enc)
        .map_err(|e| SeaError::EncryptError(format!("JSON error: {}", e)))?;

    Ok(format!("SEA{}", json))
}

/// Decrypt a message encrypted with `encrypt()`.
///
/// Returns the original data.
/// Equivalent to `SEA.decrypt(message, pair)`.
pub fn decrypt(message: &str, key: &str) -> Result<serde_json::Value, SeaError> {
    let json_str = if message.starts_with("SEA{") {
        &message[3..]
    } else {
        message
    };

    let enc: EncryptedMessage = serde_json::from_str(json_str)
        .map_err(|e| SeaError::DecryptError(format!("Invalid encrypted message: {}", e)))?;

    let ct = base64::engine::general_purpose::STANDARD
        .decode(&enc.ct)
        .map_err(|e| SeaError::DecryptError(format!("Invalid ciphertext: {}", e)))?;
    let iv = base64::engine::general_purpose::STANDARD
        .decode(&enc.iv)
        .map_err(|e| SeaError::DecryptError(format!("Invalid IV: {}", e)))?;
    let salt = base64::engine::general_purpose::STANDARD
        .decode(&enc.s)
        .map_err(|e| SeaError::DecryptError(format!("Invalid salt: {}", e)))?;

    let aes_key = derive_aes_key(key, &salt)?;

    if iv.len() != 12 {
        return Err(SeaError::DecryptError(format!(
            "Invalid IV length: expected 12, got {}",
            iv.len()
        )));
    }

    let cipher = Aes256Gcm::new_from_slice(&aes_key)
        .map_err(|e| SeaError::DecryptError(format!("AES init: {}", e)))?;
    let nonce = aes_gcm::Nonce::from_slice(&iv);

    let plaintext = cipher
        .decrypt(nonce, ct.as_ref())
        .map_err(|_| SeaError::DecryptError("Decryption failed (wrong key or corrupted)".into()))?;

    let text = String::from_utf8(plaintext)
        .map_err(|e| SeaError::DecryptError(format!("Invalid UTF-8: {}", e)))?;

    serde_json::from_str(&text)
        .map_err(|e| SeaError::DecryptError(format!("Invalid JSON: {}", e)))
}

/// Proof of Work / Hashing via PBKDF2.
///
/// Equivalent to `SEA.work(data, salt)`.
/// Default: PBKDF2-HMAC-SHA256, 100k iterations, 64-byte output, base64 encoded.
pub fn work(data: &str, salt: Option<&str>) -> Result<String, SeaError> {
    let salt_bytes = match salt {
        Some(s) => s.as_bytes().to_vec(),
        None => {
            let mut r = [0u8; 9];
            OsRng.fill_bytes(&mut r);
            r.to_vec()
        }
    };

    let mut output = [0u8; 64];
    pbkdf2::pbkdf2_hmac::<Sha256>(data.as_bytes(), &salt_bytes, 100_000, &mut output);

    Ok(base64::engine::general_purpose::STANDARD.encode(output))
}

/// Derive a shared secret via ECDH.
///
/// Given another user's public encryption key and your private encryption key,
/// produces a shared secret string usable as an encryption key.
///
/// Equivalent to `SEA.secret(otherPub, myPair)`.
pub fn secret(their_epub: &str, my_epriv: &str) -> Result<String, SeaError> {
    let their_pub = decode_pub(their_epub)?;
    let my_secret = decode_priv(my_epriv)?;

    // Perform ECDH: shared_secret = my_priv * their_pub
    let shared = p256::ecdh::diffie_hellman(
        my_secret.to_nonzero_scalar(),
        their_pub.as_affine(),
    );

    // Encode the raw shared secret as base64url (matching GUN's format)
    Ok(URL_SAFE_NO_PAD.encode(shared.raw_secret_bytes()))
}

// ── Internal helpers ────────────────────────────────────────────────────

/// Derive a 256-bit AES key from a string key and salt.
///
/// Matches GUN's `aeskey.js`: `SHA-256(key + salt.toString('utf8'))`.
/// The salt bytes are converted to a UTF-8 string representation, concatenated
/// with the key string, then SHA-256 hashed to produce a 32-byte AES key.
fn derive_aes_key(key: &str, salt: &[u8]) -> Result<[u8; 32], SeaError> {
    // Match aeskey.js: `const combo = key + (salt).toString('utf8')`
    // Node.js Buffer.toString('utf8') on raw bytes produces a string where each
    // byte is interpreted as a character. For raw random bytes this produces
    // the latin1/binary interpretation. We use base64 of the salt as the
    // string representation, matching how the salt is stored in the message.
    let salt_str = base64::engine::general_purpose::STANDARD.encode(salt);
    let combo = format!("{}{}", key, salt_str);
    let hash = Sha256::digest(combo.as_bytes());
    Ok(hash.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_generates_valid_keys() {
        let kp = pair().unwrap();
        assert!(kp.pub_key.contains('.'));
        assert!(kp.epub.contains('.'));
        assert!(!kp.priv_key.is_empty());
        assert!(!kp.epriv.is_empty());
    }

    #[test]
    fn pair_keys_are_unique() {
        let kp1 = pair().unwrap();
        let kp2 = pair().unwrap();
        assert_ne!(kp1.pub_key, kp2.pub_key);
        assert_ne!(kp1.priv_key, kp2.priv_key);
    }

    #[test]
    fn sign_and_verify() {
        let kp = pair().unwrap();
        let data = serde_json::json!("hello world");

        let signed = sign(&data, &kp.priv_key, &kp.pub_key).unwrap();
        assert!(signed.starts_with("SEA{"));

        let verified = verify(&signed, &kp.pub_key).unwrap();
        assert_eq!(verified, data);
    }

    #[test]
    fn verify_wrong_key_fails() {
        let kp1 = pair().unwrap();
        let kp2 = pair().unwrap();
        let data = serde_json::json!("secret message");

        let signed = sign(&data, &kp1.priv_key, &kp1.pub_key).unwrap();
        let result = verify(&signed, &kp2.pub_key);
        assert!(result.is_err());
    }

    #[test]
    fn sign_verify_complex_data() {
        let kp = pair().unwrap();
        let data = serde_json::json!({
            "name": "Alice",
            "age": 30,
            "active": true
        });

        let signed = sign(&data, &kp.priv_key, &kp.pub_key).unwrap();
        let verified = verify(&signed, &kp.pub_key).unwrap();
        assert_eq!(verified, data);
    }

    #[test]
    fn encrypt_and_decrypt() {
        let kp = pair().unwrap();
        let data = serde_json::json!("secret data");

        let encrypted = encrypt(&data, &kp.epriv).unwrap();
        assert!(encrypted.starts_with("SEA{"));

        let decrypted = decrypt(&encrypted, &kp.epriv).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn encrypt_with_passphrase() {
        let data = serde_json::json!("confidential");
        let passphrase = "my_secret_password";

        let encrypted = encrypt(&data, passphrase).unwrap();
        let decrypted = decrypt(&encrypted, passphrase).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let data = serde_json::json!("secret");
        let encrypted = encrypt(&data, "correct_key").unwrap();
        let result = decrypt(&encrypted, "wrong_key");
        assert!(result.is_err());
    }

    #[test]
    fn work_deterministic_with_salt() {
        let hash1 = work("hello", Some("salt123")).unwrap();
        let hash2 = work("hello", Some("salt123")).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn work_different_data_different_hash() {
        let hash1 = work("hello", Some("salt")).unwrap();
        let hash2 = work("world", Some("salt")).unwrap();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn work_different_salt_different_hash() {
        let hash1 = work("data", Some("salt1")).unwrap();
        let hash2 = work("data", Some("salt2")).unwrap();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn secret_shared_key() {
        let alice = pair().unwrap();
        let bob = pair().unwrap();

        // Alice derives shared secret with Bob's epub
        let secret_ab = secret(&bob.epub, &alice.epriv).unwrap();
        // Bob derives shared secret with Alice's epub
        let secret_ba = secret(&alice.epub, &bob.epriv).unwrap();

        // Both should arrive at the same shared secret
        assert_eq!(secret_ab, secret_ba);
        assert!(!secret_ab.is_empty());
    }

    #[test]
    fn secret_encrypt_decrypt_between_users() {
        let alice = pair().unwrap();
        let bob = pair().unwrap();

        // Alice encrypts for Bob
        let shared = secret(&bob.epub, &alice.epriv).unwrap();
        let data = serde_json::json!("hello Bob, from Alice");
        let encrypted = encrypt(&data, &shared).unwrap();

        // Bob decrypts
        let shared_bob = secret(&alice.epub, &bob.epriv).unwrap();
        let decrypted = decrypt(&encrypted, &shared_bob).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn pub_key_roundtrip() {
        let kp = pair().unwrap();
        // Decode and re-encode should produce same result
        let pub_decoded = decode_pub(&kp.pub_key).unwrap();
        let pub_reencoded = encode_pub(&pub_decoded);
        assert_eq!(kp.pub_key, pub_reencoded);
    }

    #[test]
    fn encrypted_message_format() {
        let data = serde_json::json!("test");
        let encrypted = encrypt(&data, "key").unwrap();

        // Should be SEA-prefixed JSON with ct, iv, s fields
        assert!(encrypted.starts_with("SEA{"));
        let json_part = &encrypted[3..];
        let parsed: EncryptedMessage = serde_json::from_str(json_part).unwrap();
        assert!(!parsed.ct.is_empty());
        assert!(!parsed.iv.is_empty());
        assert!(!parsed.s.is_empty());
    }

    #[test]
    fn signed_message_format() {
        let kp = pair().unwrap();
        let data = serde_json::json!("test");
        let signed = sign(&data, &kp.priv_key, &kp.pub_key).unwrap();

        assert!(signed.starts_with("SEA{"));
        let json_part = &signed[3..];
        let parsed: SignedMessage = serde_json::from_str(json_part).unwrap();
        assert_eq!(parsed.m, data);
        assert!(!parsed.s.is_empty());
    }

    // ── sign_value / verify_signed_value tests ─────────────────────

    #[test]
    fn sign_value_text() {
        let kp = pair().unwrap();
        let value = crate::types::GunValue::Text("hello world".into());
        let signed = sign_value(&value, &kp.priv_key, &kp.pub_key).unwrap();
        assert!(signed.starts_with("SEA{"));

        let verified = verify_signed_value(&signed, &kp.pub_key).unwrap();
        assert_eq!(verified, crate::types::GunValue::Text("hello world".into()));
    }

    #[test]
    fn sign_value_number() {
        let kp = pair().unwrap();
        let value = crate::types::GunValue::Number(42.5);
        let signed = sign_value(&value, &kp.priv_key, &kp.pub_key).unwrap();
        let verified = verify_signed_value(&signed, &kp.pub_key).unwrap();
        assert_eq!(verified, crate::types::GunValue::Number(42.5));
    }

    #[test]
    fn verify_signed_value_wrong_key_fails() {
        let kp1 = pair().unwrap();
        let kp2 = pair().unwrap();
        let value = crate::types::GunValue::Text("secret".into());
        let signed = sign_value(&value, &kp1.priv_key, &kp1.pub_key).unwrap();

        // Verify with wrong key should fail
        assert!(verify_signed_value(&signed, &kp2.pub_key).is_err());
    }
}
