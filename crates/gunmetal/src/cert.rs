//! Certificates — delegated write permissions for user graph namespaces.
//!
//! GUN's certificate system (`SEA.certify`) allows a user to grant
//! other users write permission to paths within their `~pubKey/...` namespace.
//!
//! ## How it works
//!
//! 1. Alice creates a certificate granting Bob write access to `~alice/shared/`
//! 2. The certificate is signed with Alice's private key
//! 3. When Bob writes to `~alice/shared/data`, the system checks:
//!    - Is Bob the owner? No (Alice is)
//!    - Does a valid, non-expired certificate grant Bob access? Check `~alice/certs/`
//! 4. If a matching certificate exists, the write is allowed
//!
//! ## Revocation
//!
//! Tombstone the certificate node with `put(Null)`. HAM ensures the newer
//! tombstone propagates to all peers.

use base64::Engine;

use crate::sea;
use crate::types::GunValue;

/// Who a certificate grants access to.
#[derive(Debug, Clone, PartialEq)]
pub enum CertWho {
    /// A specific user identified by their public key.
    PubKey(String),
    /// Any authenticated user.
    Anyone,
}

/// What paths a certificate grants access to.
#[derive(Debug, Clone, PartialEq)]
pub enum CertWhat {
    /// Exact path match (e.g., `"profile/name"`).
    Exact(String),
    /// Prefix match (e.g., `"shared/"` matches `"shared/anything"`).
    Prefix(String),
    /// All paths under the owner's namespace.
    All,
}

/// A certificate granting delegated write permission.
#[derive(Debug, Clone)]
pub struct Certificate {
    /// Who is granted access.
    pub who: CertWho,
    /// What paths they can access.
    pub what: CertWhat,
    /// When the certificate expires (ms since epoch), or None for no expiry.
    pub expiry: Option<f64>,
    /// The issuer's public key (the namespace owner).
    pub issuer: String,
    /// ECDSA signature over the canonical certificate content.
    pub signature: String,
}

/// Canonical form of a certificate for signing/verification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CertCanonical {
    who: String,       // pub key or "*"
    what: String,      // path, "path*" for prefix, or "*" for all
    expiry: Option<f64>,
    issuer: String,
}

impl Certificate {
    /// Create and sign a new certificate.
    ///
    /// The issuer's private key is used to sign the certificate content.
    /// Returns the signed certificate.
    pub fn create(
        who: CertWho,
        what: CertWhat,
        expiry: Option<f64>,
        issuer_pub: &str,
        issuer_priv: &str,
    ) -> Result<Self, String> {
        let canonical = CertCanonical {
            who: match &who {
                CertWho::PubKey(pk) => pk.clone(),
                CertWho::Anyone => "*".to_string(),
            },
            what: match &what {
                CertWhat::Exact(p) => p.clone(),
                CertWhat::Prefix(p) => format!("{}*", p),
                CertWhat::All => "*".to_string(),
            },
            expiry,
            issuer: issuer_pub.to_string(),
        };

        let data = serde_json::to_value(&canonical)
            .map_err(|e| format!("Failed to serialize certificate: {}", e))?;

        let signed = sea::sign(&data, issuer_priv, issuer_pub)
            .map_err(|e| format!("Failed to sign certificate: {}", e))?;

        Ok(Certificate {
            who,
            what,
            expiry,
            issuer: issuer_pub.to_string(),
            signature: signed,
        })
    }

    /// Verify a certificate's signature against the issuer's public key.
    pub fn verify(&self) -> Result<bool, String> {
        let canonical = CertCanonical {
            who: match &self.who {
                CertWho::PubKey(pk) => pk.clone(),
                CertWho::Anyone => "*".to_string(),
            },
            what: match &self.what {
                CertWhat::Exact(p) => p.clone(),
                CertWhat::Prefix(p) => format!("{}*", p),
                CertWhat::All => "*".to_string(),
            },
            expiry: self.expiry,
            issuer: self.issuer.clone(),
        };

        let expected_data = serde_json::to_value(&canonical)
            .map_err(|e| format!("Failed to serialize for verify: {}", e))?;

        match sea::verify(&self.signature, &self.issuer) {
            Ok(verified_data) => Ok(verified_data == expected_data),
            Err(_) => Ok(false),
        }
    }

    /// Check if this certificate has expired.
    pub fn is_expired(&self, now_ms: f64) -> bool {
        self.expiry.is_some_and(|exp| now_ms > exp)
    }

    /// Check if this certificate grants access for the given writer and path.
    pub fn grants_access(&self, writer_pub: &str, path: &str, now_ms: f64) -> bool {
        // Check expiry
        if self.is_expired(now_ms) {
            return false;
        }

        // Check who
        match &self.who {
            CertWho::PubKey(pk) => {
                if pk != writer_pub {
                    return false;
                }
            }
            CertWho::Anyone => {} // anyone is allowed
        }

        // Check what
        match &self.what {
            CertWhat::Exact(p) => path == p,
            CertWhat::Prefix(p) => path.starts_with(p),
            CertWhat::All => true,
        }
    }

    /// Generate a unique certificate ID from its content.
    ///
    /// Used as the key under `~<issuer>/certs/<cert_id>`.
    pub fn cert_id(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.signature.as_bytes());
        let hash = hasher.finalize();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&hash[..16])
    }

    /// Serialize to a GunValue for storage in the graph.
    pub fn to_gun_value(&self) -> GunValue {
        let json = serde_json::json!({
            "who": match &self.who {
                CertWho::PubKey(pk) => pk.clone(),
                CertWho::Anyone => "*".to_string(),
            },
            "what": match &self.what {
                CertWhat::Exact(p) => p.clone(),
                CertWhat::Prefix(p) => format!("{}*", p),
                CertWhat::All => "*".to_string(),
            },
            "expiry": self.expiry,
            "issuer": self.issuer,
            "signature": self.signature,
        });
        GunValue::Text(json.to_string())
    }

    /// Deserialize from a GunValue stored in the graph.
    pub fn from_gun_value(value: &GunValue) -> Option<Self> {
        let text = match value {
            GunValue::Text(s) => s,
            _ => return None,
        };

        let json: serde_json::Value = serde_json::from_str(text).ok()?;
        let obj = json.as_object()?;

        let who_str = obj.get("who")?.as_str()?;
        let what_str = obj.get("what")?.as_str()?;
        let issuer = obj.get("issuer")?.as_str()?;
        let signature = obj.get("signature")?.as_str()?;
        let expiry = obj.get("expiry").and_then(|v| v.as_f64());

        let who = if who_str == "*" {
            CertWho::Anyone
        } else {
            CertWho::PubKey(who_str.to_string())
        };

        let what = if what_str == "*" {
            CertWhat::All
        } else if let Some(prefix) = what_str.strip_suffix('*') {
            CertWhat::Prefix(prefix.to_string())
        } else {
            CertWhat::Exact(what_str.to_string())
        };

        Some(Certificate {
            who,
            what,
            expiry,
            issuer: issuer.to_string(),
            signature: signature.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pair() -> sea::SEAPair {
        sea::pair().unwrap()
    }

    #[test]
    fn create_and_verify_certificate() {
        let issuer = test_pair();
        let grantee = test_pair();

        let cert = Certificate::create(
            CertWho::PubKey(grantee.pub_key.clone()),
            CertWhat::Prefix("shared/".to_string()),
            None,
            &issuer.pub_key,
            &issuer.priv_key,
        )
        .unwrap();

        assert!(cert.verify().unwrap());
        assert_eq!(cert.issuer, issuer.pub_key);
    }

    #[test]
    fn certificate_grants_access_specific_user() {
        let issuer = test_pair();
        let grantee = test_pair();
        let other = test_pair();

        let cert = Certificate::create(
            CertWho::PubKey(grantee.pub_key.clone()),
            CertWhat::Prefix("shared/".to_string()),
            None,
            &issuer.pub_key,
            &issuer.priv_key,
        )
        .unwrap();

        // Grantee can access shared/
        assert!(cert.grants_access(&grantee.pub_key, "shared/data", 0.0));
        assert!(cert.grants_access(&grantee.pub_key, "shared/nested/deep", 0.0));

        // Other user cannot
        assert!(!cert.grants_access(&other.pub_key, "shared/data", 0.0));

        // Grantee cannot access outside prefix
        assert!(!cert.grants_access(&grantee.pub_key, "private/data", 0.0));
    }

    #[test]
    fn certificate_anyone_access() {
        let issuer = test_pair();
        let random_user = test_pair();

        let cert = Certificate::create(
            CertWho::Anyone,
            CertWhat::Prefix("public/".to_string()),
            None,
            &issuer.pub_key,
            &issuer.priv_key,
        )
        .unwrap();

        assert!(cert.grants_access(&random_user.pub_key, "public/post", 0.0));
    }

    #[test]
    fn certificate_exact_path() {
        let issuer = test_pair();
        let grantee = test_pair();

        let cert = Certificate::create(
            CertWho::PubKey(grantee.pub_key.clone()),
            CertWhat::Exact("specific/key".to_string()),
            None,
            &issuer.pub_key,
            &issuer.priv_key,
        )
        .unwrap();

        assert!(cert.grants_access(&grantee.pub_key, "specific/key", 0.0));
        assert!(!cert.grants_access(&grantee.pub_key, "specific/other", 0.0));
    }

    #[test]
    fn certificate_all_paths() {
        let issuer = test_pair();
        let grantee = test_pair();

        let cert = Certificate::create(
            CertWho::PubKey(grantee.pub_key.clone()),
            CertWhat::All,
            None,
            &issuer.pub_key,
            &issuer.priv_key,
        )
        .unwrap();

        assert!(cert.grants_access(&grantee.pub_key, "anything/at/all", 0.0));
    }

    #[test]
    fn certificate_expiry() {
        let issuer = test_pair();
        let grantee = test_pair();

        let cert = Certificate::create(
            CertWho::PubKey(grantee.pub_key.clone()),
            CertWhat::All,
            Some(1000.0), // expires at 1000ms
            &issuer.pub_key,
            &issuer.priv_key,
        )
        .unwrap();

        // Before expiry
        assert!(cert.grants_access(&grantee.pub_key, "data", 500.0));
        assert!(!cert.is_expired(500.0));

        // After expiry
        assert!(!cert.grants_access(&grantee.pub_key, "data", 1500.0));
        assert!(cert.is_expired(1500.0));
    }

    #[test]
    fn certificate_no_expiry() {
        let issuer = test_pair();
        let cert = Certificate::create(
            CertWho::Anyone,
            CertWhat::All,
            None,
            &issuer.pub_key,
            &issuer.priv_key,
        )
        .unwrap();

        assert!(!cert.is_expired(f64::MAX));
    }

    #[test]
    fn certificate_serialization_roundtrip() {
        let issuer = test_pair();
        let grantee = test_pair();

        let cert = Certificate::create(
            CertWho::PubKey(grantee.pub_key.clone()),
            CertWhat::Prefix("shared/".to_string()),
            Some(99999.0),
            &issuer.pub_key,
            &issuer.priv_key,
        )
        .unwrap();

        let value = cert.to_gun_value();
        let restored = Certificate::from_gun_value(&value).unwrap();

        assert_eq!(restored.who, cert.who);
        assert_eq!(restored.what, cert.what);
        assert_eq!(restored.expiry, cert.expiry);
        assert_eq!(restored.issuer, cert.issuer);
        assert_eq!(restored.signature, cert.signature);
    }

    #[test]
    fn certificate_id_is_deterministic() {
        let issuer = test_pair();
        let cert = Certificate::create(
            CertWho::Anyone,
            CertWhat::All,
            None,
            &issuer.pub_key,
            &issuer.priv_key,
        )
        .unwrap();

        let id1 = cert.cert_id();
        let id2 = cert.cert_id();
        assert_eq!(id1, id2);
        assert!(!id1.is_empty());
    }

    #[test]
    fn from_gun_value_null_returns_none() {
        assert!(Certificate::from_gun_value(&GunValue::Null).is_none());
    }

    #[test]
    fn from_gun_value_invalid_json_returns_none() {
        assert!(Certificate::from_gun_value(&GunValue::Text("not json".into())).is_none());
    }
}
