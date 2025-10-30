/// MD_ENHANCE-4002: Performance Integration Tests
///
/// This test file includes all performance tests from the integration module.

mod integration;

// Re-export all performance tests
#[cfg(test)]
mod performance_tests {
    use super::integration::performance_test::*;
}
