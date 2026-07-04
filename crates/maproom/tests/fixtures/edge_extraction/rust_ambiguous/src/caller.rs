//! Calls `multiply()`, which is defined in BOTH alpha.rs and beta.rs.
//! The precision-first ambiguity policy (spec B3) must emit NO edge for this call.

pub fn run() -> i32 {
    multiply()
}
