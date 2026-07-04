//! Cache management CLI commands.
//!
//! F69 honesty rewrite: every subcommand here used to construct a fresh,
//! empty, in-process `CacheSystem` that died at process exit — `stats`
//! printed all-zero fiction, `clear` "cleared" nothing, `warm` logged
//! a would-warm placeholder line and reported fabricated success counts, and
//! `invalidate`/`maintenance` cycled an empty throwaway. The REAL query
//! cache lives inside the `maproom serve` daemon process. These commands
//! now say exactly that (exit 2, config-error class) and point at the
//! working surfaces instead of fabricating success.

use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

/// Cache management commands.
#[derive(Debug, Subcommand)]
pub enum CacheCommand {
    /// Show cache statistics (use the daemon `cache.stats` RPC)
    Stats {
        /// Show detailed per-layer statistics
        #[arg(long, short)]
        detailed: bool,
    },

    /// Clear cache layers (restart the daemon to clear its cache)
    Clear {
        /// Cache layer to clear (l1, l2, l3, parse, all)
        #[arg(long, short, default_value = "all")]
        layer: String,
    },

    /// Warm the daemon search cache (use `serve --warm-queries` or the
    /// daemon `cache.warm` RPC)
    Warm {
        /// Path to file containing queries (one per line)
        // R01 / R-CLAP-1: explicit `short = 'f'` — a bare `short` here derived
        // `-q` from the field name, colliding with `queries` below and
        // panicking clap's debug assertions on every invocation (exit 101).
        #[arg(long, short = 'f')]
        queries_file: Option<PathBuf>,

        /// Individual queries to warm (can be repeated)
        #[arg(long = "query", short)]
        queries: Vec<String>,
    },

    /// Invalidate cache entries (restart the daemon; its 60s TTL also
    /// bounds staleness automatically)
    Invalidate {
        /// Invalidate all caches
        #[arg(long, short)]
        all: bool,

        /// Invalidate by pattern
        #[arg(long, short)]
        pattern: Option<String>,

        /// Invalidate specific cache layers
        #[arg(long, short)]
        layer: Option<String>,

        /// Invalidate for file change
        #[arg(long, short)]
        file: Option<PathBuf>,
    },

    /// Run cache maintenance cycle (the daemon expires entries via TTL)
    Maintenance {
        /// Run continuously
        #[arg(long, short)]
        continuous: bool,

        /// Interval in seconds (for continuous mode)
        #[arg(long, default_value = "60")]
        interval: u64,
    },
}

impl CacheCommand {
    /// Execute the cache command.
    ///
    /// Every arm errors honestly: this process holds no cache, and
    /// pretending otherwise (the pre-F69 behavior) poisoned operational
    /// signals with fictional stats. The "Configuration error:" prefix
    /// routes classify_error to exit code 2.
    pub async fn execute(&self) -> Result<()> {
        let (verb, hint) = match self {
            Self::Stats { .. } => (
                "stats",
                "query the running daemon instead: JSON-RPC method `cache.stats` \
                 (via the @crewchief/daemon-client or a raw `maproom serve` stdio session)",
            ),
            Self::Clear { .. } | Self::Invalidate { .. } => (
                "clear/invalidate",
                "restart the `maproom serve` daemon to drop its cache; entries also \
                 expire automatically (60s TTL)",
            ),
            Self::Warm { .. } => (
                "warm",
                "warm the DAEMON's cache instead: start it with \
                 `maproom serve --warm-queries <file> --warm-repo <name>`, or call the \
                 JSON-RPC method `cache.warm` on a running daemon",
            ),
            Self::Maintenance { .. } => (
                "maintenance",
                "the daemon expires entries via TTL automatically; no CLI-side \
                 maintenance exists",
            ),
        };
        anyhow::bail!(
            "Configuration error: `maproom cache {verb}` cannot operate on the query \
             cache — the cache lives inside the `maproom serve` daemon process, and a \
             CLI-side cache would die with this process (the previous implementation \
             fabricated success against a throwaway in-memory cache). To {verb}: {hint}."
        )
    }
}
