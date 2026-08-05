//! Parsing for the AWS shared `config` and `credentials` files.
//!
//! These are INI-shaped files that AWS tooling writes to `~/.aws/config` and
//! `~/.aws/credentials`. Everything the credential chain needs — static keys,
//! `role_arn` chains, `credential_process`, SSO settings, and the default region
//! — is expressed here as flat key/value pairs inside named sections.
//!
//! # The two files differ
//!
//! In `credentials`, sections are bare profile names (`[dev]`). In `config`,
//! every profile except `default` carries a `profile ` prefix (`[profile dev]`),
//! and SSO sessions live in their own `[sso-session name]` sections. This module
//! normalizes both into the same [`ProfileSet`] keyed by plain profile name.
//!
//! # Precedence
//!
//! When a key appears in both files for the same profile, the `credentials` file
//! wins. That matches the AWS CLI and SDKs: `credentials` is the more specific,
//! secret-bearing file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A parsed set of profiles plus any `sso-session` sections.
#[derive(Debug, Default, Clone)]
pub struct ProfileSet {
    /// Profile name -> settings.
    profiles: HashMap<String, HashMap<String, String>>,
    /// `sso-session` name -> settings.
    sso_sessions: HashMap<String, HashMap<String, String>>,
}

impl ProfileSet {
    /// Load profiles from the default file locations, honoring the standard
    /// `AWS_CONFIG_FILE` / `AWS_SHARED_CREDENTIALS_FILE` overrides.
    ///
    /// Missing files are not an error — a machine using only environment
    /// variables or an instance role has no `~/.aws` at all.
    pub fn load() -> Self {
        let config_path = std::env::var_os("AWS_CONFIG_FILE")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join(".aws").join("config")));
        let credentials_path = std::env::var_os("AWS_SHARED_CREDENTIALS_FILE")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join(".aws").join("credentials")));

        let mut set = Self::default();
        if let Some(path) = config_path {
            set.merge_config_file(&path);
        }
        if let Some(path) = credentials_path {
            set.merge_credentials_file(&path);
        }
        set
    }

    /// Parse an `~/.aws/config`-shaped file, stripping `profile ` prefixes and
    /// routing `sso-session` sections aside.
    fn merge_config_file(&mut self, path: &Path) {
        let Ok(contents) = std::fs::read_to_string(path) else {
            tracing::debug!("No AWS config file at {}", path.display());
            return;
        };
        for (section, settings) in parse_ini(&contents) {
            if let Some(name) = section.strip_prefix("sso-session ") {
                self.sso_sessions
                    .entry(name.trim().to_string())
                    .or_default()
                    .extend(settings);
            } else {
                let name = section
                    .strip_prefix("profile ")
                    .map(str::trim)
                    .unwrap_or(section.as_str());
                self.profiles
                    .entry(name.to_string())
                    .or_default()
                    .extend(settings);
            }
        }
    }

    /// Parse an `~/.aws/credentials`-shaped file. Sections are bare profile
    /// names and their values override anything from `config`.
    fn merge_credentials_file(&mut self, path: &Path) {
        let Ok(contents) = std::fs::read_to_string(path) else {
            tracing::debug!("No AWS credentials file at {}", path.display());
            return;
        };
        for (section, settings) in parse_ini(&contents) {
            self.profiles.entry(section).or_default().extend(settings);
        }
    }

    /// Look up one `sso-session` section's settings.
    pub fn sso_session(&self, name: &str) -> Option<&HashMap<String, String>> {
        self.sso_sessions.get(name)
    }

    /// Read a single setting from a profile.
    pub fn get(&self, profile: &str, key: &str) -> Option<&str> {
        self.profiles
            .get(profile)
            .and_then(|settings| settings.get(key))
            .map(String::as_str)
    }

    /// Whether a profile exists at all — used to distinguish "profile absent"
    /// from "profile present but unusable", which need different error messages.
    pub fn has_profile(&self, name: &str) -> bool {
        self.profiles.contains_key(name)
    }

    /// Profile names, for diagnostics when a requested profile is missing.
    pub fn profile_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.profiles.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }
}

/// Parse INI text into `(section, settings)` pairs.
///
/// Keys are lowercased (AWS treats them case-insensitively); values keep their
/// case because they carry ARNs, URLs, and secrets. `#` and `;` start comments.
/// Lines before any section header, and malformed lines, are skipped rather than
/// failing the whole file — a stray line in `~/.aws/config` should not make
/// maproom unable to embed.
fn parse_ini(contents: &str) -> Vec<(String, HashMap<String, String>)> {
    let mut sections: Vec<(String, HashMap<String, String>)> = Vec::new();
    let mut current: Option<(String, HashMap<String, String>)> = None;

    for raw_line in contents.lines() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if let Some(finished) = current.take() {
                sections.push(finished);
            }
            current = Some((header.trim().to_string(), HashMap::new()));
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if let Some((_, settings)) = current.as_mut() {
            settings.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    if let Some(finished) = current {
        sections.push(finished);
    }
    sections
}

/// Strip a trailing comment.
///
/// AWS only treats `#`/`;` as a comment when it starts the line or follows
/// whitespace, so a `#` inside a secret or URL fragment survives.
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(byte, b'#' | b';') {
            let preceded_by_space = index == 0 || bytes[index - 1].is_ascii_whitespace();
            if preceded_by_space {
                return &line[..index];
            }
        }
    }
    line
}

/// Resolve the user's home directory.
fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections_and_lowercases_keys() {
        let parsed = parse_ini(
            "[default]\n\
             AWS_ACCESS_KEY_ID = AKIA123\n\
             aws_secret_access_key=secret\n",
        );
        assert_eq!(parsed.len(), 1);
        let (name, settings) = &parsed[0];
        assert_eq!(name, "default");
        assert_eq!(settings["aws_access_key_id"], "AKIA123");
        assert_eq!(settings["aws_secret_access_key"], "secret");
    }

    #[test]
    fn config_file_strips_profile_prefix() {
        let mut set = ProfileSet::default();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config");
        std::fs::write(
            &path,
            "[default]\nregion = us-east-1\n\n[profile dev]\nregion = eu-west-1\n",
        )
        .unwrap();
        set.merge_config_file(&path);

        assert_eq!(set.get("default", "region"), Some("us-east-1"));
        assert_eq!(
            set.get("dev", "region"),
            Some("eu-west-1"),
            "`[profile dev]` must be addressable as `dev`"
        );
    }

    #[test]
    fn sso_sessions_are_kept_separate_from_profiles() {
        let mut set = ProfileSet::default();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config");
        std::fs::write(
            &path,
            "[sso-session corp]\n\
             sso_start_url = https://corp.awsapps.com/start\n\
             sso_region = us-east-1\n",
        )
        .unwrap();
        set.merge_config_file(&path);

        assert!(
            !set.has_profile("corp"),
            "an sso-session is not a profile and must not be selectable as one"
        );
        assert_eq!(
            set.sso_session("corp").unwrap()["sso_start_url"],
            "https://corp.awsapps.com/start"
        );
    }

    #[test]
    fn credentials_file_overrides_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config");
        let credentials = dir.path().join("credentials");
        std::fs::write(
            &config,
            "[default]\naws_access_key_id = FROM_CONFIG\nregion = us-east-1\n",
        )
        .unwrap();
        std::fs::write(&credentials, "[default]\naws_access_key_id = FROM_CREDS\n").unwrap();

        let mut set = ProfileSet::default();
        set.merge_config_file(&config);
        set.merge_credentials_file(&credentials);

        assert_eq!(set.get("default", "aws_access_key_id"), Some("FROM_CREDS"));
        assert_eq!(
            set.get("default", "region"),
            Some("us-east-1"),
            "keys only present in config must survive the merge"
        );
    }

    #[test]
    fn comments_are_stripped_only_at_token_boundaries() {
        assert_eq!(strip_comment("key = value # trailing").trim(), "key = value");
        assert_eq!(strip_comment("# whole line").trim(), "");
        assert_eq!(strip_comment("key = value ; trailing").trim(), "key = value");
        assert_eq!(
            strip_comment("key = pa#ssword").trim(),
            "key = pa#ssword",
            "a '#' with no preceding space is part of the value"
        );
    }

    #[test]
    fn malformed_lines_do_not_abort_the_file() {
        let parsed = parse_ini("[default]\ngarbage line\nregion = us-east-1\n");
        assert_eq!(parsed[0].1["region"], "us-east-1");
    }

    #[test]
    fn keys_before_any_section_are_ignored() {
        let parsed = parse_ini("stray = value\n[default]\nregion = us-east-1\n");
        assert_eq!(parsed.len(), 1);
        assert!(!parsed[0].1.contains_key("stray"));
    }

    #[test]
    fn missing_files_are_not_an_error() {
        let mut set = ProfileSet::default();
        set.merge_config_file(Path::new("/nonexistent/aws/config"));
        set.merge_credentials_file(Path::new("/nonexistent/aws/credentials"));
        assert!(set.profile_names().is_empty());
    }

    #[test]
    fn values_containing_equals_are_preserved() {
        // Secret access keys and base64 tokens routinely contain '='.
        let parsed = parse_ini("[default]\naws_secret_access_key = abc==def=\n");
        assert_eq!(parsed[0].1["aws_secret_access_key"], "abc==def=");
    }
}
