//! AWS request signing and credential resolution.
//!
//! A minimal, dependency-light implementation of the parts of the AWS protocol
//! stack that the Bedrock embedding provider needs:
//!
//! - [`sigv4`] — `AWS4-HMAC-SHA256` request signing.
//! - [`credentials`] — the standard credential provider chain.
//! - [`profile`] — parsing for `~/.aws/config` and `~/.aws/credentials`.
//!
//! The AWS SDK was evaluated for this role and rejected; see the `hmac` entry in
//! `Cargo.toml` for the MSRV and dependency-weight reasoning.

pub mod credentials;
pub mod profile;
pub mod sigv4;

pub use credentials::{AwsCredentials, CredentialSource, CredentialsProvider};

use self::profile::ProfileSet;

/// Region used when nothing else specifies one.
///
/// `us-east-1` is where Bedrock launched and has the widest model availability,
/// so it is the least surprising default — but it is only reached when the
/// operator has configured no region at all, and the resolved value is logged.
pub const DEFAULT_REGION: &str = "us-east-1";

/// Resolve the AWS region to call Bedrock in.
///
/// Precedence, highest first:
///
/// 1. `MAPROOM_BEDROCK_REGION` — lets maproom embed in a region that has the
///    model while the rest of the environment's AWS tooling points elsewhere.
/// 2. `AWS_REGION` — the SDK-standard variable, set by Lambda and ECS.
/// 3. `AWS_DEFAULT_REGION` — the CLI-standard variable.
/// 4. `region` in the selected shared-config profile.
/// 5. [`DEFAULT_REGION`].
pub fn resolve_region(profiles: &ProfileSet, profile_name: Option<&str>) -> String {
    for variable in ["MAPROOM_BEDROCK_REGION", "AWS_REGION", "AWS_DEFAULT_REGION"] {
        if let Some(region) = env_nonempty(variable) {
            tracing::debug!("Using AWS region {region} from {variable}");
            return region;
        }
    }

    if let Some(region) = profile_name
        .or(Some("default"))
        .and_then(|name| profiles.get(name, "region"))
    {
        tracing::debug!("Using AWS region {region} from shared config profile");
        return region.to_string();
    }

    tracing::debug!("No AWS region configured; defaulting to {DEFAULT_REGION}");
    DEFAULT_REGION.to_string()
}

/// Read an environment variable, treating whitespace-only values as unset.
fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Region resolution reads process-global environment state, so these tests
    // are serialized against each other and against the factory's env tests.
    use serial_test::serial;

    /// Clear every variable that participates in region resolution.
    fn clear_region_env() {
        for variable in ["MAPROOM_BEDROCK_REGION", "AWS_REGION", "AWS_DEFAULT_REGION"] {
            std::env::remove_var(variable);
        }
    }

    #[test]
    #[serial]
    fn maproom_region_wins_over_aws_region() {
        clear_region_env();
        std::env::set_var("AWS_REGION", "us-west-2");
        std::env::set_var("MAPROOM_BEDROCK_REGION", "eu-central-1");

        assert_eq!(
            resolve_region(&ProfileSet::default(), None),
            "eu-central-1",
            "maproom's override must beat the ambient AWS environment"
        );
        clear_region_env();
    }

    #[test]
    #[serial]
    fn aws_region_wins_over_aws_default_region() {
        clear_region_env();
        std::env::set_var("AWS_DEFAULT_REGION", "ap-southeast-2");
        std::env::set_var("AWS_REGION", "us-west-2");

        assert_eq!(resolve_region(&ProfileSet::default(), None), "us-west-2");
        clear_region_env();
    }

    #[test]
    #[serial]
    fn falls_back_to_default_region_when_nothing_is_configured() {
        clear_region_env();
        assert_eq!(resolve_region(&ProfileSet::default(), None), DEFAULT_REGION);
    }
}
