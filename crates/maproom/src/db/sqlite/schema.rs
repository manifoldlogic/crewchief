use anyhow::Result;
use rusqlite::Connection;

pub fn init_schema(conn: &Connection) -> Result<()> {
    // Create Repositories table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS repos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            root_path TEXT NOT NULL
        )",
        [],
    )?;

    // Create Worktrees table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS worktrees (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            abs_path TEXT NOT NULL,
            UNIQUE(repo_id, name)
        )",
        [],
    )?;

    // Create Commits table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS commits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
            sha TEXT NOT NULL,
            committed_at DATETIME,
            UNIQUE(repo_id, sha)
        )",
        [],
    )?;

    // Create Files table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
            worktree_id INTEGER NOT NULL REFERENCES worktrees(id) ON DELETE CASCADE,
            commit_id INTEGER NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
            relpath TEXT NOT NULL,
            language TEXT,
            content_hash TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            last_modified DATETIME,
            UNIQUE(commit_id, relpath, content_hash)
        )",
        [],
    )?;

    // Create Chunks table
    // Note: worktree_ids is a JSON array in SQLite
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chunks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
            blob_sha TEXT NOT NULL,
            symbol_name TEXT,
            kind TEXT NOT NULL,
            signature TEXT,
            docstring TEXT,
            start_line INTEGER NOT NULL,
            end_line INTEGER NOT NULL,
            preview TEXT NOT NULL,
            ts_doc_text TEXT,
            recency_score REAL NOT NULL,
            churn_score REAL NOT NULL,
            metadata JSON,
            worktree_ids JSON NOT NULL,
            UNIQUE(file_id, start_line, end_line)
        )",
        [],
    )?;

    // Create Chunk Edges table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chunk_edges (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            src_chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
            dst_chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
            type TEXT NOT NULL,
            UNIQUE(src_chunk_id, dst_chunk_id, type)
        )",
        [],
    )?;

    // Create Vector Table using vec0
    // We assume sqlite-vec is loaded.
    // Embedding is 1536 dimensions (float32)
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS vec_chunks USING vec0(
            chunk_id INTEGER PRIMARY KEY,
            code_embedding float[1536],
            text_embedding float[1536]
        )",
        [],
    )?;

    // Create FTS5 Table for code search
    // We use 'trigram' tokenizer if available, else standard
    // For MVP, we start with standard tokenizer to ensure compatibility
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS fts_chunks USING fts5(
            content,
            docstring,
            symbol_name,
            content='chunks',
            content_rowid='id'
        )",
        [],
    )?;

    // Triggers to keep FTS updated? 
    // Actually, for FTS5 external content tables, we need to manage updates manually or use triggers.
    // Manual updates are often safer/more controlled in application logic.
    // But triggers are standard for FTS5 content tables.
    // Let's rely on application logic to update FTS for now (to match Postgres architecture where we update tsvector column).
    // Wait, Postgres uses a generated column or explicit update. 
    // SQLite FTS5 content= option means it reads from the table, but we still need to INSERT into the fts table to rebuild index
    // 'rebuild' command is needed. Or we use triggers.
    // Let's stick to manual inserts for now in `insert_chunk`.

    Ok(())
}

