# crewchief-maproom

Rust indexer + CLI for Maproom. Stores AST-aware chunks and metadata into Postgres with pgvector.

## Setup

1. Install Postgres with `vector`, `pg_trgm`, and `unaccent` extensions.
2. Create a DB and apply migrations:

```
createdb maproom
psql maproom -f migrations/0001_init.sql
psql maproom -f scripts/analyze.sql
```

Or via CLI:

```
export DATABASE_URL=postgres://USER:PASSWORD@localhost:5432/maproom
cargo run -p crewchief-maproom -- db migrate
```

## Usage

```
cargo run -p crewchief-maproom -- scan \
  --repo crewchief \
  --worktree radar \
  --path /path/to/worktree \
  --commit $(git rev-parse HEAD)
```

## Env

- `DATABASE_URL` (recommended set via `.env`)

Create a `.env` at the repo root or in `crates/maproom/` by copying `.env.example`:

```
cp crates/maproom/.env.example .env
# or
cp crates/maproom/.env.example crates/maproom/.env
```

Then edit the password:

```
DATABASE_URL=postgres://<your_usernamew >:<your_password>@localhost:5432/maproom
```
