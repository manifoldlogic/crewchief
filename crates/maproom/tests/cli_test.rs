//! CLI argument parsing tests for maproom binary.
//!
//! These tests verify that CLI arguments are parsed correctly, particularly
//! for the --provider flag which validates embedding provider names.

use clap::Parser;

/// Minimal CLI structure for testing argument parsing.
///
/// This mirrors the main CLI structure but only includes the fields
/// necessary for testing provider validation.
#[derive(Parser, Debug)]
#[command(name = "maproom-test")]
struct TestCli {
    #[command(subcommand)]
    command: TestCommands,
}

#[derive(clap::Subcommand, Debug)]
enum TestCommands {
    Scan {
        #[arg(long, value_parser = validate_provider)]
        provider: Option<String>,
    },
    Upsert {
        #[arg(long, value_parser = validate_provider)]
        provider: Option<String>,
    },
}

/// Validate provider name against supported providers.
///
/// Returns the provider name in lowercase if valid, or an error message if invalid.
fn validate_provider(s: &str) -> Result<String, String> {
    match s.to_lowercase().as_str() {
        "ollama" | "openai" | "google" => Ok(s.to_lowercase()),
        _ => Err(format!(
            "Invalid provider: '{}'. Supported providers: ollama, openai, google",
            s
        )),
    }
}

#[test]
fn test_validate_provider_ollama() {
    let result = validate_provider("ollama");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "ollama");
}

#[test]
fn test_validate_provider_openai() {
    let result = validate_provider("openai");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "openai");
}

#[test]
fn test_validate_provider_google() {
    let result = validate_provider("google");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "google");
}

#[test]
fn test_validate_provider_case_insensitive() {
    // Test uppercase
    let result = validate_provider("OLLAMA");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "ollama");

    // Test mixed case
    let result = validate_provider("OpenAI");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "openai");

    // Test uppercase Google
    let result = validate_provider("GOOGLE");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "google");
}

#[test]
fn test_validate_provider_invalid() {
    let result = validate_provider("invalid");
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("Invalid provider: 'invalid'"));
    assert!(err_msg.contains("ollama"));
    assert!(err_msg.contains("openai"));
    assert!(err_msg.contains("google"));
}

#[test]
fn test_validate_provider_empty() {
    let result = validate_provider("");
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("Invalid provider"));
}

#[test]
fn test_validate_provider_typo() {
    // Common typos should be rejected
    assert!(validate_provider("olama").is_err()); // missing 'l'
    assert!(validate_provider("opeanai").is_err()); // typo
    assert!(validate_provider("googel").is_err()); // typo
}

#[test]
fn test_scan_with_valid_provider() {
    let args = vec!["test", "scan", "--provider", "ollama"];
    let result = TestCli::try_parse_from(args);
    assert!(
        result.is_ok(),
        "Failed to parse valid provider: {:?}",
        result.err()
    );

    if let TestCommands::Scan { provider } = result.unwrap().command {
        assert_eq!(provider, Some("ollama".to_string()));
    } else {
        panic!("Expected Scan command");
    }
}

#[test]
fn test_scan_with_openai_provider() {
    let args = vec!["test", "scan", "--provider", "openai"];
    let result = TestCli::try_parse_from(args);
    assert!(
        result.is_ok(),
        "Failed to parse openai provider: {:?}",
        result.err()
    );

    if let TestCommands::Scan { provider } = result.unwrap().command {
        assert_eq!(provider, Some("openai".to_string()));
    } else {
        panic!("Expected Scan command");
    }
}

#[test]
fn test_scan_with_google_provider() {
    let args = vec!["test", "scan", "--provider", "google"];
    let result = TestCli::try_parse_from(args);
    assert!(
        result.is_ok(),
        "Failed to parse google provider: {:?}",
        result.err()
    );

    if let TestCommands::Scan { provider } = result.unwrap().command {
        assert_eq!(provider, Some("google".to_string()));
    } else {
        panic!("Expected Scan command");
    }
}

#[test]
fn test_scan_with_invalid_provider() {
    let args = vec!["test", "scan", "--provider", "invalid"];
    let result = TestCli::try_parse_from(args);
    assert!(result.is_err(), "Expected error for invalid provider");

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Invalid provider: 'invalid'"),
        "Error message should explain the problem: {}",
        err_msg
    );
    assert!(
        err_msg.contains("ollama") || err_msg.contains("openai") || err_msg.contains("google"),
        "Error message should list supported providers: {}",
        err_msg
    );
}

#[test]
fn test_scan_without_provider() {
    let args = vec!["test", "scan"];
    let result = TestCli::try_parse_from(args);
    assert!(
        result.is_ok(),
        "Should parse without provider flag: {:?}",
        result.err()
    );

    if let TestCommands::Scan { provider } = result.unwrap().command {
        assert_eq!(
            provider, None,
            "Provider should be None when flag not provided"
        );
    } else {
        panic!("Expected Scan command");
    }
}

#[test]
fn test_upsert_with_valid_provider() {
    let args = vec!["test", "upsert", "--provider", "ollama"];
    let result = TestCli::try_parse_from(args);
    assert!(
        result.is_ok(),
        "Failed to parse valid provider for upsert: {:?}",
        result.err()
    );

    if let TestCommands::Upsert { provider } = result.unwrap().command {
        assert_eq!(provider, Some("ollama".to_string()));
    } else {
        panic!("Expected Upsert command");
    }
}

#[test]
fn test_upsert_with_invalid_provider() {
    let args = vec!["test", "upsert", "--provider", "unknown"];
    let result = TestCli::try_parse_from(args);
    assert!(
        result.is_err(),
        "Expected error for invalid provider in upsert"
    );

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Invalid provider: 'unknown'"),
        "Error message should explain the problem: {}",
        err_msg
    );
}

#[test]
fn test_upsert_without_provider() {
    let args = vec!["test", "upsert"];
    let result = TestCli::try_parse_from(args);
    assert!(
        result.is_ok(),
        "Should parse upsert without provider flag: {:?}",
        result.err()
    );

    if let TestCommands::Upsert { provider } = result.unwrap().command {
        assert_eq!(
            provider, None,
            "Provider should be None when flag not provided"
        );
    } else {
        panic!("Expected Upsert command");
    }
}

#[test]
fn test_provider_normalization() {
    // Test that all case variations normalize to lowercase
    let test_cases = vec![
        ("ollama", "ollama"),
        ("Ollama", "ollama"),
        ("OLLAMA", "ollama"),
        ("openai", "openai"),
        ("OpenAI", "openai"),
        ("OPENAI", "openai"),
        ("google", "google"),
        ("Google", "google"),
        ("GOOGLE", "google"),
    ];

    for (input, expected) in test_cases {
        let result = validate_provider(input);
        assert!(result.is_ok(), "Failed to validate provider: {}", input);
        assert_eq!(
            result.unwrap(),
            expected,
            "Provider '{}' should normalize to '{}'",
            input,
            expected
        );
    }
}

#[test]
fn test_error_message_quality() {
    // Error messages should be helpful and actionable
    let result = validate_provider("aws");
    assert!(result.is_err());

    let err_msg = result.unwrap_err();

    // Should mention the invalid value
    assert!(
        err_msg.contains("aws"),
        "Error should mention the invalid provider name"
    );

    // Should list all valid options
    assert!(
        err_msg.contains("ollama"),
        "Error should list 'ollama' as valid option"
    );
    assert!(
        err_msg.contains("openai"),
        "Error should list 'openai' as valid option"
    );
    assert!(
        err_msg.contains("google"),
        "Error should list 'google' as valid option"
    );

    // Should be clear about the problem
    assert!(
        err_msg.contains("Invalid provider"),
        "Error should clearly state the problem"
    );
}
