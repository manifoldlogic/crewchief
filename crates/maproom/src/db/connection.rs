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
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    Ok(home.join(".maproom").join("maproom.db"))
}

/// Get database connection URL.
///
/// Priority order:
/// 1. MAPROOM_DATABASE_URL env var (explicit config)
/// 2. ~/.maproom/maproom.db (default SQLite database)
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
    // 1. Check for explicit MAPROOM_DATABASE_URL (highest priority)
    if let Ok(url) = env::var("MAPROOM_DATABASE_URL") {
        debug!("Using explicit MAPROOM_DATABASE_URL from environment");
        return Ok(url);
    }

    // 2. Default to SQLite at ~/.maproom/maproom.db
    let sqlite_path = get_default_sqlite_path()?;
    let url = format!("sqlite://{}", sqlite_path.display());

    if sqlite_path.exists() {
        debug!("Found existing SQLite database at {}", sqlite_path.display());
    } else {
        debug!("SQLite database will be created at {}", sqlite_path.display());
    }

    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_explicit_database_url_takes_precedence() {
        // Setup: Set env var
        env::set_var(
            "MAPROOM_DATABASE_URL",
            "sqlite:///custom/path/maproom.db",
        );

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
        assert!(url.starts_with("sqlite://"), "Expected sqlite:// URL, got: {}", url);
        assert!(url.contains(".maproom/maproom.db"), "Expected .maproom/maproom.db path, got: {}", url);
    }

    #[test]
    fn test_get_default_sqlite_path() {
        let path = get_default_sqlite_path().unwrap();

        assert!(path.ends_with(".maproom/maproom.db") || path.ends_with(".maproom\\maproom.db"));
    }
}
