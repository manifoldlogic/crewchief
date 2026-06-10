//! UUID generation for GUN's `.set()` operation.
//!
//! Generates time-sortable unique identifiers used as collection keys.
//! Format: `<timestamp_base36><random_base36_12chars>` (~20 chars total).
//!
//! Properties:
//! - Time-sortable: lexicographic ordering matches chronological ordering
//! - ~62 bits of collision resistance from the random suffix
//! - Uses `OsRng` for cryptographic-quality randomness (M16)

use rand::rngs::OsRng;
use rand::RngCore;

use crate::state::now_ms;

/// Generate a unique, time-sortable identifier.
///
/// Format: `<timestamp_base36><random_base36_12chars>`
///
/// Used by `GunChain::set_value()` to generate unique souls for
/// anonymous items added to collections.
pub fn generate_uuid() -> String {
    let timestamp = now_ms() as u64;
    let ts_part = base36_encode(timestamp);
    let rand_part = random_base36(12);
    format!("{}{}", ts_part, rand_part)
}

/// Encode a u64 as a base-36 string (digits 0-9, letters a-z).
fn base36_encode(mut n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }

    const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut buf = Vec::with_capacity(14); // u64::MAX is 13 chars in base36

    while n > 0 {
        buf.push(CHARS[(n % 36) as usize]);
        n /= 36;
    }

    buf.reverse();
    String::from_utf8(buf).expect("base36 chars are valid UTF-8")
}

/// Generate a random base-36 string of the given length.
fn random_base36(len: usize) -> String {
    const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut rng = OsRng;
    (0..len)
        .map(|_| {
            let idx = (rng.next_u32() % 36) as usize;
            CHARS[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base36_zero() {
        assert_eq!(base36_encode(0), "0");
    }

    #[test]
    fn base36_known_values() {
        assert_eq!(base36_encode(35), "z");
        assert_eq!(base36_encode(36), "10");
        assert_eq!(base36_encode(100), "2s");
        assert_eq!(base36_encode(1_000_000), "lfls");
    }

    #[test]
    fn base36_roundtrip_ordering() {
        // Larger numbers with the same digit count sort lexicographically later.
        // Real timestamps (ms since epoch) always have the same base36 length.
        let a = base36_encode(1_000_000_000);
        let b = base36_encode(2_000_000_000);
        assert_eq!(a.len(), b.len(), "same-magnitude numbers should have same base36 length");
        assert!(b > a, "expected {} > {}", b, a);
    }

    #[test]
    fn random_base36_length() {
        let s = random_base36(12);
        assert_eq!(s.len(), 12);
    }

    #[test]
    fn random_base36_valid_chars() {
        let s = random_base36(100);
        assert!(s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }

    #[test]
    fn generate_uuid_nonempty() {
        let id = generate_uuid();
        assert!(!id.is_empty());
    }

    #[test]
    fn generate_uuid_unique() {
        let ids: Vec<String> = (0..100).map(|_| generate_uuid()).collect();
        let unique: std::collections::HashSet<&String> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len(), "UUIDs should be unique");
    }

    #[test]
    fn generate_uuid_time_sortable() {
        // UUIDs generated later should sort after earlier ones.
        // The timestamp prefix ensures this (same-length base36 strings
        // of larger numbers sort later).
        let a = generate_uuid();
        // Small delay to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = generate_uuid();
        assert!(b > a, "expected later UUID {} > earlier UUID {}", b, a);
    }

    #[test]
    fn generate_uuid_reasonable_length() {
        let id = generate_uuid();
        // Timestamp in base36 is ~9 chars for current epoch ms, + 12 random = ~21
        assert!(id.len() >= 15, "UUID too short: {} (len {})", id, id.len());
        assert!(id.len() <= 30, "UUID too long: {} (len {})", id, id.len());
    }
}
