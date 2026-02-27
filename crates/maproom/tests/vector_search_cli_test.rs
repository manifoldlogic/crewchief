//! Integration tests for vector-search CLI command
//!
//! VECSRCH-3001: Integration Testing
//!
//! These tests verify vector search functionality end-to-end.
//! Requirements:
//! - Running PostgreSQL with pgvector extension
//! - MAPROOM_DATABASE_URL environment variable
//! - OPENAI_API_KEY environment variable
//! - Indexed repository with generated embeddings

// TODO: Update to use cargo::cargo_bin_cmd! macro
#![allow(deprecated)]

#[cfg(test)]
mod vector_search_cli_tests {
    use assert_cmd::Command;
    use predicates::prelude::*;
    use serde_json::Value;

    /// Test that vector-search command exists and shows help
    #[test]
    fn test_vector_search_help() {
        let mut cmd = Command::cargo_bin("maproom").unwrap();
        cmd.arg("vector-search").arg("--help");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("vector-search"))
            .stdout(predicate::str::contains("--query"))
            .stdout(predicate::str::contains("--repo"))
            .stdout(predicate::str::contains("--k"))
            .stdout(predicate::str::contains("--threshold"));
    }

    /// Test that vector-search returns valid JSON
    #[test]
    #[ignore] // Requires database setup
    fn test_vector_search_returns_json() {
        let mut cmd = Command::cargo_bin("maproom").unwrap();
        cmd.arg("vector-search")
            .arg("--repo")
            .arg("test-repo")
            .arg("--query")
            .arg("test query");

        let output = cmd.assert().success();

        // Verify output is valid JSON
        let stdout = std::str::from_utf8(&output.get_output().stdout).unwrap();
        let json: Value = serde_json::from_str(stdout).expect("Output should be valid JSON");

        // Verify JSON schema
        assert!(json.get("hits").is_some(), "JSON should have 'hits' field");
        assert!(
            json.get("total").is_some(),
            "JSON should have 'total' field"
        );
        assert!(
            json.get("query").is_some(),
            "JSON should have 'query' field"
        );
        assert!(json.get("mode").is_some(), "JSON should have 'mode' field");
        assert_eq!(json["mode"], "vector", "Mode should be 'vector'");
    }

    /// Test vector-search with search parameters
    #[test]
    #[ignore] // Requires database setup
    fn test_vector_search_with_parameters() {
        let mut cmd = Command::cargo_bin("maproom").unwrap();
        cmd.arg("vector-search")
            .arg("--repo")
            .arg("test-repo")
            .arg("--worktree")
            .arg("main")
            .arg("--query")
            .arg("authentication function")
            .arg("--k")
            .arg("5")
            .arg("--threshold")
            .arg("0.7");

        let output = cmd.assert().success();

        let stdout = std::str::from_utf8(&output.get_output().stdout).unwrap();
        let json: Value = serde_json::from_str(stdout).unwrap();

        // Verify parameters in output
        assert_eq!(json["query"], "authentication function");
        assert_eq!(json["k"], 5);
        assert_eq!(json["threshold"], 0.7);

        // Verify all results meet threshold
        if let Some(hits) = json["hits"].as_array() {
            for hit in hits {
                let score = hit["score"].as_f64().unwrap();
                assert!(score >= 0.7, "All results should meet threshold");
            }
        }
    }

    /// Test that vector-search filters by worktree
    #[test]
    #[ignore] // Requires database setup
    fn test_vector_search_worktree_filter() {
        let mut cmd = Command::cargo_bin("maproom").unwrap();
        cmd.arg("vector-search")
            .arg("--repo")
            .arg("test-repo")
            .arg("--worktree")
            .arg("feature-branch")
            .arg("--query")
            .arg("test");

        cmd.assert().success();
    }

    /// Test vector-search error handling for missing repo
    #[test]
    #[ignore] // Requires database setup
    fn test_vector_search_missing_repo_error() {
        let mut cmd = Command::cargo_bin("maproom").unwrap();
        cmd.arg("vector-search")
            .arg("--repo")
            .arg("nonexistent-repo")
            .arg("--query")
            .arg("test");

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("not found"));
    }

    /// Test that hits contain required fields
    #[test]
    #[ignore] // Requires database setup and seeded data
    fn test_vector_search_hit_schema() {
        let mut cmd = Command::cargo_bin("maproom").unwrap();
        cmd.arg("vector-search")
            .arg("--repo")
            .arg("test-repo")
            .arg("--query")
            .arg("function");

        let output = cmd.assert().success();
        let stdout = std::str::from_utf8(&output.get_output().stdout).unwrap();
        let json: Value = serde_json::from_str(stdout).unwrap();

        if let Some(hits) = json["hits"].as_array() {
            assert!(
                !hits.is_empty(),
                "Should return at least one hit for seeded data"
            );

            for hit in hits {
                // Verify required fields
                assert!(hit.get("chunk_id").is_some(), "Hit should have chunk_id");
                assert!(hit.get("score").is_some(), "Hit should have score");
                assert!(hit.get("file_path").is_some(), "Hit should have file_path");
                assert!(
                    hit.get("start_line").is_some(),
                    "Hit should have start_line"
                );
                assert!(hit.get("end_line").is_some(), "Hit should have end_line");
                assert!(hit.get("kind").is_some(), "Hit should have kind");
                // symbol_name can be null, so just check it exists
                assert!(
                    hit.get("symbol_name").is_some(),
                    "Hit should have symbol_name field"
                );

                // Verify score is in valid range
                let score = hit["score"].as_f64().unwrap();
                assert!(
                    score >= 0.0 && score <= 1.0,
                    "Score should be between 0.0 and 1.0"
                );
            }
        }
    }
}
