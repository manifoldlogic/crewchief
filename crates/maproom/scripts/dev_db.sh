#!/usr/bin/env bash
set -euo pipefail

DB_NAME=${1:-maproom}

createdb "$DB_NAME" || true
psql "$DB_NAME" -v ON_ERROR_STOP=1 -f "$(dirname "$0")/../migrations/0001_init.sql"
psql "$DB_NAME" -v ON_ERROR_STOP=1 -f "$(dirname "$0")/analyze.sql"
echo "Database $DB_NAME initialized."


