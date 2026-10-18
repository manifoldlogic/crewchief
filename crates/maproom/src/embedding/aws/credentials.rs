//! AWS credential resolution.
//!
//! This is the piece that decides whether maproom is actually deployable inside
//! an enterprise. Signing a Bedrock request is mechanical; *obtaining* the
//! credentials to sign with is where real deployments differ — a laptop uses SSO,
//! CI uses static keys or OIDC, an EKS pod uses a projected service-account
//! token, an EC2 builder uses the instance role, and a regulated environment
//! routes all of it through `credential_process`.
//!
//! # Resolution order
//!
//! Sources are tried in the order below, which mirrors the AWS SDKs:
//!
//! 1. **Environment variables** — `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`
//!    (+ optional `AWS_SESSION_TOKEN`).
//! 2. **An explicitly named profile** — `MAPROOM_AWS_PROFILE` or `AWS_PROFILE`.
//!    If the operator named a profile and it fails, that is a hard error: silently
//!    falling through to an instance role would run against the wrong account.
//! 3. **Web identity from the environment** — `AWS_WEB_IDENTITY_TOKEN_FILE` +
//!    `AWS_ROLE_ARN`. This is how EKS IRSA and GitHub Actions OIDC arrive.
//! 4. **The `default` profile**, if one exists.
//! 5. **Container credentials** — ECS task roles and EKS Pod Identity, via
//!    `AWS_CONTAINER_CREDENTIALS_RELATIVE_URI` / `_FULL_URI`.
//! 6. **EC2 instance metadata (IMDSv2)**.
//!
//! A profile itself may resolve through static keys, `credential_process`, SSO,
//! `role_arn` + `source_profile` chaining, or `role_arn` + a web identity token.
//!
//! # Caching
//!
//! Resolved credentials are cached until shortly before they expire. Static keys
//! never expire and are resolved once. Every network-backed source is re-fetched
//! automatically, so a long-running `maproom watch` survives credential rotation
//! without a restart.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::embedding::aws::sigv4::{self, SignableRequest, SigningCredentials};
use crate::embedding::error::{ApiError, ConfigError, EmbeddingError};

use super::profile::ProfileSet;

/// Refresh credentials this long before they actually expire, so an in-flight
/// batch never fails on a token that lapses mid-request.
const EXPIRY_MARGIN: Duration = Duration::from_secs(300);

/// Link-local address of the EC2 instance metadata service.
const IMDS_ENDPOINT: &str = "http://169.254.169.254";

/// Link-local address ECS publishes task credentials on.
const ECS_ENDPOINT: &str = "http://169.254.170.2";

/// Timeout for metadata lookups. These are link-local and answer in
/// milliseconds when present; when absent, the socket usually fails fast but can
/// hang on hosts with unusual routing, so the ceiling is deliberately tight.
const METADATA_TIMEOUT: Duration = Duration::from_secs(2);

/// Timeout for STS and SSO calls, which are ordinary internet round trips.
const STS_TIMEOUT: Duration = Duration::from_secs(10);

/// Guard against a `source_profile` cycle in `~/.aws/config`.
const MAX_ROLE_CHAIN_DEPTH: usize = 8;

/// Where a set of credentials came from. Surfaced in logs and error messages so
/// an operator can tell "wrong account" from "no credentials" immediately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    /// `AWS_ACCESS_KEY_ID` and friends.
    Environment,
    /// Static keys in a shared-config profile.
    Profile,
    /// A `credential_process` command's stdout.
    CredentialProcess,
    /// AWS IAM Identity Center (SSO) cached token.
    Sso,
    /// `sts:AssumeRole`, reached through `role_arn` + `source_profile`.
    AssumeRole,
    /// `sts:AssumeRoleWithWebIdentity` — EKS IRSA, GitHub OIDC.
    WebIdentity,
    /// ECS task role or EKS Pod Identity.
    Container,
    /// EC2 instance profile via IMDSv2.
    Imds,
}

impl CredentialSource {
    /// Human-readable label for logs and errors.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Environment => "environment variables",
            Self::Profile => "shared config profile",
            Self::CredentialProcess => "credential_process",
            Self::Sso => "IAM Identity Center (SSO)",
            Self::AssumeRole => "sts:AssumeRole",
            Self::WebIdentity => "sts:AssumeRoleWithWebIdentity",
            Self::Container => "container credentials endpoint",
            Self::Imds => "EC2 instance metadata (IMDSv2)",
        }
    }
}

/// Resolved AWS credentials, with the expiry and provenance the signer does not
/// need but the cache and diagnostics do.
#[derive(Clone)]
pub struct AwsCredentials {
    /// Access key id.
    pub access_key_id: String,
    /// Secret access key.
    pub secret_access_key: String,
    /// Session token for temporary credentials.
    pub session_token: Option<String>,
    /// Absolute expiry, if these are temporary credentials.
    pub expires_at: Option<SystemTime>,
    /// Which source produced these.
    pub source: CredentialSource,
}

impl std::fmt::Debug for AwsCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwsCredentials")
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"<redacted>")
            .field("session_token", &self.session_token.as_ref().map(|_| "<redacted>"))
            .field("expires_at", &self.expires_at)
            .field("source", &self.source)
            .finish()
    }
}

impl AwsCredentials {
    /// Whether these credentials are still usable, allowing for the refresh margin.
    fn is_fresh(&self) -> bool {
        match self.expires_at {
            None => true,
            Some(expiry) => SystemTime::now() + EXPIRY_MARGIN < expiry,
        }
    }

    /// Narrow to the fields the signer needs.
    pub fn for_signing(&self) -> SigningCredentials {
        SigningCredentials {
            access_key_id: self.access_key_id.clone(),
            secret_access_key: self.secret_access_key.clone(),
            session_token: self.session_token.clone(),
        }
    }
}

/// Resolves and caches AWS credentials.
///
/// Cloning is cheap and shares the cache, so one provider can back every
/// concurrent embedding request without re-resolving per call.
#[derive(Clone)]
pub struct CredentialsProvider {
    http: reqwest::Client,
    profiles: Arc<ProfileSet>,
    /// Explicitly requested profile name, if any.
    profile_name: Option<String>,
    /// Region used for regional STS endpoints.
    region: String,
    cache: Arc<RwLock<Option<AwsCredentials>>>,
}

impl CredentialsProvider {
    /// Build a provider for a region, honoring `MAPROOM_AWS_PROFILE` /
    /// `AWS_PROFILE` for profile selection.
    pub fn new(http: reqwest::Client, region: String) -> Self {
        let profile_name = env_nonempty("MAPROOM_AWS_PROFILE").or_else(|| env_nonempty("AWS_PROFILE"));
        Self {
            http,
            profiles: Arc::new(ProfileSet::load()),
            profile_name,
            region,
            cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Which profile the provider will use, if one was named.
    pub fn profile_name(&self) -> Option<&str> {
        self.profile_name.as_deref()
    }

    /// Fetch signing credentials, resolving or refreshing as needed.
    pub async fn signing_credentials(&self) -> Result<SigningCredentials, EmbeddingError> {
        Ok(self.credentials().await?.for_signing())
    }

    /// Fetch credentials, using the cache when they are still fresh.
    pub async fn credentials(&self) -> Result<AwsCredentials, EmbeddingError> {
        if let Some(cached) = self.cache.read().await.as_ref() {
            if cached.is_fresh() {
                return Ok(cached.clone());
            }
        }

        // Re-check under the write lock so a burst of concurrent sub-batches
        // triggers one refresh, not one per task.
        let mut cache = self.cache.write().await;
        if let Some(cached) = cache.as_ref() {
            if cached.is_fresh() {
                return Ok(cached.clone());
            }
        }

        let resolved = self.resolve().await?;
        tracing::info!(
            "Resolved AWS credentials from {} (access key {}…, expires: {})",
            resolved.source.label(),
            &resolved.access_key_id.chars().take(8).collect::<String>(),
            resolved
                .expires_at
                .map(|expiry| format!("{:?}", DateTime::<Utc>::from(expiry)))
                .unwrap_or_else(|| "never".to_string()),
        );
        *cache = Some(resolved.clone());
        Ok(resolved)
    }

    /// Walk the resolution order and return the first source that yields credentials.
    async fn resolve(&self) -> Result<AwsCredentials, EmbeddingError> {
        let mut attempted: Vec<String> = Vec::new();

        // 1. Environment variables.
        if let Some(credentials) = credentials_from_env() {
            return Ok(credentials);
        }
        attempted.push("environment (AWS_ACCESS_KEY_ID unset)".to_string());

        // 2. An explicitly named profile. Failure here is fatal on purpose:
        //    falling through would silently use a different AWS account.
        if let Some(name) = &self.profile_name {
            if !self.profiles.has_profile(name) {
                return Err(EmbeddingError::Config(ConfigError::InvalidValue {
                    field: "AWS_PROFILE".to_string(),
                    reason: format!(
                        "AWS profile '{}' not found in ~/.aws/config or ~/.aws/credentials.\n\
                         Available profiles: {}\n\
                         Unset AWS_PROFILE/MAPROOM_AWS_PROFILE to fall back to the default chain.",
                        name,
                        match self.profiles.profile_names().as_slice() {
                            [] => "(none)".to_string(),
                            names => names.join(", "),
                        }
                    ),
                }));
            }
            return self.credentials_from_profile(name, 0).await;
        }

        // 3. Web identity handed to us by the environment (EKS IRSA, OIDC in CI).
        if let (Some(token_file), Some(role_arn)) = (
            env_nonempty("AWS_WEB_IDENTITY_TOKEN_FILE"),
            env_nonempty("AWS_ROLE_ARN"),
        ) {
            return self
                .assume_role_with_web_identity(
                    &PathBuf::from(token_file),
                    &role_arn,
                    env_nonempty("AWS_ROLE_SESSION_NAME").as_deref(),
                )
                .await;
        }
        attempted.push("web identity (AWS_WEB_IDENTITY_TOKEN_FILE unset)".to_string());

        // 4. The default profile, when one is configured.
        if self.profiles.has_profile("default") {
            match self.credentials_from_profile("default", 0).await {
                Ok(credentials) => return Ok(credentials),
                Err(error) => {
                    // Unlike an explicitly named profile, a broken `default` is
                    // worth stepping over — the host may still have a role.
                    tracing::debug!("Default AWS profile did not yield credentials: {error}");
                    attempted.push(format!("default profile ({error})"));
                }
            }
        } else {
            attempted.push("default profile (not configured)".to_string());
        }

        // 5. ECS task role / EKS Pod Identity.
        if let Some(credentials) = self.credentials_from_container().await? {
            return Ok(credentials);
        }
        attempted.push("container credentials (endpoint env vars unset)".to_string());

        // 6. EC2 instance role.
        match self.credentials_from_imds().await {
            Ok(Some(credentials)) => return Ok(credentials),
            Ok(None) => attempted.push("EC2 IMDSv2 (not an EC2 instance)".to_string()),
            Err(error) => attempted.push(format!("EC2 IMDSv2 ({error})")),
        }

        Err(EmbeddingError::Config(ConfigError::MissingConfig(format!(
            "No AWS credentials found for the Bedrock embedding provider.\n\
             \n\
             Tried, in order:\n  - {}\n\
             \n\
             Configure one of:\n\
               * export AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=...\n\
               * aws sso login --profile <name> && export AWS_PROFILE=<name>\n\
               * an EC2 instance role, ECS task role, or EKS service account (IRSA)\n\
             \n\
             Verify with: aws sts get-caller-identity",
            attempted.join("\n  - "),
        ))))
    }

    /// Resolve one profile, following `role_arn` chains up to a depth limit.
    ///
    /// Boxed because the role-chaining case recurses into this same function.
    fn credentials_from_profile<'a>(
        &'a self,
        name: &'a str,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AwsCredentials, EmbeddingError>> + Send + 'a>>
    {
        Box::pin(async move {
            if depth > MAX_ROLE_CHAIN_DEPTH {
                return Err(EmbeddingError::Config(ConfigError::InvalidValue {
                    field: "source_profile".to_string(),
                    reason: format!(
                        "AWS profile role chain exceeded {MAX_ROLE_CHAIN_DEPTH} hops at '{name}'. \
                         Check ~/.aws/config for a source_profile cycle."
                    ),
                }));
            }

            let get = |key: &str| self.profiles.get(name, key).map(str::to_string);

            // A profile that assumes a role delegates to whatever supplies the
            // *source* credentials, then calls STS.
            if let Some(role_arn) = get("role_arn") {
                // `role_arn` + `web_identity_token_file` is the profile form of IRSA.
                if let Some(token_file) = get("web_identity_token_file") {
                    return self
                        .assume_role_with_web_identity(
                            &PathBuf::from(token_file),
                            &role_arn,
                            get("role_session_name").as_deref(),
                        )
                        .await;
                }

                let source = match (get("source_profile"), get("credential_source")) {
                    (Some(source_profile), _) => {
                        self.credentials_from_profile(&source_profile, depth + 1).await?
                    }
                    (None, Some(credential_source)) => {
                        self.credentials_from_credential_source(&credential_source).await?
                    }
                    (None, None) => {
                        return Err(EmbeddingError::Config(ConfigError::InvalidValue {
                            field: format!("profile {name}"),
                            reason: "role_arn is set but neither source_profile nor \
                                     credential_source is. Add one so the role can be assumed."
                                .to_string(),
                        }));
                    }
                };

                return self
                    .assume_role(
                        &source,
                        &role_arn,
                        get("role_session_name").as_deref(),
                        get("external_id").as_deref(),
                        get("duration_seconds").as_deref(),
                    )
                    .await;
            }

            // A profile can shell out to an external credential helper.
            if let Some(command) = get("credential_process") {
                return credentials_from_process(&command).await;
            }

            // IAM Identity Center.
            if get("sso_start_url").is_some() || get("sso_session").is_some() {
                return self.credentials_from_sso(name).await;
            }

            // Plain static keys.
            match (get("aws_access_key_id"), get("aws_secret_access_key")) {
                (Some(access_key_id), Some(secret_access_key)) => Ok(AwsCredentials {
                    access_key_id,
                    secret_access_key,
                    session_token: get("aws_session_token"),
                    expires_at: None,
                    source: CredentialSource::Profile,
                }),
                _ => Err(EmbeddingError::Config(ConfigError::InvalidValue {
                    field: format!("profile {name}"),
                    reason: "profile has no aws_access_key_id/aws_secret_access_key, \
                             credential_process, sso_start_url, or role_arn"
                        .to_string(),
                })),
            }
        })
    }

    /// Handle `credential_source = Ec2InstanceMetadata | EcsContainer | Environment`.
    async fn credentials_from_credential_source(
        &self,
        source: &str,
    ) -> Result<AwsCredentials, EmbeddingError> {
        match source.trim() {
            "Environment" => credentials_from_env().ok_or_else(|| {
                EmbeddingError::Config(ConfigError::MissingConfig(
                    "credential_source = Environment, but AWS_ACCESS_KEY_ID is not set".to_string(),
                ))
            }),
            "Ec2InstanceMetadata" => self.credentials_from_imds().await?.ok_or_else(|| {
                EmbeddingError::Config(ConfigError::MissingConfig(
                    "credential_source = Ec2InstanceMetadata, but IMDS is unreachable".to_string(),
                ))
            }),
            "EcsContainer" => self.credentials_from_container().await?.ok_or_else(|| {
                EmbeddingError::Config(ConfigError::MissingConfig(
                    "credential_source = EcsContainer, but no container credential endpoint is set"
                        .to_string(),
                ))
            }),
            other => Err(EmbeddingError::Config(ConfigError::InvalidValue {
                field: "credential_source".to_string(),
                reason: format!(
                    "unsupported value '{other}'. Expected Environment, \
                     Ec2InstanceMetadata, or EcsContainer."
                ),
            })),
        }
    }

    /// Exchange an SSO cached access token for role credentials.
    async fn credentials_from_sso(&self, profile: &str) -> Result<AwsCredentials, EmbeddingError> {
        let account_id = self.profiles.get(profile, "sso_account_id").ok_or_else(|| {
            sso_config_error(profile, "sso_account_id")
        })?;
        let role_name = self.profiles.get(profile, "sso_role_name").ok_or_else(|| {
            sso_config_error(profile, "sso_role_name")
        })?;

        // Newer configs point at a shared `[sso-session]`; older ones inline the
        // start URL and region on the profile itself.
        let (start_url, sso_region) = match self.profiles.get(profile, "sso_session") {
            Some(session_name) => {
                let session = self.profiles.sso_session(session_name).ok_or_else(|| {
                    EmbeddingError::Config(ConfigError::InvalidValue {
                        field: format!("profile {profile}"),
                        reason: format!(
                            "sso_session = '{session_name}' but no [sso-session {session_name}] \
                             section exists in ~/.aws/config"
                        ),
                    })
                })?;
                (
                    session.get("sso_start_url").cloned(),
                    session.get("sso_region").cloned(),
                )
            }
            None => (
                self.profiles.get(profile, "sso_start_url").map(str::to_string),
                self.profiles.get(profile, "sso_region").map(str::to_string),
            ),
        };

        let start_url = start_url.ok_or_else(|| sso_config_error(profile, "sso_start_url"))?;
        let sso_region = sso_region.unwrap_or_else(|| self.region.clone());

        let access_token = load_sso_access_token(&start_url).ok_or_else(|| {
            EmbeddingError::Config(ConfigError::MissingConfig(format!(
                "No valid IAM Identity Center token cached for {start_url}.\n\
                 Run: aws sso login --profile {profile}"
            )))
        })?;

        let url = format!(
            "https://portal.sso.{sso_region}.amazonaws.com/federation/credentials?role_name={}&account_id={}",
            form_encode(role_name),
            form_encode(account_id),
        );
        let response = self
            .http
            .get(&url)
            .header("x-amz-sso_bearer_token", &access_token)
            .timeout(STS_TIMEOUT)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::Api(ApiError::Authentication(format!(
                "IAM Identity Center rejected the cached token (HTTP {status}): {body}\n\
                 Run: aws sso login --profile {profile}"
            ))));
        }

        #[derive(Deserialize)]
        struct SsoResponse {
            #[serde(rename = "roleCredentials")]
            role_credentials: SsoRoleCredentials,
        }
        #[derive(Deserialize)]
        struct SsoRoleCredentials {
            #[serde(rename = "accessKeyId")]
            access_key_id: String,
            #[serde(rename = "secretAccessKey")]
            secret_access_key: String,
            #[serde(rename = "sessionToken")]
            session_token: String,
            /// Milliseconds since the Unix epoch, unlike every other AWS API.
            expiration: i64,
        }

        let parsed: SsoResponse = response.json().await?;
        Ok(AwsCredentials {
            access_key_id: parsed.role_credentials.access_key_id,
            secret_access_key: parsed.role_credentials.secret_access_key,
            session_token: Some(parsed.role_credentials.session_token),
            expires_at: Some(
                SystemTime::UNIX_EPOCH
                    + Duration::from_millis(parsed.role_credentials.expiration.max(0) as u64),
            ),
            source: CredentialSource::Sso,
        })
    }

    /// Call `sts:AssumeRole` using `source` to sign the request.
    async fn assume_role(
        &self,
        source: &AwsCredentials,
        role_arn: &str,
        session_name: Option<&str>,
        external_id: Option<&str>,
        duration_seconds: Option<&str>,
    ) -> Result<AwsCredentials, EmbeddingError> {
        let session_name = session_name.unwrap_or("maproom");
        let mut body = format!(
            "Action=AssumeRole&Version=2011-06-15&RoleArn={}&RoleSessionName={}",
            form_encode(role_arn),
            form_encode(session_name),
        );
        if let Some(external_id) = external_id {
            body.push_str(&format!("&ExternalId={}", form_encode(external_id)));
        }
        if let Some(duration) = duration_seconds {
            body.push_str(&format!("&DurationSeconds={}", form_encode(duration)));
        }

        let host = format!("sts.{}.amazonaws.com", self.region);
        let headers = vec![(
            "content-type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        )];
        let signable = SignableRequest {
            method: "POST",
            host: &host,
            path: "/",
            query: "",
            headers: &headers,
            body: body.as_bytes(),
        };
        let signed = sigv4::sign_request(
            &signable,
            &source.for_signing(),
            &self.region,
            "sts",
            &sigv4::format_amz_date(SystemTime::now()),
        );

        let mut request = self
            .http
            .post(format!("https://{host}/"))
            .timeout(STS_TIMEOUT)
            .body(body);
        for (name, value) in &signed.headers {
            request = request.header(name, value);
        }

        let response = request.send().await?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(EmbeddingError::Api(ApiError::Authentication(format!(
                "sts:AssumeRole for {role_arn} failed (HTTP {}): {}",
                status.as_u16(),
                text.trim(),
            ))));
        }

        parse_sts_credentials(&text, CredentialSource::AssumeRole)
    }

    /// Call `sts:AssumeRoleWithWebIdentity`. This request is unsigned — the JWT
    /// is the credential.
    async fn assume_role_with_web_identity(
        &self,
        token_file: &PathBuf,
        role_arn: &str,
        session_name: Option<&str>,
    ) -> Result<AwsCredentials, EmbeddingError> {
        let token = tokio::fs::read_to_string(token_file).await.map_err(|error| {
            EmbeddingError::Config(ConfigError::FileError(format!(
                "Failed to read web identity token at {}: {error}\n\
                 In EKS this file is projected by the service account; check that the \
                 pod's service account is annotated with eks.amazonaws.com/role-arn.",
                token_file.display(),
            )))
        })?;

        let session_name = session_name.unwrap_or("maproom");
        let body = format!(
            "Action=AssumeRoleWithWebIdentity&Version=2011-06-15&RoleArn={}&RoleSessionName={}&WebIdentityToken={}",
            form_encode(role_arn),
            form_encode(session_name),
            form_encode(token.trim()),
        );

        let host = format!("sts.{}.amazonaws.com", self.region);
        let response = self
            .http
            .post(format!("https://{host}/"))
            .header("content-type", "application/x-www-form-urlencoded")
            .timeout(STS_TIMEOUT)
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(EmbeddingError::Api(ApiError::Authentication(format!(
                "sts:AssumeRoleWithWebIdentity for {role_arn} failed (HTTP {}): {}",
                status.as_u16(),
                text.trim(),
            ))));
        }

        parse_sts_credentials(&text, CredentialSource::WebIdentity)
    }

    /// Read credentials from the ECS / EKS Pod Identity container endpoint.
    ///
    /// Returns `Ok(None)` when neither endpoint variable is set, meaning "not
    /// running in a container with a task role" rather than a failure.
    async fn credentials_from_container(&self) -> Result<Option<AwsCredentials>, EmbeddingError> {
        let url = match (
            env_nonempty("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI"),
            env_nonempty("AWS_CONTAINER_CREDENTIALS_FULL_URI"),
        ) {
            (Some(relative), _) => format!("{ECS_ENDPOINT}{relative}"),
            (None, Some(full)) => full,
            (None, None) => return Ok(None),
        };

        let mut request = self.http.get(&url).timeout(METADATA_TIMEOUT);

        // The full-URI form (EKS Pod Identity) carries a bearer token, supplied
        // either inline or — preferred, and required by newer agents — in a file.
        let token = match env_nonempty("AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE") {
            Some(path) => tokio::fs::read_to_string(&path).await.ok().map(|t| t.trim().to_string()),
            None => env_nonempty("AWS_CONTAINER_AUTHORIZATION_TOKEN"),
        };
        if let Some(token) = token {
            request = request.header("Authorization", token);
        }

        let response = request.send().await.map_err(|error| {
            EmbeddingError::Config(ConfigError::MissingConfig(format!(
                "Container credential endpoint {url} is unreachable: {error}"
            )))
        })?;

        if !response.status().is_success() {
            return Err(EmbeddingError::Api(ApiError::Authentication(format!(
                "Container credential endpoint returned HTTP {}",
                response.status().as_u16()
            ))));
        }

        let parsed: MetadataCredentials = response.json().await?;
        Ok(Some(parsed.into_credentials(CredentialSource::Container)?))
    }

    /// Read credentials from EC2 instance metadata using IMDSv2.
    ///
    /// Returns `Ok(None)` when the service is absent (not an EC2 instance) or
    /// when `AWS_EC2_METADATA_DISABLED` is set.
    async fn credentials_from_imds(&self) -> Result<Option<AwsCredentials>, EmbeddingError> {
        if env_nonempty("AWS_EC2_METADATA_DISABLED")
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
        {
            tracing::debug!("Skipping IMDS: AWS_EC2_METADATA_DISABLED is set");
            return Ok(None);
        }

        let base = env_nonempty("AWS_EC2_METADATA_SERVICE_ENDPOINT")
            .unwrap_or_else(|| IMDS_ENDPOINT.to_string());

        // IMDSv2 is session-oriented: PUT for a token, then send it on each read.
        // Only v2 is attempted; v1 is disabled by default on modern accounts and
        // falling back to it would weaken SSRF protection.
        let token_response = self
            .http
            .put(format!("{base}/latest/api/token"))
            .header("x-aws-ec2-metadata-token-ttl-seconds", "21600")
            .timeout(METADATA_TIMEOUT)
            .send()
            .await;

        let token = match token_response {
            Ok(response) if response.status().is_success() => response.text().await?,
            // No IMDS here — this is the common case off EC2, not an error.
            _ => return Ok(None),
        };

        let role_response = self
            .http
            .get(format!("{base}/latest/meta-data/iam/security-credentials/"))
            .header("x-aws-ec2-metadata-token", &token)
            .timeout(METADATA_TIMEOUT)
            .send()
            .await?;

        if !role_response.status().is_success() {
            // IMDS exists but no instance profile is attached.
            return Ok(None);
        }

        let role_name = role_response.text().await?.trim().to_string();
        if role_name.is_empty() {
            return Ok(None);
        }

        let credentials_response = self
            .http
            .get(format!(
                "{base}/latest/meta-data/iam/security-credentials/{role_name}"
            ))
            .header("x-aws-ec2-metadata-token", &token)
            .timeout(METADATA_TIMEOUT)
            .send()
            .await?;

        if !credentials_response.status().is_success() {
            return Err(EmbeddingError::Api(ApiError::Authentication(format!(
                "IMDS returned HTTP {} for instance role '{role_name}'",
                credentials_response.status().as_u16()
            ))));
        }

        let parsed: MetadataCredentials = credentials_response.json().await?;
        Ok(Some(parsed.into_credentials(CredentialSource::Imds)?))
    }
}

/// The JSON shape ECS and IMDS both return.
#[derive(Deserialize)]
struct MetadataCredentials {
    #[serde(rename = "AccessKeyId")]
    access_key_id: String,
    #[serde(rename = "SecretAccessKey")]
    secret_access_key: String,
    #[serde(rename = "Token")]
    token: Option<String>,
    /// ECS uses this name; IMDS returns the same field.
    #[serde(rename = "Expiration")]
    expiration: Option<String>,
}

impl MetadataCredentials {
    fn into_credentials(
        self,
        source: CredentialSource,
    ) -> Result<AwsCredentials, EmbeddingError> {
        Ok(AwsCredentials {
            access_key_id: self.access_key_id,
            secret_access_key: self.secret_access_key,
            session_token: self.token,
            expires_at: self.expiration.as_deref().and_then(parse_rfc3339),
            source,
        })
    }
}

/// Read static credentials from the environment.
fn credentials_from_env() -> Option<AwsCredentials> {
    let access_key_id = env_nonempty("AWS_ACCESS_KEY_ID")?;
    let secret_access_key = env_nonempty("AWS_SECRET_ACCESS_KEY")?;
    Some(AwsCredentials {
        access_key_id,
        secret_access_key,
        session_token: env_nonempty("AWS_SESSION_TOKEN"),
        // Session tokens from the environment carry no machine-readable expiry;
        // treat them as static and let the API report expiry if it happens.
        expires_at: None,
        source: CredentialSource::Environment,
    })
}

/// Run a `credential_process` command and parse its stdout.
async fn credentials_from_process(command: &str) -> Result<AwsCredentials, EmbeddingError> {
    let mut parts = shell_split(command);
    if parts.is_empty() {
        return Err(EmbeddingError::Config(ConfigError::InvalidValue {
            field: "credential_process".to_string(),
            reason: "value is empty".to_string(),
        }));
    }
    let program = parts.remove(0);

    let output = tokio::process::Command::new(&program)
        .args(&parts)
        .output()
        .await
        .map_err(|error| {
            EmbeddingError::Config(ConfigError::InvalidValue {
                field: "credential_process".to_string(),
                reason: format!("failed to run '{program}': {error}"),
            })
        })?;

    if !output.status.success() {
        return Err(EmbeddingError::Config(ConfigError::InvalidValue {
            field: "credential_process".to_string(),
            reason: format!(
                "'{program}' exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim(),
            ),
        }));
    }

    #[derive(Deserialize)]
    struct ProcessCredentials {
        #[serde(rename = "Version")]
        version: u32,
        #[serde(rename = "AccessKeyId")]
        access_key_id: String,
        #[serde(rename = "SecretAccessKey")]
        secret_access_key: String,
        #[serde(rename = "SessionToken")]
        session_token: Option<String>,
        #[serde(rename = "Expiration")]
        expiration: Option<String>,
    }

    let parsed: ProcessCredentials = serde_json::from_slice(&output.stdout).map_err(|error| {
        EmbeddingError::Config(ConfigError::InvalidValue {
            field: "credential_process".to_string(),
            reason: format!("'{program}' did not emit valid credential JSON: {error}"),
        })
    })?;

    if parsed.version != 1 {
        return Err(EmbeddingError::Config(ConfigError::InvalidValue {
            field: "credential_process".to_string(),
            reason: format!(
                "unsupported payload Version {} (expected 1)",
                parsed.version
            ),
        }));
    }

    Ok(AwsCredentials {
        access_key_id: parsed.access_key_id,
        secret_access_key: parsed.secret_access_key,
        session_token: parsed.session_token,
        expires_at: parsed.expiration.as_deref().and_then(parse_rfc3339),
        source: CredentialSource::CredentialProcess,
    })
}

/// Find a non-expired SSO access token in `~/.aws/sso/cache`.
///
/// The cache filename is a hash of the session name or start URL depending on
/// CLI version, so rather than reproducing that hashing we scan the directory
/// and match on the `startUrl` recorded inside each entry.
fn load_sso_access_token(start_url: &str) -> Option<String> {
    let cache_dir = dirs::home_dir()?.join(".aws").join("sso").join("cache");
    let entries = std::fs::read_dir(cache_dir).ok()?;

    #[derive(Deserialize)]
    struct CachedToken {
        #[serde(rename = "accessToken")]
        access_token: Option<String>,
        #[serde(rename = "expiresAt")]
        expires_at: Option<String>,
        #[serde(rename = "startUrl")]
        start_url: Option<String>,
    }

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(token) = serde_json::from_str::<CachedToken>(&contents) else {
            continue;
        };

        let (Some(access_token), Some(cached_url)) = (token.access_token, token.start_url) else {
            continue;
        };
        if cached_url.trim_end_matches('/') != start_url.trim_end_matches('/') {
            continue;
        }
        // Registration files live in the same directory and have no expiry;
        // treat a missing expiry as unusable rather than assuming it is valid.
        let Some(expiry) = token.expires_at.as_deref().and_then(parse_rfc3339) else {
            continue;
        };
        if SystemTime::now() + EXPIRY_MARGIN >= expiry {
            tracing::debug!("Cached SSO token for {start_url} has expired");
            continue;
        }
        return Some(access_token);
    }
    None
}

/// Pull the four credential fields out of an STS XML response.
///
/// STS speaks the AWS Query protocol, which is XML-only. Rather than take an XML
/// parser dependency for four well-known leaf elements, this extracts them
/// directly; anything malformed fails loudly rather than silently signing with
/// empty strings.
fn parse_sts_credentials(
    xml: &str,
    source: CredentialSource,
) -> Result<AwsCredentials, EmbeddingError> {
    let field = |name: &str| extract_xml_value(xml, name);

    let (Some(access_key_id), Some(secret_access_key), Some(session_token)) = (
        field("AccessKeyId"),
        field("SecretAccessKey"),
        field("SessionToken"),
    ) else {
        return Err(EmbeddingError::Api(ApiError::InvalidResponse(format!(
            "STS response did not contain a full credential set: {}",
            xml.chars().take(500).collect::<String>()
        ))));
    };

    Ok(AwsCredentials {
        access_key_id,
        secret_access_key,
        session_token: Some(session_token),
        expires_at: field("Expiration").as_deref().and_then(parse_rfc3339),
        source,
    })
}

/// Extract the text content of the first `<name>…</name>` element.
fn extract_xml_value(xml: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}

/// Parse an RFC 3339 timestamp into a [`SystemTime`].
fn parse_rfc3339(value: &str) -> Option<SystemTime> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| SystemTime::from(parsed.with_timezone(&Utc)))
}

/// Percent-encode a value for an `application/x-www-form-urlencoded` body.
fn form_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Split a `credential_process` command into program and arguments.
///
/// Handles single and double quotes so paths with spaces work. The command is
/// **not** run through a shell — no globbing, pipes, or substitution — which
/// keeps a hostile `~/.aws/config` from turning into arbitrary shell execution
/// beyond the single program it names.
fn shell_split(command: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut has_token = false;

    for character in command.chars() {
        match (quote, character) {
            (Some(open), c) if c == open => {
                quote = None;
            }
            (Some(_), c) => current.push(c),
            (None, c @ ('\'' | '"')) => {
                quote = Some(c);
                has_token = true;
            }
            (None, c) if c.is_whitespace() => {
                if has_token || !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            (None, c) => current.push(c),
        }
    }
    if has_token || !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// Read an environment variable, treating empty as unset.
///
/// AWS tooling and container runtimes frequently export empty strings for unset
/// values; treating `AWS_PROFILE=""` as a profile named "" fails confusingly.
fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Build the "SSO profile is missing a key" error.
fn sso_config_error(profile: &str, key: &str) -> EmbeddingError {
    EmbeddingError::Config(ConfigError::InvalidValue {
        field: format!("profile {profile}"),
        reason: format!(
            "SSO profile is missing '{key}'. A complete SSO profile needs \
             sso_start_url (or sso_session), sso_account_id, and sso_role_name."
        ),
    })
}

/// Names of every environment variable the chain reads. Used by diagnostics to
/// report what the current environment would select.
pub fn relevant_env_vars() -> Vec<&'static str> {
    let names = [
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_SESSION_TOKEN",
        "AWS_PROFILE",
        "MAPROOM_AWS_PROFILE",
        "AWS_REGION",
        "AWS_DEFAULT_REGION",
        "AWS_ROLE_ARN",
        "AWS_WEB_IDENTITY_TOKEN_FILE",
        "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
        "AWS_CONTAINER_CREDENTIALS_FULL_URI",
        "AWS_EC2_METADATA_DISABLED",
        "AWS_CONFIG_FILE",
        "AWS_SHARED_CREDENTIALS_FILE",
    ];
    // Deduplicate defensively so callers can print the list verbatim.
    let mut seen = HashSet::new();
    names.into_iter().filter(|name| seen.insert(*name)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_credentials_respect_the_refresh_margin() {
        let make = |expires_at| AwsCredentials {
            access_key_id: "AKIA".to_string(),
            secret_access_key: "secret".to_string(),
            session_token: None,
            expires_at,
            source: CredentialSource::Environment,
        };

        assert!(make(None).is_fresh(), "static keys never expire");
        assert!(
            make(Some(SystemTime::now() + Duration::from_secs(3600))).is_fresh(),
            "an hour of headroom is fresh"
        );
        assert!(
            !make(Some(SystemTime::now() + Duration::from_secs(60))).is_fresh(),
            "inside the 5 minute margin must be treated as stale"
        );
        assert!(
            !make(Some(SystemTime::now() - Duration::from_secs(1))).is_fresh(),
            "already expired"
        );
    }

    #[test]
    fn sts_xml_credentials_are_extracted() {
        let xml = r#"<AssumeRoleResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
  <AssumeRoleResult>
    <Credentials>
      <AccessKeyId>ASIAEXAMPLE</AccessKeyId>
      <SecretAccessKey>secret/key+value</SecretAccessKey>
      <SessionToken>token-value</SessionToken>
      <Expiration>2030-01-01T00:00:00Z</Expiration>
    </Credentials>
  </AssumeRoleResult>
</AssumeRoleResponse>"#;

        let credentials = parse_sts_credentials(xml, CredentialSource::AssumeRole).unwrap();
        assert_eq!(credentials.access_key_id, "ASIAEXAMPLE");
        assert_eq!(credentials.secret_access_key, "secret/key+value");
        assert_eq!(credentials.session_token.as_deref(), Some("token-value"));
        assert!(credentials.expires_at.is_some());
        assert_eq!(credentials.source, CredentialSource::AssumeRole);
    }

    #[test]
    fn incomplete_sts_response_is_an_error_not_empty_credentials() {
        // Signing with empty strings would produce an opaque
        // SignatureDoesNotMatch instead of a usable diagnostic.
        let xml = "<AssumeRoleResponse><AccessKeyId>ASIA</AccessKeyId></AssumeRoleResponse>";
        let error = parse_sts_credentials(xml, CredentialSource::AssumeRole).unwrap_err();
        assert!(error.to_string().contains("full credential set"));
    }

    #[test]
    fn sts_error_response_does_not_parse_as_credentials() {
        let xml = "<ErrorResponse><Error><Code>AccessDenied</Code></Error></ErrorResponse>";
        assert!(parse_sts_credentials(xml, CredentialSource::AssumeRole).is_err());
    }

    #[test]
    fn xml_extraction_finds_nested_elements() {
        assert_eq!(
            extract_xml_value("<a><b>value</b></a>", "b").as_deref(),
            Some("value")
        );
        assert_eq!(extract_xml_value("<a></a>", "missing"), None);
    }

    #[test]
    fn form_encoding_escapes_arn_and_jwt_characters() {
        assert_eq!(
            form_encode("arn:aws:iam::123456789012:role/My-Role"),
            "arn%3Aaws%3Aiam%3A%3A123456789012%3Arole%2FMy-Role"
        );
        // JWTs are base64url, which may carry '=' padding.
        assert_eq!(form_encode("a.b-c_d="), "a.b-c_d%3D");
        assert_eq!(form_encode("plain-text.value~1"), "plain-text.value~1");
    }

    #[test]
    fn shell_split_handles_quotes_and_spaces() {
        assert_eq!(shell_split("aws-vault exec dev"), vec!["aws-vault", "exec", "dev"]);
        assert_eq!(
            shell_split("\"/opt/my helper/creds\" --profile dev"),
            vec!["/opt/my helper/creds", "--profile", "dev"]
        );
        assert_eq!(
            shell_split("helper --json '{\"a\": 1}'"),
            vec!["helper", "--json", "{\"a\": 1}"]
        );
        assert!(shell_split("   ").is_empty());
    }

    #[test]
    fn shell_split_preserves_empty_quoted_arguments() {
        assert_eq!(shell_split("helper '' x"), vec!["helper", "", "x"]);
    }

    #[test]
    fn rfc3339_parsing_accepts_aws_timestamp_forms() {
        assert!(parse_rfc3339("2030-01-01T00:00:00Z").is_some());
        assert!(parse_rfc3339("2030-01-01T00:00:00+00:00").is_some());
        assert!(parse_rfc3339("not a timestamp").is_none());
    }

    #[test]
    fn metadata_credentials_deserialize_from_imds_shape() {
        let json = r#"{
            "AccessKeyId": "ASIAEXAMPLE",
            "SecretAccessKey": "secret",
            "Token": "session",
            "Expiration": "2030-01-01T00:00:00Z"
        }"#;
        let parsed: MetadataCredentials = serde_json::from_str(json).unwrap();
        let credentials = parsed.into_credentials(CredentialSource::Imds).unwrap();
        assert_eq!(credentials.access_key_id, "ASIAEXAMPLE");
        assert_eq!(credentials.session_token.as_deref(), Some("session"));
        assert_eq!(credentials.source, CredentialSource::Imds);
        assert!(credentials.expires_at.is_some());
    }

    #[test]
    fn credential_source_labels_are_distinct() {
        let sources = [
            CredentialSource::Environment,
            CredentialSource::Profile,
            CredentialSource::CredentialProcess,
            CredentialSource::Sso,
            CredentialSource::AssumeRole,
            CredentialSource::WebIdentity,
            CredentialSource::Container,
            CredentialSource::Imds,
        ];
        let labels: HashSet<&str> = sources.iter().map(|s| s.label()).collect();
        assert_eq!(labels.len(), sources.len(), "labels must identify the source");
    }

    #[test]
    fn debug_never_leaks_secrets() {
        let credentials = AwsCredentials {
            access_key_id: "AKIAEXAMPLE".to_string(),
            secret_access_key: "super-secret".to_string(),
            session_token: Some("token".to_string()),
            expires_at: None,
            source: CredentialSource::Environment,
        };
        let rendered = format!("{credentials:?}");
        assert!(!rendered.contains("super-secret"));
        assert!(!rendered.contains("\"token\""));
        assert!(rendered.contains("AKIAEXAMPLE"));
    }

    #[tokio::test]
    async fn credential_process_parses_version_1_payload() {
        let script = write_script(
            r#"#!/bin/sh
echo '{"Version":1,"AccessKeyId":"ASIAPROC","SecretAccessKey":"s","SessionToken":"t","Expiration":"2030-01-01T00:00:00Z"}'
"#,
        );
        let credentials = credentials_from_process(script.path())
            .await
            .unwrap();
        assert_eq!(credentials.access_key_id, "ASIAPROC");
        assert_eq!(credentials.source, CredentialSource::CredentialProcess);
        assert!(credentials.expires_at.is_some());
    }

    #[tokio::test]
    async fn credential_process_rejects_unknown_version() {
        let script = write_script(
            "#!/bin/sh\necho '{\"Version\":2,\"AccessKeyId\":\"a\",\"SecretAccessKey\":\"b\"}'\n",
        );
        let error = credentials_from_process(script.path())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("Version"));
    }

    #[tokio::test]
    async fn credential_process_failure_is_reported_with_stderr() {
        let script = write_script("#!/bin/sh\necho 'no session' >&2\nexit 1\n");
        let error = credentials_from_process(script.path())
            .await
            .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("no session"), "stderr must reach the operator: {message}");
    }

    #[tokio::test]
    async fn credential_process_missing_program_is_a_config_error() {
        let error = credentials_from_process("/nonexistent/helper")
            .await
            .unwrap_err();
        assert!(matches!(error, EmbeddingError::Config(_)));
    }

    /// A temp directory holding an executable script, kept alive by the caller.
    struct Script {
        _dir: tempfile::TempDir,
        path: PathBuf,
    }

    impl Script {
        fn path(&self) -> &str {
            self.path.to_str().unwrap()
        }
    }

    /// Write an executable shell script to a temp directory.
    ///
    /// The file handle must be fully closed before exec: an open write handle
    /// makes the kernel reject the exec with ETXTBSY.
    fn write_script(body: &str) -> Script {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credential-helper");
        std::fs::write(&path, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        Script { _dir: dir, path }
    }
}
