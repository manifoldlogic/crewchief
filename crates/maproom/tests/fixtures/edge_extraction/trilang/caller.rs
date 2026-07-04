//! Rust caller + in-file test (cross-file call + test_of).

pub fn r_caller() -> i32 {
    r_helper() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_r_caller() {
        // Call outside a macro (assert_eq! would hide it in a token-tree).
        let x = r_caller();
        assert_eq!(x, 2);
    }
}
