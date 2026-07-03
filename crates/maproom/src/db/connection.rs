//! Database connection URL resolution for SQLite.
//!
//! This module provides automatic database URL detection:
//! - Explicit MAPROOM_DATABASE_URL (highest priority)
//! - SQLite at ~/.maproom/maproom.db (default)

use anyhow::Result;
use std::env;
use std::path::PathBuf;
use tracing::debug;

/// Get the default SQLite database path.
///
/// Returns `~/.maproom/maproom.db` on all platforms.
pub fn get_default_sqlite_path() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    Ok(home.join(".maproom").join("maproom.db"))
}

/// Expand tilde (~) in a path to the user's home directory.
///
/// Only expands a leading ~ followed by / or end of string.
/// Returns the original string if no expansion is needed.
fn expand_tilde(path: &str) -> Result<String> {
    if path.starts_with("~/") {
        let home = dirs::home_dir().ok_or_else(|| {
            anyhow::anyhow!("Could not determine home directory for tilde expansion")
        })?;
        Ok(format!("{}{}", home.display(), &path[1..]))
    } else if path == "~" {
        let home = dirs::home_dir().ok_or_else(|| {
            anyhow::anyhow!("Could not determine home directory for tilde expansion")
        })?;
        Ok(home.display().to_string())
    } else {
        Ok(path.to_string())
    }
}

/// Get database connection URL.
///
/// Priority order:
/// 1. MAPROOM_DATABASE_URL env var (explicit config)
/// 2. ~/.maproom/maproom.db (default SQLite database)
///
/// Tilde (~) is expanded to the user's home directory in explicit URLs.
///
/// # Examples
///
/// ```no_run
/// use maproom::db::connection::get_database_url;
///
/// let url = get_database_url().expect("Failed to get database URL");
/// println!("Connecting to: {}", url);
/// ```
pub fn get_database_url() -> Result<String> {
    // 1. Check for explicit MAPROOM_DATABASE_URL (highest priority)
    if let Ok(url) = env::var("MAPROOM_DATABASE_URL") {
        // Review H5: trim ONCE and use the trimmed value throughout. The old
        // code trimmed only for the emptiness check and returned the raw
        // string — a LEADING space defeated backend_for_url's starts_with
        // scheme detection, silently selecting SQLite with exit 0 (bypassing
        // the F70 postgres guard) and turning the DSN (credentials included)
        // into an on-disk directory name.
        let url = url.trim();
        // R12 / R-URL-1: an empty URL used to fall through to rusqlite's
        // empty-filename behavior — a private temp DB deleted on close, so
        // every write was silently discarded with exit 0.
        if url.is_empty() {
            anyhow::bail!("Configuration error: MAPROOM_DATABASE_URL / --database-url is empty");
        }
        debug!("Using explicit MAPROOM_DATABASE_URL from environment");

        // Expand tilde in the path portion of sqlite:// URLs
        if let Some(path) = url.strip_prefix("sqlite://") {
            let expanded_path = expand_tilde(path)?;
            return Ok(format!("sqlite://{}", expanded_path));
        }

        return Ok(url.to_string());
    }

    // 2. Default to SQLite at ~/.maproom/maproom.db
    let sqlite_path = get_default_sqlite_path()?;
    let url = format!("sqlite://{}", sqlite_path.display());

    if sqlite_path.exists() {
        debug!(
            "Found existing SQLite database at {}",
            sqlite_path.display()
        );
    } else {
        debug!(
            "SQLite database will be created at {}",
            sqlite_path.display()
        );
    }

    Ok(url)
}

#[cfg(test)]
mod tests {
    #[test]
    #[serial_test::serial]
    fn get_database_url_trims_whitespace() {
        // Review H5: ' postgres://...' must classify as Postgres, not SQLite.
        std::env::set_var("MAPROOM_DATABASE_URL", "  postgres://u@h/db  ");
        let url = get_database_url().unwrap();
        assert_eq!(url, "postgres://u@h/db");
        assert_eq!(backend_for_url(&url), Backend::Postgres);
        std::env::remove_var("MAPROOM_DATABASE_URL");
    }

    #[test]
    #[serial_test::serial]
    fn get_database_url_rejects_empty() {
        // R12 / R-URL-1
        std::env::set_var("MAPROOM_DATABASE_URL", "   ");
        let err = get_database_url().unwrap_err();
        assert!(err.to_string().contains("empty"), "got: {err}");
        std::env::remove_var("MAPROOM_DATABASE_URL");
    }

    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_explicit_database_url_takes_precedence() {
        // Setup: Set env var
        env::set_var("MAPROOM_DATABASE_URL", "sqlite:///custom/path/maproom.db");

        let url = get_database_url().unwrap();

        assert_eq!(url, "sqlite:///custom/path/maproom.db");

        // Cleanup
        env::remove_var("MAPROOM_DATABASE_URL");
    }

    #[test]
    #[serial]
    fn test_default_sqlite_path() {
        env::remove_var("MAPROOM_DATABASE_URL");

        let url = get_database_url().unwrap();

        // Should be SQLite URL with home directory
        assert!(
            url.starts_with("sqlite://"),
            "Expected sqlite:// URL, got: {}",
            url
        );
        assert!(
            url.contains(".maproom/maproom.db"),
            "Expected .maproom/maproom.db path, got: {}",
            url
        );
    }

    #[test]
    #[serial]
    fn test_tilde_expansion_in_database_url() {
        // Setup: Set env var with tilde
        env::set_var("MAPROOM_DATABASE_URL", "sqlite://~/.maproom/maproom.db");

        let url = get_database_url().unwrap();

        // Should NOT contain tilde after expansion
        assert!(!url.contains("~"), "Tilde should be expanded, got: {}", url);
        // Should contain .maproom/maproom.db
        assert!(
            url.contains(".maproom/maproom.db"),
            "Expected .maproom/maproom.db path, got: {}",
            url
        );
        // Should start with sqlite://
        assert!(
            url.starts_with("sqlite://"),
            "Expected sqlite:// URL, got: {}",
            url
        );

        // Cleanup
        env::remove_var("MAPROOM_DATABASE_URL");
    }

    #[test]
    fn test_expand_tilde() {
        // Test tilde at start with slash
        let expanded = expand_tilde("~/test/path").unwrap();
        assert!(!expanded.contains("~"), "Tilde should be expanded");
        assert!(
            expanded.ends_with("/test/path"),
            "Path suffix should be preserved"
        );

        // Test no tilde
        let no_tilde = expand_tilde("/absolute/path").unwrap();
        assert_eq!(no_tilde, "/absolute/path");

        // Test tilde in middle (should not expand)
        let middle_tilde = expand_tilde("/path/~test").unwrap();
        assert_eq!(middle_tilde, "/path/~test");
    }

    #[test]
    fn test_get_default_sqlite_path() {
        let path = get_default_sqlite_path().unwrap();

        assert!(path.ends_with(".maproom/maproom.db") || path.ends_with(".maproom\\maproom.db"));
    }
}

/// Which storage backend a resolved database URL selects (R-WIRE-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Sqlite,
    Postgres,
}

/// Classify a resolved database URL by scheme. `postgres://` / `postgresql://`
/// select Postgres; everything else (including bare paths and `sqlite://`)
/// selects SQLite. `connect()` dispatches on this; `get_database_url()` is left
/// to resolve/normalize the DSN unchanged.
pub fn backend_for_url(url: &str) -> Backend {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        Backend::Postgres
    } else {
        Backend::Sqlite
    }
}

#[cfg(test)]
mod backend_for_url_tests {
    use super::{backend_for_url, Backend};

    #[test]
    fn classifies_schemes() {
        assert_eq!(backend_for_url("postgres://h/db"), Backend::Postgres);
        assert_eq!(backend_for_url("postgresql://h/db"), Backend::Postgres);
        assert_eq!(backend_for_url("sqlite:///tmp/x.db"), Backend::Sqlite);
        assert_eq!(
            backend_for_url("/home/u/.maproom/maproom.db"),
            Backend::Sqlite
        );
    }
}
