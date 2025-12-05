use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

use crate::daemon::types::{JsonRpcRequest, JsonRpcResponse};

/// Protocol version for compatibility checking.
/// Increment when making breaking changes to message format.
pub const PROTOCOL_VERSION: u32 = 1;

/// Maximum message size: 10MB
/// Prevents memory exhaustion from malicious/malformed messages
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Codec for length-prefixed JSON-RPC messages over Unix sockets.
///
/// Uses tokio_util's battle-tested LengthDelimitedCodec for framing,
/// then deserializes JSON payload.
pub struct JsonRpcCodec {
    inner: LengthDelimitedCodec,
}

impl JsonRpcCodec {
    pub fn new() -> Self {
        Self {
            inner: LengthDelimitedCodec::builder()
                .max_frame_length(MAX_MESSAGE_SIZE)
                .length_field_type::<u32>()
                .big_endian()
                .new_codec(),
        }
    }
}

impl Default for JsonRpcCodec {
    fn default() -> Self {
        Self::new()
    }
}

/// JSON-RPC message envelope (request or response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
}

impl Decoder for JsonRpcCodec {
    type Item = JsonRpcMessage;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Delegate framing to LengthDelimitedCodec
        if let Some(bytes) = self.inner.decode(src)? {
            // Parse JSON payload
            let message = serde_json::from_slice(&bytes).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid JSON-RPC message: {}", e),
                )
            })?;
            Ok(Some(message))
        } else {
            Ok(None) // Need more data
        }
    }
}

impl Encoder<JsonRpcMessage> for JsonRpcCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: JsonRpcMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Serialize to JSON
        let json = serde_json::to_vec(&item).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize JSON-RPC message: {}", e),
            )
        })?;

        // Delegate framing to LengthDelimitedCodec
        self.inner.encode(Bytes::from(json), dst)
    }
}

/// Initial handshake message sent by client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handshake {
    pub version: u32,
    pub client_id: uuid::Uuid,
}

impl Handshake {
    pub fn new(client_id: uuid::Uuid) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            client_id,
        }
    }

    pub fn is_compatible(&self) -> bool {
        // Major version must match (breaking changes)
        self.version == PROTOCOL_VERSION
    }
}

/// Handshake response from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub version: u32,
    pub accepted: bool,
    pub reason: Option<String>,
}

impl HandshakeResponse {
    pub fn accepted() -> Self {
        Self {
            version: PROTOCOL_VERSION,
            accepted: true,
            reason: None,
        }
    }

    pub fn rejected(reason: String) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            accepted: false,
            reason: Some(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_encode_decode_round_trip() {
        let mut codec = JsonRpcCodec::new();
        let mut buffer = BytesMut::new();

        let request = JsonRpcMessage::Request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "search".into(),
            params: Some(serde_json::json!({"query": "test"})),
            id: Some(serde_json::json!(42)),
        });

        // Encode
        codec.encode(request.clone(), &mut buffer).unwrap();
        assert!(buffer.len() > 4); // Length prefix + payload

        // Decode
        let decoded = codec.decode(&mut buffer).unwrap().unwrap();
        match (request, decoded) {
            (JsonRpcMessage::Request(r1), JsonRpcMessage::Request(r2)) => {
                assert_eq!(r1.method, r2.method);
                assert_eq!(r1.id, r2.id);
            }
            _ => panic!("Type mismatch"),
        }
    }

    #[test]
    fn test_partial_read_handling() {
        let mut codec = JsonRpcCodec::new();
        let mut buffer = BytesMut::new();

        let message = JsonRpcMessage::Request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "ping".into(),
            params: None,
            id: Some(serde_json::json!(1)),
        });

        // Encode full message
        codec.encode(message.clone(), &mut buffer).unwrap();
        let full_len = buffer.len();

        // Split buffer to simulate partial read
        let first_half = buffer.split_to(full_len / 2);
        let second_half = buffer.clone();

        // Try decoding first half
        let mut partial_buf = first_half.clone();
        let result = codec.decode(&mut partial_buf).unwrap();
        assert!(result.is_none()); // Should return None (need more data)

        // Add second half
        partial_buf.unsplit(second_half);
        let result = codec.decode(&mut partial_buf).unwrap();
        assert!(result.is_some()); // Should now decode successfully
    }

    #[test]
    fn test_oversized_message_rejected() {
        let mut codec = JsonRpcCodec::new();
        let mut buffer = BytesMut::new();

        // Create message larger than MAX_MESSAGE_SIZE
        let huge_payload = "x".repeat(11 * 1024 * 1024); // 11MB
        let message = JsonRpcMessage::Request(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: "test".into(),
            params: Some(serde_json::json!({"data": huge_payload})),
            id: Some(serde_json::json!(1)),
        });

        // Encoding huge message should fail
        let result = codec.encode(message, &mut buffer);
        assert!(result.is_err());
    }

    #[test]
    fn test_handshake_version_check() {
        let handshake = Handshake::new(uuid::Uuid::new_v4());
        assert_eq!(handshake.version, PROTOCOL_VERSION);
        assert!(handshake.is_compatible());

        let old_version = Handshake {
            version: 0,
            client_id: uuid::Uuid::new_v4(),
        };
        assert!(!old_version.is_compatible());
    }

    #[test]
    fn test_handshake_response() {
        let accepted = HandshakeResponse::accepted();
        assert!(accepted.accepted);
        assert!(accepted.reason.is_none());

        let rejected = HandshakeResponse::rejected("Version mismatch".into());
        assert!(!rejected.accepted);
        assert_eq!(rejected.reason.as_deref(), Some("Version mismatch"));
    }
}
