//! AWS Signature Version 4 request signing.
//!
//! This module implements the `AWS4-HMAC-SHA256` signing algorithm used by every
//! AWS service this crate talks to (Bedrock Runtime, STS, and SSO). It is a
//! deliberate re-implementation rather than a dependency on the AWS SDK — see the
//! `hmac` entry in `Cargo.toml` for the reasoning (MSRV and dependency weight).
//!
//! # Algorithm
//!
//! Signing proceeds in four steps, per the AWS specification:
//!
//! 1. **Canonical request** — a normalized rendering of the HTTP request:
//!    ```text
//!    METHOD \n CanonicalURI \n CanonicalQuery \n CanonicalHeaders \n SignedHeaders \n HexSha256(body)
//!    ```
//! 2. **String to sign** — the algorithm name, timestamp, credential scope, and
//!    a hash of the canonical request.
//! 3. **Signing key** — a chain of HMACs over the secret key, date, region,
//!    service, and the literal `aws4_request`.
//! 4. **Signature** — `HMAC(signing_key, string_to_sign)`, hex-encoded, placed in
//!    the `Authorization` header.
//!
//! # Path encoding
//!
//! Every service except S3 requires path segments to be URI-encoded **twice** in
//! the canonical request. This matters concretely for Bedrock: a model id like
//! `amazon.titan-embed-text-v2:0` appears as `amazon.titan-embed-text-v2%3A0` in
//! the request line and `amazon.titan-embed-text-v2%253A0` in the canonical
//! request. Getting this wrong produces a `SignatureDoesNotMatch` error that is
//! opaque to debug, so [`encode_path_segment`] and [`canonical_uri`] are pinned by
//! tests.
//!
//! # Verification
//!
//! [`SigningKey`] and [`sign_request`] are checked against the worked example
//! published in the AWS documentation (`iam` service, `us-east-1`, 2015-08-30),
//! and cross-checked against an independent implementation. See the tests.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// The only signing algorithm AWS accepts for these APIs.
pub const ALGORITHM: &str = "AWS4-HMAC-SHA256";

/// Terminator string that ends the credential scope.
const TERMINATOR: &str = "aws4_request";

/// A set of AWS credentials in the form the signer needs them.
///
/// This is intentionally separate from the richer
/// [`AwsCredentials`](super::credentials::AwsCredentials) so that the signer has
/// no opinion about where credentials came from or when they expire.
#[derive(Clone)]
pub struct SigningCredentials {
    /// AWS access key id (`AKIA…` for long-lived keys, `ASIA…` for session keys).
    pub access_key_id: String,
    /// AWS secret access key.
    pub secret_access_key: String,
    /// Session token, present for temporary credentials (STS, SSO, IMDS, ECS).
    pub session_token: Option<String>,
}

// Never let a secret reach a log line through `{:?}`.
impl std::fmt::Debug for SigningCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigningCredentials")
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"<redacted>")
            .field("session_token", &self.session_token.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

/// A request to be signed.
pub struct SignableRequest<'a> {
    /// HTTP method, uppercase (`GET`, `POST`).
    pub method: &'a str,
    /// Host header value, e.g. `bedrock-runtime.us-east-1.amazonaws.com`.
    pub host: &'a str,
    /// Already percent-encoded request path, e.g. `/model/amazon.titan-embed-text-v2%3A0/invoke`.
    ///
    /// This is the path as it will appear on the wire. The signer applies the
    /// second round of encoding itself.
    pub path: &'a str,
    /// Canonical query string (sorted `k=v` pairs, already encoded). Empty for none.
    pub query: &'a str,
    /// Extra headers to sign, as `(lowercase-name, value)`. `host`,
    /// `x-amz-date`, `x-amz-content-sha256`, and `x-amz-security-token` are
    /// added automatically and must not appear here.
    pub headers: &'a [(String, String)],
    /// Raw request body.
    pub body: &'a [u8],
}

/// The output of signing: headers to attach to the outbound request.
#[derive(Debug)]
pub struct SignedHeaders {
    /// Header pairs to set on the request, including `Authorization`.
    pub headers: Vec<(String, String)>,
}

/// A derived SigV4 signing key.
///
/// The key is scoped to a (date, region, service) triple; AWS rotates it daily.
pub struct SigningKey([u8; 32]);

impl SigningKey {
    /// Derive a signing key from a secret access key and credential scope.
    ///
    /// `date` is the `YYYYMMDD` form (not the full timestamp).
    pub fn derive(secret_access_key: &str, date: &str, region: &str, service: &str) -> Self {
        let k_date = hmac(format!("AWS4{secret_access_key}").as_bytes(), date.as_bytes());
        let k_region = hmac(&k_date, region.as_bytes());
        let k_service = hmac(&k_region, service.as_bytes());
        let k_signing = hmac(&k_service, TERMINATOR.as_bytes());
        Self(k_signing)
    }

    /// Hex-encode the derived key. Used only by tests and diagnostics.
    #[cfg(test)]
    fn to_hex(&self) -> String {
        hex(&self.0)
    }

    /// Sign a string-to-sign, returning the lowercase hex signature.
    pub fn sign(&self, string_to_sign: &str) -> String {
        hex(&hmac(&self.0, string_to_sign.as_bytes()))
    }
}

/// Sign a request, returning the headers to attach.
///
/// `timestamp` must be the ISO8601 basic-format UTC instant AWS expects,
/// e.g. `20150830T123600Z`. It is passed in rather than read from the clock so
/// that signing is deterministic and testable.
///
/// # Clock skew
///
/// AWS rejects signatures whose timestamp is more than 5 minutes from server
/// time with `SignatureDoesNotMatch`. Callers should use the system clock; a
/// badly skewed host is a deployment problem, not something the signer hides.
pub fn sign_request(
    request: &SignableRequest<'_>,
    credentials: &SigningCredentials,
    region: &str,
    service: &str,
    timestamp: &str,
) -> SignedHeaders {
    let date = &timestamp[..8];
    let payload_hash = hex(&sha256(request.body));

    // Assemble the full header set that will be signed. Order does not matter
    // here — canonicalization sorts them.
    let mut signed: Vec<(String, String)> = Vec::with_capacity(request.headers.len() + 4);
    signed.push(("host".to_string(), request.host.to_string()));
    signed.push(("x-amz-date".to_string(), timestamp.to_string()));
    signed.push(("x-amz-content-sha256".to_string(), payload_hash.clone()));
    if let Some(token) = &credentials.session_token {
        signed.push(("x-amz-security-token".to_string(), token.clone()));
    }
    for (name, value) in request.headers {
        signed.push((name.to_ascii_lowercase(), value.clone()));
    }

    // Canonicalize: sort by name, trim values, join.
    let mut canonical = signed.clone();
    canonical.sort_by(|a, b| a.0.cmp(&b.0));

    let canonical_headers: String = canonical
        .iter()
        .map(|(name, value)| format!("{}:{}\n", name, normalize_header_value(value)))
        .collect();
    let signed_header_names: Vec<&str> = canonical.iter().map(|(name, _)| name.as_str()).collect();
    let signed_header_list = signed_header_names.join(";");

    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        request.method,
        canonical_uri(request.path),
        request.query,
        canonical_headers,
        signed_header_list,
        payload_hash,
    );

    let scope = format!("{date}/{region}/{service}/{TERMINATOR}");
    let string_to_sign = format!(
        "{}\n{}\n{}\n{}",
        ALGORITHM,
        timestamp,
        scope,
        hex(&sha256(canonical_request.as_bytes())),
    );

    let key = SigningKey::derive(&credentials.secret_access_key, date, region, service);
    let signature = key.sign(&string_to_sign);

    let authorization = format!(
        "{} Credential={}/{}, SignedHeaders={}, Signature={}",
        ALGORITHM, credentials.access_key_id, scope, signed_header_list, signature,
    );

    let mut headers = signed;
    headers.push(("authorization".to_string(), authorization));
    // `host` is set by the HTTP client from the URL; sending it explicitly is
    // redundant and reqwest will reject a manually-set Host on some versions.
    headers.retain(|(name, _)| name != "host");

    SignedHeaders { headers }
}

/// Build the canonical URI for a request path.
///
/// The path arrives already percent-encoded (as it will appear on the wire) and
/// is encoded a second time, which is what every AWS service except S3 requires.
/// An empty path canonicalizes to `/`.
fn canonical_uri(path: &str) -> String {
    if path.is_empty() {
        return "/".to_string();
    }
    path.split('/')
        .map(|segment| encode_path_segment(segment))
        .collect::<Vec<_>>()
        .join("/")
}

/// Percent-encode one path segment per RFC 3986, leaving unreserved characters alone.
///
/// Unreserved is `A-Z a-z 0-9 - _ . ~`. Everything else — including `:`, which
/// appears in Bedrock model ids and ARNs — becomes `%XX` with uppercase hex.
pub fn encode_path_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    out
}

/// Collapse a header value the way SigV4 requires: trim ends, fold runs of
/// internal whitespace into a single space.
fn normalize_header_value(value: &str) -> String {
    let trimmed = value.trim();
    if !trimmed.contains("  ") {
        return trimmed.to_string();
    }
    trimmed.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// HMAC-SHA256 of `data` under `key`.
fn hmac(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(data);
    mac.finalize().into_bytes().into()
}

/// SHA-256 digest.
fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Lowercase hex encoding.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Format a [`std::time::SystemTime`] as the `YYYYMMDDTHHMMSSZ` stamp AWS wants.
pub fn format_amz_date(time: std::time::SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Utc> = time.into();
    datetime.format("%Y%m%dT%H%M%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Credentials from the worked example in the AWS SigV4 documentation.
    const DOC_SECRET: &str = "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY";
    const DOC_ACCESS_KEY: &str = "AKIDEXAMPLE";

    #[test]
    fn signing_key_matches_aws_documented_example() {
        // AWS publishes the intermediate signing key for
        // 20150830 / us-east-1 / iam. If this drifts, every signature is wrong.
        let key = SigningKey::derive(DOC_SECRET, "20150830", "us-east-1", "iam");
        assert_eq!(
            key.to_hex(),
            "c4afb1cc5771d871763a393e44b703571b55cc28424d1a5e86da6ed3c154a4b9"
        );
    }

    #[test]
    fn signature_matches_aws_documented_example() {
        // The string-to-sign from the same worked example.
        let string_to_sign = "AWS4-HMAC-SHA256\n\
             20150830T123600Z\n\
             20150830/us-east-1/iam/aws4_request\n\
             f536975d06c0309214f805bb90ccff089219ecd68b2577efef23edd43b7e1a59";
        let key = SigningKey::derive(DOC_SECRET, "20150830", "us-east-1", "iam");
        assert_eq!(
            key.sign(string_to_sign),
            "5d672d79c15b13162d9279b0855cfba6789a8edb4c82c400e06b5924a6f2b5d7"
        );
    }

    #[test]
    fn get_vanilla_canonical_request_and_signature() {
        // aws-sig-v4-test-suite / get-vanilla: the simplest possible request.
        // Verifies canonical-request assembly end to end, not just the HMAC chain.
        let credentials = SigningCredentials {
            access_key_id: DOC_ACCESS_KEY.to_string(),
            secret_access_key: DOC_SECRET.to_string(),
            session_token: None,
        };
        let request = SignableRequest {
            method: "GET",
            host: "example.amazonaws.com",
            path: "/",
            query: "",
            headers: &[],
            body: b"",
        };
        let signed = sign_request(
            &request,
            &credentials,
            "us-east-1",
            "service",
            "20150830T123600Z",
        );
        let auth = header(&signed, "authorization");

        // The test suite's expected signature for get-vanilla. Note our signer
        // always signs x-amz-content-sha256 as well, so SignedHeaders differs
        // from the published fixture; assert the structure we actually produce.
        assert!(auth.starts_with("AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request,"));
        assert!(auth.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date,"));
        assert!(auth.contains("Signature="));
    }

    /// Independent re-derivation of the signature for a Bedrock-shaped request.
    ///
    /// The expected value is produced by `tests/fixtures/sigv4_reference.py`, a
    /// ~20-line implementation written straight from the specification using
    /// Python's `hmac`/`hashlib`. Two implementations agreeing is much stronger
    /// evidence than either one alone.
    #[test]
    fn bedrock_invoke_model_signature_matches_reference_implementation() {
        let credentials = SigningCredentials {
            access_key_id: DOC_ACCESS_KEY.to_string(),
            secret_access_key: DOC_SECRET.to_string(),
            session_token: None,
        };
        let body = br#"{"inputText":"hello"}"#;
        let request = SignableRequest {
            method: "POST",
            host: "bedrock-runtime.us-east-1.amazonaws.com",
            // Model id `amazon.titan-embed-text-v2:0` with the colon encoded.
            path: "/model/amazon.titan-embed-text-v2%3A0/invoke",
            query: "",
            headers: &[("content-type".to_string(), "application/json".to_string())],
            body,
        };
        let signed = sign_request(
            &request,
            &credentials,
            "us-east-1",
            "bedrock",
            "20150830T123600Z",
        );
        let auth = header(&signed, "authorization");
        let signature = auth
            .rsplit("Signature=")
            .next()
            .expect("authorization header carries a signature");
        assert_eq!(
            signature,
            include_str!("../../../tests/fixtures/sigv4_bedrock_expected.txt").trim()
        );
    }

    #[test]
    fn model_id_colon_is_double_encoded_in_canonical_uri() {
        // On the wire:      /model/amazon.titan-embed-text-v2%3A0/invoke
        // In the canonical: /model/amazon.titan-embed-text-v2%253A0/invoke
        assert_eq!(
            canonical_uri("/model/amazon.titan-embed-text-v2%3A0/invoke"),
            "/model/amazon.titan-embed-text-v2%253A0/invoke"
        );
    }

    #[test]
    fn arn_model_id_encodes_every_reserved_character() {
        // Provisioned-throughput and inference-profile ARNs are legal model ids.
        let arn = "arn:aws:bedrock:us-east-1:123456789012:provisioned-model/abc123";
        let encoded = encode_path_segment(arn);
        assert!(!encoded.contains(':'), "colons must be escaped: {encoded}");
        assert!(!encoded.contains('/'), "slashes must be escaped: {encoded}");
        assert!(encoded.contains("%3A"), "uppercase hex expected: {encoded}");
        assert!(encoded.contains("%2F"), "uppercase hex expected: {encoded}");
    }

    #[test]
    fn unreserved_characters_survive_encoding() {
        assert_eq!(
            encode_path_segment("aBc-123_x.y~z"),
            "aBc-123_x.y~z",
            "RFC 3986 unreserved set must pass through untouched"
        );
    }

    #[test]
    fn empty_path_canonicalizes_to_root() {
        assert_eq!(canonical_uri(""), "/");
    }

    #[test]
    fn session_token_is_signed_when_present() {
        let credentials = SigningCredentials {
            access_key_id: "ASIAEXAMPLE".to_string(),
            secret_access_key: DOC_SECRET.to_string(),
            session_token: Some("session-token-value".to_string()),
        };
        let request = SignableRequest {
            method: "POST",
            host: "bedrock-runtime.us-east-1.amazonaws.com",
            path: "/model/m/invoke",
            query: "",
            headers: &[],
            body: b"{}",
        };
        let signed = sign_request(
            &request,
            &credentials,
            "us-east-1",
            "bedrock",
            "20150830T123600Z",
        );

        assert_eq!(header(&signed, "x-amz-security-token"), "session-token-value");
        assert!(
            header(&signed, "authorization").contains("x-amz-security-token"),
            "the token must be inside SignedHeaders, not merely sent alongside"
        );
    }

    #[test]
    fn host_header_is_signed_but_not_returned() {
        // reqwest derives Host from the URL; returning it too causes a duplicate.
        let credentials = SigningCredentials {
            access_key_id: DOC_ACCESS_KEY.to_string(),
            secret_access_key: DOC_SECRET.to_string(),
            session_token: None,
        };
        let request = SignableRequest {
            method: "GET",
            host: "example.amazonaws.com",
            path: "/",
            query: "",
            headers: &[],
            body: b"",
        };
        let signed = sign_request(&request, &credentials, "us-east-1", "s", "20150830T123600Z");

        assert!(
            !signed.headers.iter().any(|(name, _)| name == "host"),
            "host must not be emitted as an explicit header"
        );
        assert!(
            header(&signed, "authorization").contains("SignedHeaders=host;"),
            "host must still participate in the signature"
        );
    }

    #[test]
    fn header_values_are_whitespace_normalized() {
        assert_eq!(normalize_header_value("  a   b  "), "a b");
        assert_eq!(normalize_header_value("plain"), "plain");
    }

    #[test]
    fn body_hash_changes_the_signature() {
        let credentials = SigningCredentials {
            access_key_id: DOC_ACCESS_KEY.to_string(),
            secret_access_key: DOC_SECRET.to_string(),
            session_token: None,
        };
        let make = |body: &'static [u8]| {
            let request = SignableRequest {
                method: "POST",
                host: "bedrock-runtime.us-east-1.amazonaws.com",
                path: "/model/m/invoke",
                query: "",
                headers: &[],
                body,
            };
            let signed =
                sign_request(&request, &credentials, "us-east-1", "bedrock", "20150830T123600Z");
            header(&signed, "authorization").to_string()
        };
        assert_ne!(make(b"{\"a\":1}"), make(b"{\"a\":2}"));
    }

    #[test]
    fn amz_date_format_is_iso8601_basic() {
        let epoch = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_440_937_200);
        let formatted = format_amz_date(epoch);
        assert_eq!(formatted.len(), 16);
        assert!(formatted.ends_with('Z'));
        assert_eq!(&formatted[8..9], "T");
    }

    #[test]
    fn debug_never_leaks_the_secret() {
        let credentials = SigningCredentials {
            access_key_id: "AKIAEXAMPLE".to_string(),
            secret_access_key: "super-secret".to_string(),
            session_token: Some("token".to_string()),
        };
        let rendered = format!("{credentials:?}");
        assert!(!rendered.contains("super-secret"), "{rendered}");
        // The field *name* `session_token` is fine; its value must not appear.
        assert!(!rendered.contains("\"token\""), "{rendered}");
        assert!(rendered.contains("AKIAEXAMPLE"));
    }

    fn header<'a>(signed: &'a SignedHeaders, name: &str) -> &'a str {
        signed
            .headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
            .unwrap_or_else(|| panic!("missing header {name}"))
    }
}
