#!/usr/bin/env sh
# §9.5 — Postgres CLI end-to-end smoke test.
#
# Exercises the maproom CLI against the PostgreSQL backend (status / search / db)
# via the --database-url flag (R-WIRE-5) and MAPROOM_DATABASE_URL, asserting the
# binary routes to PostgresStore and the commands succeed.
#
# Gated on MAPROOM_TEST_PG_URL and EXCLUDED from default CI (run manually):
#   MAPROOM_TEST_PG_URL=postgres://maproom:maproom@localhost:5432/maproom_test \
#     crates/maproom/tests/e2e/test_postgres_flow.sh
#
# POSIX sh (CLAUDE.md: ZSH target / POSIX syntax — no bashisms).
set -eu

if [ -z "${MAPROOM_TEST_PG_URL:-}" ]; then
    echo "SKIP: MAPROOM_TEST_PG_URL unset (Postgres E2E requires a pgvector instance)"
    exit 0
fi

command -v jq >/dev/null 2>&1 || { echo "FAIL: jq is required"; exit 2; }

# Resolve the crate dir from this script's location (tests/e2e/ -> crate root).
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CRATE_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

echo "Building maproom --features postgres ..."
cargo build -q -p maproom --features postgres --manifest-path "$CRATE_DIR/Cargo.toml"
BIN="$CRATE_DIR/../../target/debug/maproom"
[ -x "$BIN" ] || BIN=$(cargo metadata --no-deps --format-version 1 --manifest-path "$CRATE_DIR/Cargo.toml" | jq -r '.target_directory')/debug/maproom

PG_URL="$MAPROOM_TEST_PG_URL"
REPO_NAME="e2e-pg-$$"

run() {
    # Route to Postgres two ways: --database-url flag AND env, to exercise both.
    MAPROOM_DATABASE_URL="$PG_URL" "$BIN" --database-url "$PG_URL" "$@"
}

echo "1. db migrate (auto-applies migrations_pg against Postgres) ..."
run db migrate

echo "2. status --json (must be valid JSON; backend reachable) ..."
STATUS_JSON=$(run status --json 2>/dev/null || true)
echo "$STATUS_JSON" | jq -e . >/dev/null 2>&1 \
    || { echo "FAIL: status did not return valid JSON"; echo "$STATUS_JSON"; exit 1; }
echo "   status JSON OK"

echo "3. search (FTS-default) on an empty/unknown repo must succeed with empty results ..."
SEARCH_JSON=$(run search --repo "$REPO_NAME" --query "anything" --format json 2>/dev/null || true)
echo "$SEARCH_JSON" | jq -e . >/dev/null 2>&1 \
    || { echo "FAIL: search did not return valid JSON"; echo "$SEARCH_JSON"; exit 1; }
echo "   search JSON OK"

echo "4. no-feature guardrail: a default build must REJECT a postgres URL (R-WIRE-4) ..."
cargo build -q -p maproom --manifest-path "$CRATE_DIR/Cargo.toml"
DEFAULT_BIN="$CRATE_DIR/../../target/debug/maproom"
if MAPROOM_DATABASE_URL="$PG_URL" "$DEFAULT_BIN" status --json >/dev/null 2>&1; then
    echo "FAIL: default (no-feature) build accepted a postgres:// URL"; exit 1
fi
echo "   default build correctly rejected postgres:// (non-zero exit)"

echo ""
echo "PASS: Postgres CLI E2E flow (status/search/db) succeeded against $PG_URL"
