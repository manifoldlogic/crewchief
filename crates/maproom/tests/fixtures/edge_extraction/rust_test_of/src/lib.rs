//! test_of derivation fixture (spec B7/B8).
//!
//! `test_alpha` (a test) and `beta` (not a test) both call `alpha`. Only the
//! test caller yields a `test_of` edge; both yield `calls` edges.

pub fn alpha() -> i32 {
    43
}

/// A non-test function that also calls alpha — a `calls` edge, never `test_of`.
pub fn beta() -> i32 {
    alpha() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha() {
        // Call OUTSIDE a macro: assert_eq!(alpha(), ..) would hide the call in a
        // macro token-tree (documented out-of-scope for extraction).
        let result = alpha();
        assert_eq!(result, 43);
    }
}
