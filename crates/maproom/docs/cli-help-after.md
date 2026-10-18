<!-- Keep in sync with CLAUDE.md environment variables -->
<!-- If packages/cli/src/utils/maproom-binary.ts changes, update -->

ENVIRONMENT VARIABLES:
    CREWCHIEF_MAPROOM_BIN
        Override path to the maproom binary. Takes precedence over all
        other resolution methods.

        Example: export CREWCHIEF_MAPROOM_BIN="/path/to/maproom"

        Resolution priority:
          1. CREWCHIEF_MAPROOM_BIN environment variable (highest)
          2. maproomBinaryPath in crewchief.config.js
          3. Global installation (maproom in PATH)
          4. Packaged binary (bundled with CLI)

    MAPROOM_DATABASE_URL
        Database URL; determines the storage backend at runtime:
          - sqlite:// or a plain path  -> SQLite backend (default)
          - postgres:// / postgresql:// -> PostgreSQL backend
            (requires a build with --features postgres)
        Default: $HOME/.maproom/maproom.db (SQLite)

        Example (SQLite):   export MAPROOM_DATABASE_URL="sqlite://$HOME/.maproom/my-project.db"
        Example (Postgres): export MAPROOM_DATABASE_URL="postgres://user:pass@localhost/maproom"

        Overridable per-invocation by the global --database-url flag, which
        takes precedence over MAPROOM_DATABASE_URL.

        For per-repository databases, configure in .claude/settings.json:
          { "env": { "MAPROOM_DATABASE_URL": "sqlite:///home/user/.maproom/myrepo.db" } }

        Note: Use absolute paths or $HOME in shell. Tilde (~) is not expanded
        in JSON config files.

    MAPROOM_DB_ROOT
        Root directory for per-repository databases. Each repo gets its own
        subdirectory: $MAPROOM_DB_ROOT/<repo-name>/maproom.db

        MAPROOM_DATABASE_URL takes precedence if both are set.

        Example: export MAPROOM_DB_ROOT="$HOME/.maproom"

        Note: Use $HOME, not ~. Tilde is not expanded in JSON config files.

    MAPROOM_EMBEDDING_PROVIDER
        Embedding provider: ollama, openai, google, or bedrock
        ('aws' and 'aws-bedrock' are accepted as aliases for bedrock)
        Default: auto-detect Ollama. If no provider is detected, embedding
        is a configuration error (exit 2) — openai, google, and bedrock are
        never selected automatically and must be requested explicitly.

    MAPROOM_EMBEDDING_MODEL
        Model for embeddings. Provider defaults:
          ollama: mxbai-embed-large | openai: text-embedding-3-small
          bedrock: amazon.titan-embed-text-v2:0

    MAPROOM_EMBEDDING_DIMENSION
        Override the embedding dimension. Usually inferred from the model.
        Required for Bedrock model ids this build does not recognize
        (for example a provisioned-throughput ARN).
        Storage supports 768, 1024, and 1536 only. Titan v2's narrower 512
        and 256 widths are rejected up front for that reason.

    RUST_LOG
        Logging level: error, warn, info, debug, trace
        Example: RUST_LOG=debug maproom status

    OPENAI_API_KEY
        Required when using openai embedding provider.

    GOOGLE_PROJECT_ID
        Required when using google embedding provider.

AWS BEDROCK:
    Bedrock uses the standard AWS credential chain — no maproom-specific
    secret is needed. Credentials are resolved in this order:

      1. AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY (+ AWS_SESSION_TOKEN)
      2. An explicitly named profile (MAPROOM_AWS_PROFILE or AWS_PROFILE)
      3. Web identity: AWS_WEB_IDENTITY_TOKEN_FILE + AWS_ROLE_ARN (EKS IRSA)
      4. The 'default' profile in ~/.aws/config or ~/.aws/credentials
      5. ECS task role / EKS Pod Identity container endpoint
      6. EC2 instance role via IMDSv2

    A profile may itself use static keys, credential_process, IAM Identity
    Center (SSO), or role_arn + source_profile chaining.

    Quick start:
        $ aws sso login --profile my-profile      # or any credential source
        $ export AWS_PROFILE=my-profile
        $ export MAPROOM_EMBEDDING_PROVIDER=bedrock
        $ maproom scan --path /path/to/repo --generate-embeddings

    Required IAM permission: bedrock:InvokeModel on the model, and the model
    must be enabled under Bedrock > Model access in the AWS console.

    Supported models (dimension, texts per request):
        amazon.titan-embed-text-v2:0    1024, 1                [default]
        amazon.titan-embed-text-v1      1536, 1
        cohere.embed-english-v3         1024, 96
        cohere.embed-multilingual-v3    1024, 96

    MAPROOM_BEDROCK_REGION
        Region for Bedrock calls. Falls back to AWS_REGION, then
        AWS_DEFAULT_REGION, then the profile's region, then us-east-1.
        Set this when Bedrock should run in a different region than the
        rest of your AWS tooling (model availability differs per region).

    MAPROOM_AWS_PROFILE
        Shared-config profile to use. Falls back to AWS_PROFILE.
        If set and unusable, this is a hard error rather than a silent
        fallback — otherwise maproom could bill the wrong account.

    MAPROOM_BEDROCK_ENDPOINT_URL
        Override the Bedrock Runtime endpoint, for VPC/PrivateLink
        endpoints or an egress proxy. Also honors the SDK-standard
        AWS_ENDPOINT_URL_BEDROCK_RUNTIME and AWS_ENDPOINT_URL.

    MAPROOM_BEDROCK_USE_FIPS
        Set to 'true' to use the FIPS 140-3 endpoint for the region.

    Tuning throughput (Titan issues one request per text):
        MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY   default 12
        MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE    default 96 (Cohere only)
      Lower the concurrency if you see sustained throttling, or request a
      higher 'InvokeModel requests per minute' quota in Service Quotas.

    OLLAMA_URL
        Ollama server URL. Default: http://localhost:11434
        Endpoint resolution precedence (first set wins):
          MAPROOM_EMBEDDING_API_ENDPOINT > MAPROOM_OLLAMA_URL > OLLAMA_URL
          > OLLAMA_HOST > auto-detected > http://localhost:11434
        Scheme-less host:port values (OLLAMA_HOST convention) get http://
        prepended automatically.

BEFORE SEARCHING:
    Always check indexing status before performing searches:

        $ maproom status

    If repository not indexed: maproom scan --path /path/to/repo
    If embeddings missing:     maproom generate-embeddings
    For debug output:          RUST_LOG=debug maproom status

DEVELOPMENT SETUP:
    Build from source:
        $ cargo build --release --bin maproom

    Configure path:
        export CREWCHIEF_MAPROOM_BIN="./target/release/maproom"

    Or in .claude/settings.json:
        { "env": { "CREWCHIEF_MAPROOM_BIN": "./target/release/..." } }

EXIT CODES:
    0   Success. Command completed successfully. Parse stdout for results.
        An empty result set (e.g., no search hits, no stale worktrees) is
        still exit code 0.

    1   Runtime error. A transient error occurred (database lock, network
        timeout, file not found). The command may succeed if retried.

    2   Configuration error. A persistent error due to missing or invalid
        configuration (no API key, invalid provider, missing extension).
        The command will not succeed until configuration is fixed.
        Also used by clap for argument parsing errors.
