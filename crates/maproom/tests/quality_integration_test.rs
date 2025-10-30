/// MD_ENHANCE-4002: Quality Integration Tests
///
/// This test file includes all quality and performance tests from the integration module.

mod integration;

// Re-export all quality tests
#[cfg(test)]
mod quality_tests {
    use super::integration::quality_test::*;
}
