//! Rust edge-extraction fixture (spec F-A).
//! Ground truth (same-file `calls` edges):
//!   alpha -> beta
//!   Calculator::multiply -> Calculator::add   (src must be the METHOD chunk)
//!   test_alpha -> alpha                       (also a test_of source, F-B)

pub fn beta() -> i32 {
    42
}

pub fn alpha() -> i32 {
    beta() + 1
}

pub struct Calculator;

impl Calculator {
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    pub fn multiply(&self, a: i32, b: i32) -> i32 {
        let mut acc = 0;
        for _ in 0..b {
            acc = self.add(acc, a);
        }
        acc
    }
}

pub fn generic_probe<T: Default>() -> T {
    T::default()
}

pub fn no_edges_here() {
    println!("macro only");
    let _x = generic_probe::<i32>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha() {
        let result = alpha();
        assert_eq!(result, 43);
    }
}
