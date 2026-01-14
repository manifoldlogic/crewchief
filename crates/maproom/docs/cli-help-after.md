<!-- Keep in sync with CLAUDE.md environment variables -->
<!-- If packages/cli/src/utils/maproom-binary.ts changes, update -->

ENVIRONMENT VARIABLES:
    CREWCHIEF_MAPROOM_BIN
        Override path to the maproom binary. Takes precedence over all
        other resolution methods.

        Example: export CREWCHIEF_MAPROOM_BIN="/path/to/crewchief-maproom"

        Resolution priority:
          1. CREWCHIEF_MAPROOM_BIN environment variable (highest)
          2. maproomBinaryPath in crewchief.config.js
          3. Global installation (crewchief-maproom in PATH)
          4. Packaged binary (bundled with CLI)

    MAPROOM_DATABASE_URL
        Path to the SQLite database file. Default: ~/.maproom/maproom.db

        Example: export MAPROOM_DATABASE_URL="~/.maproom/my-project.db"

        For per-repository databases, configure in .claude/settings.json:
          { "env": { "MAPROOM_DATABASE_URL": "~/.maproom/myrepo.db" } }

    MAPROOM_DB_ROOT
        Alternative database directory root. MAPROOM_DATABASE_URL takes
        precedence if both are set.

        Example: export MAPROOM_DB_ROOT="~/.maproom/crewchief"

    MAPROOM_EMBEDDING_PROVIDER
        Embedding provider: ollama, openai, or google
        Default: ollama (if detected), otherwise openai

    MAPROOM_EMBEDDING_MODEL
        Model for embeddings. Provider defaults:
          ollama: mxbai-embed-large | openai: text-embedding-3-small

    RUST_LOG
        Logging level: error, warn, info, debug, trace
        Example: RUST_LOG=debug crewchief-maproom status

    OPENAI_API_KEY
        Required when using openai embedding provider.

    GOOGLE_PROJECT_ID
        Required when using google embedding provider.

    OLLAMA_URL
        Ollama server URL. Default: http://localhost:11434

BEFORE SEARCHING:
    Always check indexing status before performing searches:

        $ crewchief-maproom status

    If repository not indexed: crewchief-maproom scan --path /path/to/repo
    If embeddings missing:     crewchief-maproom generate-embeddings
    For debug output:          RUST_LOG=debug crewchief-maproom status

DEVELOPMENT SETUP:
    Build from source:
        $ cargo build --release --bin crewchief-maproom

    Configure path:
        export CREWCHIEF_MAPROOM_BIN="./target/release/crewchief-maproom"

    Or in .claude/settings.json:
        { "env": { "CREWCHIEF_MAPROOM_BIN": "./target/release/..." } }

For full documentation: https://github.com/crewchief/crewchief
