//! HAM — Hypothetical Amnesia Machine.
//!
//! The core CRDT conflict resolution algorithm that everything in GUN wraps.
//! Given five parameters, HAM decides whether to accept, reject, or defer an
//! incoming value for a specific key on a node.
//!
//! From the GUN source (`gun.js:578`):
//! ```js
//! function ham(val, key, soul, state, msg){
//!     var vertex = graph[soul] || empty, was = state_is(vertex, key, 1), known = vertex[key];
//!     var now = State(), u;
//!     if(state > now){
//!         setTimeout(function(){ ham(val, key, soul, state, msg) }, ...); // future
//!         return;
//!     }
//!     if(state < was){ return } // old, discard
//!     if(state === was && (val === known || L(val) <= L(known))){ return } // same state, tie-break by serialized length
//!     // accept: state > was, OR state == was && L(val) > L(known)
//! }
//! ```
//!
//! The algorithm reduces to:
//! 1. **Future**: if incoming state > machine time → defer
//! 2. **Old**: if incoming state < current state for that key → discard
//! 3. **Same**: if states are equal, tie-break by `JSON.stringify` length (larger wins),
//!    and if equal length, by value equality → discard if same or smaller
//! 4. **Accept**: incoming state is newer, or same state with larger serialized value

use crate::types::GunValue;

/// The result of HAM conflict resolution for a single key-value update.
#[derive(Debug, Clone, PartialEq)]
pub enum HamResult {
    /// The incoming value should be accepted — it wins the conflict.
    Accept,

    /// The incoming value is old — the current value already has a newer state.
    Old,

    /// The incoming value is identical or smaller at the same state — discard.
    Same,

    /// The incoming state is in the future relative to our clock.
    /// The `f64` is how many milliseconds in the future it is.
    /// The caller should defer and retry after this duration.
    Future(f64),

    /// An error occurred (e.g., incoming state is invalid).
    Error(&'static str),
}

/// Run the HAM conflict resolution algorithm.
///
/// # Parameters
///
/// - `machine_state`: Current local machine time (from `State::now()`)
/// - `incoming_state`: The state timestamp from the remote peer's update
/// - `current_state`: The state timestamp of the locally stored value for this key
/// - `incoming_value`: The value the remote peer wants to write
/// - `current_value`: The value currently stored locally (None if key doesn't exist)
///
/// # Returns
///
/// A `HamResult` indicating whether to accept, reject, or defer the update.
pub fn ham(
    machine_state: f64,
    incoming_state: f64,
    current_state: f64,
    incoming_value: &GunValue,
    current_value: Option<&GunValue>,
) -> HamResult {
    // Validate states
    if incoming_state.is_nan() || current_state.is_nan() || machine_state.is_nan() {
        return HamResult::Error("invalid state: NaN");
    }
    if incoming_state.is_infinite() && incoming_state.is_sign_positive() {
        return HamResult::Error("invalid state: +Infinity");
    }

    // 1. Future: incoming state is ahead of our clock → defer
    if incoming_state > machine_state {
        return HamResult::Future(incoming_state - machine_state);
    }

    // 2. Old: incoming state is behind the current state for this key → discard
    if incoming_state < current_state {
        return HamResult::Old;
    }

    // 3. Same state: tie-break by JSON.stringify length
    //    From source: `if(state === was && (val === known || L(val) <= L(known))){ return }`
    //    L = JSON.stringify
    if (incoming_state - current_state).abs() < f64::EPSILON {
        match current_value {
            Some(current) => {
                // If values are equal, discard (no change needed)
                if incoming_value == current {
                    return HamResult::Same;
                }
                // Tie-break: larger serialized length wins
                let incoming_len = incoming_value.json_len();
                let current_len = current.json_len();
                if incoming_len <= current_len {
                    return HamResult::Same;
                }
                // incoming_len > current_len → accept
            }
            None => {
                // No current value → accept any incoming value
            }
        }
    }

    // 4. Accept: incoming state is strictly newer, or wins the tie-break
    HamResult::Accept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::GunValue;

    #[test]
    fn newer_state_wins() {
        let result = ham(
            100.0,                              // machine time
            50.0,                               // incoming state (newer than current)
            30.0,                               // current state
            &GunValue::Text("new".into()),
            Some(&GunValue::Text("old".into())),
        );
        assert_eq!(result, HamResult::Accept);
    }

    #[test]
    fn older_state_loses() {
        let result = ham(
            100.0,
            30.0,                               // incoming state (older than current)
            50.0,                               // current state
            &GunValue::Text("old".into()),
            Some(&GunValue::Text("current".into())),
        );
        assert_eq!(result, HamResult::Old);
    }

    #[test]
    fn future_state_deferred() {
        let result = ham(
            100.0,                              // machine time
            200.0,                              // incoming state is in the future
            50.0,
            &GunValue::Text("future".into()),
            Some(&GunValue::Text("current".into())),
        );
        assert_eq!(result, HamResult::Future(100.0));
    }

    #[test]
    fn same_state_same_value_discarded() {
        let result = ham(
            100.0,
            50.0,
            50.0,                               // same state
            &GunValue::Text("same".into()),     // same value
            Some(&GunValue::Text("same".into())),
        );
        assert_eq!(result, HamResult::Same);
    }

    #[test]
    fn same_state_larger_value_wins() {
        // "longer" (8 chars) serializes longer than "short" (7 chars)
        let result = ham(
            100.0,
            50.0,
            50.0,
            &GunValue::Text("longer!!".into()),  // json_len = 10
            Some(&GunValue::Text("short".into())), // json_len = 7
        );
        assert_eq!(result, HamResult::Accept);
    }

    #[test]
    fn same_state_smaller_value_loses() {
        let result = ham(
            100.0,
            50.0,
            50.0,
            &GunValue::Text("hi".into()),        // json_len = 4
            Some(&GunValue::Text("hello".into())), // json_len = 7
        );
        assert_eq!(result, HamResult::Same);
    }

    #[test]
    fn no_current_value_accepts() {
        let result = ham(
            100.0,
            50.0,
            f64::NEG_INFINITY,                   // no prior state
            &GunValue::Text("new".into()),
            None,                                // no current value
        );
        assert_eq!(result, HamResult::Accept);
    }

    #[test]
    fn nan_state_is_error() {
        let result = ham(
            100.0,
            f64::NAN,
            50.0,
            &GunValue::Null,
            None,
        );
        assert!(matches!(result, HamResult::Error(_)));
    }

    #[test]
    fn null_tombstone_accepted_when_newer() {
        let result = ham(
            100.0,
            60.0,                               // newer than current
            50.0,
            &GunValue::Null,                    // tombstone
            Some(&GunValue::Text("alive".into())),
        );
        assert_eq!(result, HamResult::Accept);
    }

    #[test]
    fn link_value_accepted_when_newer() {
        let result = ham(
            100.0,
            60.0,
            50.0,
            &GunValue::Link("other-soul".into()),
            Some(&GunValue::Text("was text".into())),
        );
        assert_eq!(result, HamResult::Accept);
    }
}
