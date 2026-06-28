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
        Embedding provider: ollama, openai, or google
        Default: ollama (if detected), otherwise openai

    MAPROOM_EMBEDDING_MODEL
        Model for embeddings. Provider defaults:
          ollama: mxbai-embed-large | openai: text-embedding-3-small

    RUST_LOG
        Logging level: error, warn, info, debug, trace
        Example: RUST_LOG=debug maproom status

    OPENAI_API_KEY
        Required when using openai embedding provider.

    GOOGLE_PROJECT_ID
        Required when using google embedding provider.

    OLLAMA_URL
        Ollama server URL. Default: http://localhost:11434

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
