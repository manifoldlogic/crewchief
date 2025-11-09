//! Heuristics for intelligent context assembly.
//!
//! This module provides pattern-based detection and scoring heuristics that enhance
//! context assembly quality:
//! - **Test file detection**: Identify test files via filename patterns
//! - **Config file detection**: Identify configuration files that provide essential context
//! - **Heuristic weights**: Configurable multipliers for different file types
//!
//! These heuristics work alongside the importance scoring system to ensure tests
//! and config files are appropriately prioritized during context assembly.

use regex::Regex;
use std::path::Path;

/// Configuration for heuristic scoring weights and patterns.
#[derive(Debug, Clone)]
pub struct HeuristicsConfig {
    /// Multiplier for test files (default: 1.5)
    pub test_weight: f64,
    /// Multiplier for config files (default: 1.1)
    pub config_weight: f64,
    /// Patterns for detecting test files
    pub test_patterns: Vec<String>,
    /// Patterns for detecting config files
    pub config_patterns: Vec<String>,
}

impl Default for HeuristicsConfig {
    fn default() -> Self {
        Self {
            test_weight: 1.5,
            config_weight: 1.1,
            test_patterns: vec![
                r"\.test\.(ts|js|tsx|jsx|rs|go|py)$".to_string(),
                r"\.spec\.(ts|js|tsx|jsx|rs|go|py)$".to_string(),
                r"__tests__".to_string(), // Match __tests__ anywhere in path
                r"^tests/".to_string(),   // Match tests/ at start of path
                r"/tests/".to_string(),   // Match /tests/ anywhere in path
                r"_test\.(ts|js|tsx|jsx|rs|go|py)$".to_string(),
            ],
            config_patterns: vec![
                r"^package\.json$".to_string(),
                r"^tsconfig\.json$".to_string(),
                r"^jsconfig\.json$".to_string(),
                r"\.config\.(ts|js|json|yaml|yml|toml)$".to_string(),
                r"^\.env(\..+)?$".to_string(),
                r"^Cargo\.toml$".to_string(),
                r"^go\.mod$".to_string(),
                r"^pyproject\.toml$".to_string(),
                r"^setup\.py$".to_string(),
            ],
        }
    }
}

impl HeuristicsConfig {
    /// Create a new heuristics configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the test weight multiplier.
    pub fn with_test_weight(mut self, weight: f64) -> Self {
        self.test_weight = weight;
        self
    }

    /// Set the config weight multiplier.
    pub fn with_config_weight(mut self, weight: f64) -> Self {
        self.config_weight = weight;
        self
    }

    /// Add a custom test pattern.
    pub fn add_test_pattern(mut self, pattern: String) -> Self {
        self.test_patterns.push(pattern);
        self
    }

    /// Add a custom config pattern.
    pub fn add_config_pattern(mut self, pattern: String) -> Self {
        self.config_patterns.push(pattern);
        self
    }
}

/// Type of file detected by heuristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Test file
    Test,
    /// Configuration file
    Config,
    /// Regular code file (no special type)
    Regular,
}

/// Heuristic scorer that detects file types and applies appropriate weights.
pub struct HeuristicScorer {
    config: HeuristicsConfig,
    test_patterns: Vec<Regex>,
    config_patterns: Vec<Regex>,
}

impl HeuristicScorer {
    /// Create a new heuristic scorer with default configuration.
    pub fn new() -> Self {
        Self::with_config(HeuristicsConfig::default())
    }

    /// Create a new heuristic scorer with custom configuration.
    pub fn with_config(config: HeuristicsConfig) -> Self {
        // Compile all regex patterns
        let test_patterns = config
            .test_patterns
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        let config_patterns = config
            .config_patterns
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        Self {
            config,
            test_patterns,
            config_patterns,
        }
    }

    /// Detect the file type based on the file path.
    ///
    /// # Arguments
    /// * `file_path` - Relative path to the file
    ///
    /// # Returns
    /// FileType indicating whether this is a test, config, or regular file
    ///
    /// # Example
    /// ```ignore
    /// let scorer = HeuristicScorer::new();
    /// assert_eq!(scorer.detect_file_type("src/handler.test.ts"), FileType::Test);
    /// assert_eq!(scorer.detect_file_type("package.json"), FileType::Config);
    /// assert_eq!(scorer.detect_file_type("src/handler.ts"), FileType::Regular);
    /// ```
    pub fn detect_file_type(&self, file_path: &str) -> FileType {
        // Normalize path separators for consistent matching
        let normalized_path = file_path.replace('\\', "/");

        // Check test patterns first (more specific)
        for pattern in &self.test_patterns {
            if pattern.is_match(&normalized_path) {
                return FileType::Test;
            }
        }

        // Extract filename for config detection
        let file_name = Path::new(&normalized_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Check config patterns
        for pattern in &self.config_patterns {
            if pattern.is_match(file_name) || pattern.is_match(&normalized_path) {
                return FileType::Config;
            }
        }

        FileType::Regular
    }

    /// Check if the file is a test file.
    pub fn is_test_file(&self, file_path: &str) -> bool {
        self.detect_file_type(file_path) == FileType::Test
    }

    /// Check if the file is a config file.
    pub fn is_config_file(&self, file_path: &str) -> bool {
        self.detect_file_type(file_path) == FileType::Config
    }

    /// Apply heuristic weight multiplier based on file type.
    ///
    /// # Arguments
    /// * `base_score` - The base importance score
    /// * `file_path` - Relative path to the file
    ///
    /// # Returns
    /// Score with heuristic weight applied
    ///
    /// # Example
    /// ```ignore
    /// let scorer = HeuristicScorer::new();
    /// let base_score = 1.0;
    /// let test_score = scorer.apply_heuristic_weight(base_score, "handler.test.ts");
    /// // test_score = 1.0 * 1.5 = 1.5
    /// ```
    pub fn apply_heuristic_weight(&self, base_score: f64, file_path: &str) -> f64 {
        match self.detect_file_type(file_path) {
            FileType::Test => base_score * self.config.test_weight,
            FileType::Config => base_score * self.config.config_weight,
            FileType::Regular => base_score,
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &HeuristicsConfig {
        &self.config
    }
}

impl Default for HeuristicScorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heuristics_config_defaults() {
        let config = HeuristicsConfig::default();
        assert_eq!(config.test_weight, 1.5);
        assert_eq!(config.config_weight, 1.1);
        assert!(!config.test_patterns.is_empty());
        assert!(!config.config_patterns.is_empty());
    }

    #[test]
    fn test_heuristics_config_builder() {
        let config = HeuristicsConfig::new()
            .with_test_weight(2.0)
            .with_config_weight(1.2)
            .add_test_pattern(r"\.custom_test\.ts$".to_string())
            .add_config_pattern(r"^custom\.config\.json$".to_string());

        assert_eq!(config.test_weight, 2.0);
        assert_eq!(config.config_weight, 1.2);
        assert!(config
            .test_patterns
            .contains(&r"\.custom_test\.ts$".to_string()));
        assert!(config
            .config_patterns
            .contains(&r"^custom\.config\.json$".to_string()));
    }

    #[test]
    fn test_detect_typescript_test_files() {
        let scorer = HeuristicScorer::new();

        // .test. pattern
        assert_eq!(
            scorer.detect_file_type("src/handler.test.ts"),
            FileType::Test
        );
        assert_eq!(
            scorer.detect_file_type("src/component.test.tsx"),
            FileType::Test
        );
        assert_eq!(scorer.detect_file_type("src/utils.test.js"), FileType::Test);

        // .spec. pattern
        assert_eq!(
            scorer.detect_file_type("src/handler.spec.ts"),
            FileType::Test
        );
        assert_eq!(
            scorer.detect_file_type("src/component.spec.tsx"),
            FileType::Test
        );

        // __tests__ directory
        assert_eq!(
            scorer.detect_file_type("src/__tests__/handler.ts"),
            FileType::Test
        );
        assert_eq!(
            scorer.detect_file_type("__tests__/integration.test.ts"),
            FileType::Test
        );

        // /tests/ directory
        assert_eq!(
            scorer.detect_file_type("tests/unit/handler.ts"),
            FileType::Test
        );

        // _test suffix
        assert_eq!(
            scorer.detect_file_type("src/handler_test.ts"),
            FileType::Test
        );
    }

    #[test]
    fn test_detect_rust_test_files() {
        let scorer = HeuristicScorer::new();

        assert_eq!(scorer.detect_file_type("src/lib_test.rs"), FileType::Test);
        assert_eq!(
            scorer.detect_file_type("src/parser.test.rs"),
            FileType::Test
        );
        assert_eq!(
            scorer.detect_file_type("tests/integration.rs"),
            FileType::Test
        );
    }

    #[test]
    fn test_detect_config_files() {
        let scorer = HeuristicScorer::new();

        // npm/node
        assert_eq!(scorer.detect_file_type("package.json"), FileType::Config);
        assert_eq!(scorer.detect_file_type("tsconfig.json"), FileType::Config);
        assert_eq!(scorer.detect_file_type("jsconfig.json"), FileType::Config);

        // Various config file patterns
        assert_eq!(scorer.detect_file_type("vite.config.ts"), FileType::Config);
        assert_eq!(
            scorer.detect_file_type("webpack.config.js"),
            FileType::Config
        );
        assert_eq!(
            scorer.detect_file_type("jest.config.json"),
            FileType::Config
        );

        // Environment files
        assert_eq!(scorer.detect_file_type(".env"), FileType::Config);
        assert_eq!(scorer.detect_file_type(".env.local"), FileType::Config);
        assert_eq!(scorer.detect_file_type(".env.production"), FileType::Config);

        // Rust
        assert_eq!(scorer.detect_file_type("Cargo.toml"), FileType::Config);

        // Go
        assert_eq!(scorer.detect_file_type("go.mod"), FileType::Config);

        // Python
        assert_eq!(scorer.detect_file_type("pyproject.toml"), FileType::Config);
        assert_eq!(scorer.detect_file_type("setup.py"), FileType::Config);
    }

    #[test]
    fn test_detect_regular_files() {
        let scorer = HeuristicScorer::new();

        assert_eq!(scorer.detect_file_type("src/handler.ts"), FileType::Regular);
        assert_eq!(scorer.detect_file_type("src/lib.rs"), FileType::Regular);
        assert_eq!(
            scorer.detect_file_type("src/component.tsx"),
            FileType::Regular
        );
        assert_eq!(scorer.detect_file_type("README.md"), FileType::Regular);
    }

    #[test]
    fn test_is_test_file() {
        let scorer = HeuristicScorer::new();

        assert!(scorer.is_test_file("src/handler.test.ts"));
        assert!(scorer.is_test_file("src/__tests__/integration.ts"));
        assert!(!scorer.is_test_file("src/handler.ts"));
        assert!(!scorer.is_test_file("package.json"));
    }

    #[test]
    fn test_is_config_file() {
        let scorer = HeuristicScorer::new();

        assert!(scorer.is_config_file("package.json"));
        assert!(scorer.is_config_file("tsconfig.json"));
        assert!(scorer.is_config_file(".env"));
        assert!(!scorer.is_config_file("src/handler.ts"));
        assert!(!scorer.is_config_file("src/handler.test.ts"));
    }

    #[test]
    fn test_apply_heuristic_weight_test_files() {
        let scorer = HeuristicScorer::new();

        let base_score = 1.0;
        let test_score = scorer.apply_heuristic_weight(base_score, "handler.test.ts");

        // Default test weight is 1.5
        assert!((test_score - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_apply_heuristic_weight_config_files() {
        let scorer = HeuristicScorer::new();

        let base_score = 1.0;
        let config_score = scorer.apply_heuristic_weight(base_score, "package.json");

        // Default config weight is 1.1
        assert!((config_score - 1.1).abs() < 0.01);
    }

    #[test]
    fn test_apply_heuristic_weight_regular_files() {
        let scorer = HeuristicScorer::new();

        let base_score = 1.0;
        let regular_score = scorer.apply_heuristic_weight(base_score, "handler.ts");

        // Regular files get no boost
        assert!((regular_score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_custom_test_weight() {
        let config = HeuristicsConfig::new().with_test_weight(2.0);
        let scorer = HeuristicScorer::with_config(config);

        let base_score = 1.0;
        let test_score = scorer.apply_heuristic_weight(base_score, "handler.test.ts");

        assert!((test_score - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_custom_config_weight() {
        let config = HeuristicsConfig::new().with_config_weight(1.5);
        let scorer = HeuristicScorer::with_config(config);

        let base_score = 1.0;
        let config_score = scorer.apply_heuristic_weight(base_score, "package.json");

        assert!((config_score - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_custom_patterns() {
        let config = HeuristicsConfig::new()
            .add_test_pattern(r"\.custom_test\.ts$".to_string())
            .add_config_pattern(r"^myapp\.config\.json$".to_string());

        let scorer = HeuristicScorer::with_config(config);

        // Custom test pattern should match
        assert!(scorer.is_test_file("src/handler.custom_test.ts"));

        // Custom config pattern should match
        assert!(scorer.is_config_file("myapp.config.json"));
    }

    #[test]
    fn test_path_normalization() {
        let scorer = HeuristicScorer::new();

        // Test with backslashes (Windows-style paths)
        assert_eq!(
            scorer.detect_file_type("src\\handler.test.ts"),
            FileType::Test
        );
        assert_eq!(
            scorer.detect_file_type("src\\__tests__\\integration.ts"),
            FileType::Test
        );
    }

    #[test]
    fn test_nested_paths() {
        let scorer = HeuristicScorer::new();

        // Deeply nested test files
        assert_eq!(
            scorer.detect_file_type("src/modules/auth/handlers/login.test.ts"),
            FileType::Test
        );

        // Config files in subdirectories
        assert_eq!(
            scorer.detect_file_type("configs/jest.config.js"),
            FileType::Config
        );
    }

    #[test]
    fn test_combined_scoring_scenario() {
        let scorer = HeuristicScorer::new();

        // Scenario: Score different file types
        let base_score = 1.0;

        let test_file = "src/handler.test.ts";
        let config_file = "package.json";
        let regular_file = "src/handler.ts";

        let test_score = scorer.apply_heuristic_weight(base_score, test_file);
        let config_score = scorer.apply_heuristic_weight(base_score, config_file);
        let regular_score = scorer.apply_heuristic_weight(base_score, regular_file);

        // Test files should score highest
        assert!(test_score > config_score);
        assert!(test_score > regular_score);

        // Config files should score higher than regular
        assert!(config_score > regular_score);
    }

    #[test]
    fn test_invalid_regex_patterns_are_skipped() {
        let config = HeuristicsConfig::new().add_test_pattern("[invalid(regex".to_string());

        // Should not panic, just skip invalid patterns
        let scorer = HeuristicScorer::with_config(config);

        // Valid patterns should still work
        assert!(scorer.is_test_file("handler.test.ts"));
    }
}
