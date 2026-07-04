//! Calls a function defined in a sibling file (cross-file resolution target).

pub fn caller_a() -> i32 {
    // helper_b is defined in helper.rs — resolved by the cross-file post-pass.
    helper_b() + 1
}
