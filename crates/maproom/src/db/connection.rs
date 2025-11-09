//! Database connection URL resolution with fallback logic.
//!
//! This module provides automatic database URL detection for different environments:
//! - Explicit DATABASE_URL (highest priority)
//! - MAPROOM_DB_HOST component-based configuration
//! - maproom-postgres hostname auto-detection
//! - localhost fallback (development)

use anyhow::Result;
use std::env;
use std::process::Command;
use tracing::debug;

/// Get database connection URL with fallback logic.
///
/// Priority order:
/// 1. DATABASE_URL env var (explicit config)
/// 2. MAPROOM_DB_HOST env var (component-based config)
/// 3. maproom-postgres hostname resolution (auto-detect)
/// 4. localhost fallback (development)
///
/// # Examples
///
/// ```no_run
/// use crewchief_maproom::db::connection::get_database_url;
///
/// let url = get_database_url().expect("Failed to get database URL");
/// println!("Connecting to: {}", url);
/// ```
pub fn get_database_url() -> Result<String> {
    // 1. Check for explicit DATABASE_URL
    if let Ok(url) = env::var("DATABASE_URL") {
        debug!("Using explicit DATABASE_URL from environment");
        return Ok(url);
    }

    // 2. Check for MAPROOM_DB_HOST component-based config
    if let Ok(host) = env::var("MAPROOM_DB_HOST") {
        let port = env::var("MAPROOM_DB_PORT").unwrap_or_else(|_| "5432".to_string());
        let url = format!("postgresql://maproom:maproom@{}:{}/maproom", host, port);
        debug!("Using MAPROOM_DB_HOST: {}", host);
        return Ok(url);
    }

    // 3. Try to resolve maproom-postgres hostname
    if can_resolve_hostname("maproom-postgres") {
        let url = "postgresql://maproom:maproom@maproom-postgres:5432/maproom".to_string();
        debug!("Auto-detected maproom-postgres hostname");
        return Ok(url);
    }

    // 4. Fallback to localhost
    debug!("Falling back to localhost:5433");
    Ok("postgresql://maproom:maproom@127.0.0.1:5433/maproom".to_string())
}

/// Check if a hostname can be resolved via DNS.
///
/// Uses `getent hosts` on Linux/Unix, `ping` as fallback.
/// Times out after 1 second to avoid hanging.
fn can_resolve_hostname(hostname: &str) -> bool {
    // Try getent hosts first (works on Linux)
    let getent_result = Command::new("getent").args(&["hosts", hostname]).output();

    if let Ok(output) = getent_result {
        if output.status.success() {
            return true;
        }
    }

    // Fallback: try ping with 1 packet, 1 second timeout
    let ping_result = Command::new("ping")
        .args(&["-c", "1", "-W", "1", hostname])
        .output();

    if let Ok(output) = ping_result {
        return output.status.success();
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_explicit_database_url_takes_precedence() {
        // Setup: Set all env vars
        env::set_var(
            "DATABASE_URL",
            "postgresql://explicit:pass@explicit-host:5432/explicit",
        );
        env::set_var("MAPROOM_DB_HOST", "should-be-ignored");

        let url = get_database_url().unwrap();

        assert_eq!(
            url,
            "postgresql://explicit:pass@explicit-host:5432/explicit"
        );

        // Cleanup
        env::remove_var("DATABASE_URL");
        env::remove_var("MAPROOM_DB_HOST");
    }

    #[test]
    #[serial]
    fn test_maproom_db_host_override() {
        env::remove_var("DATABASE_URL");
        env::set_var("MAPROOM_DB_HOST", "custom-postgres");
        env::set_var("MAPROOM_DB_PORT", "5555");

        let url = get_database_url().unwrap();

        assert_eq!(
            url,
            "postgresql://maproom:maproom@custom-postgres:5555/maproom"
        );

        env::remove_var("MAPROOM_DB_HOST");
        env::remove_var("MAPROOM_DB_PORT");
    }

    #[test]
    #[serial]
    fn test_maproom_db_host_default_port() {
        env::remove_var("DATABASE_URL");
        env::set_var("MAPROOM_DB_HOST", "custom-postgres");
        env::remove_var("MAPROOM_DB_PORT");

        let url = get_database_url().unwrap();

        assert_eq!(
            url,
            "postgresql://maproom:maproom@custom-postgres:5432/maproom"
        );

        env::remove_var("MAPROOM_DB_HOST");
    }

    #[test]
    #[serial]
    fn test_fallback_when_no_env_vars() {
        env::remove_var("DATABASE_URL");
        env::remove_var("MAPROOM_DB_HOST");

        let url = get_database_url().unwrap();

        // Should either detect maproom-postgres or fallback to localhost
        assert!(
            url.contains("maproom-postgres:5432") || url.contains("127.0.0.1:5433"),
            "Expected maproom-postgres or localhost, got: {}",
            url
        );
    }
}
