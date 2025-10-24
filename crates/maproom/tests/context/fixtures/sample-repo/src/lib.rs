//! Sample library for integration testing.
//!
//! This is a realistic Rust codebase used for testing the context assembly system.

pub mod utils;
pub mod api;

/// Main entry point for the library
pub fn run() {
    println!("Running sample application");
    let config = utils::load_config();
    api::process_request(&config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run() {
        run();
    }
}
