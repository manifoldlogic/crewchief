//! Integration tests for large batch processing in incremental indexing.
//!
//! Tests large-scale operations:
//! - Processing 1000+ file changes in a single batch
//! - Measuring indexing throughput (files/second)
//! - Monitoring memory usage patterns
//! - Verifying index accuracy after large batches
//! - Performance benchmarking and regression detection
//!
//! These tests validate that the incremental indexing system can handle
//! real-world scenarios like git checkouts with thousands of file changes.

use anyhow::Result;
use crewchief_maproom::db::{create_pool, PgPool};
use crewchief_maproom::incremental::{
    ChangeType, FileHasher, IncrementalProcessor, UpdateTask, Trigger,
};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use tempfile::TempDir;
use tokio;

/// Test helper for batch processing.
struct BatchTestRepo {
    temp_dir: TempDir,
    pool: PgPool,
    repo_id: i64,
    worktree_id: i64,
    commit_id: i64,
}

impl BatchTestRepo {
    /// Create a new test repository.
    async fn new() -> Result<Self> {
        let _ = dotenvy::dotenv();
        let temp_dir = TempDir::new()?;
        let pool = create_pool().await?;

        Self::setup_schema(&pool).await?;
        let (repo_id, worktree_id, commit_id) =
            Self::create_test_entities(&pool, temp_dir.path()).await?;

        Ok(Self {
            temp_dir,
            pool,
            repo_id,
            worktree_id,
            commit_id,
        })
    }

    /// Set up database schema.
    async fn setup_schema(pool: &PgPool) -> Result<()> {
        let client = pool.get().await?;

        client
            .batch_execute(
                r#"
                CREATE SCHEMA IF NOT EXISTS maproom;

                CREATE TABLE IF NOT EXISTS maproom.repos (
                    id BIGSERIAL PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    root_path TEXT NOT NULL,
                    created_at TIMESTAMPTZ DEFAULT NOW()
                );

                CREATE TABLE IF NOT EXISTS maproom.worktrees (
                    id BIGSERIAL PRIMARY KEY,
                    repo_id BIGINT NOT NULL REFERENCES maproom.repos(id),
                    name TEXT NOT NULL,
                    abs_path TEXT NOT NULL,
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(repo_id, name)
                );

                CREATE TABLE IF NOT EXISTS maproom.commits (
                    id BIGSERIAL PRIMARY KEY,
                    repo_id BIGINT NOT NULL REFERENCES maproom.repos(id),
                    sha TEXT NOT NULL,
                    committed_at TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(repo_id, sha)
                );

                CREATE TABLE IF NOT EXISTS maproom.files (
                    id BIGSERIAL PRIMARY KEY,
                    repo_id BIGINT NOT NULL REFERENCES maproom.repos(id),
                    worktree_id BIGINT NOT NULL REFERENCES maproom.worktrees(id),
                    commit_id BIGINT NOT NULL REFERENCES maproom.commits(id),
                    relpath TEXT NOT NULL,
                    language TEXT,
                    blake3_hash TEXT,
                    size_bytes BIGINT,
                    last_modified TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(commit_id, relpath, blake3_hash)
                );

                CREATE INDEX IF NOT EXISTS idx_files_worktree ON maproom.files(worktree_id);
                CREATE INDEX IF NOT EXISTS idx_files_hash ON maproom.files(blake3_hash);

                CREATE TABLE IF NOT EXISTS maproom.chunks (
                    id BIGSERIAL PRIMARY KEY,
                    file_id BIGINT NOT NULL REFERENCES maproom.files(id) ON DELETE CASCADE,
                    chunk_index INTEGER NOT NULL,
                    content TEXT NOT NULL,
                    start_line INTEGER NOT NULL,
                    end_line INTEGER NOT NULL,
                    chunk_type TEXT NOT NULL,
                    language TEXT,
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(file_id, chunk_index)
                );

                CREATE INDEX IF NOT EXISTS idx_chunks_file ON maproom.chunks(file_id);

                CREATE TABLE IF NOT EXISTS maproom.chunk_edges (
                    id BIGSERIAL PRIMARY KEY,
                    src_chunk_id BIGINT NOT NULL REFERENCES maproom.chunks(id) ON DELETE CASCADE,
                    dst_chunk_id BIGINT NOT NULL REFERENCES maproom.chunks(id) ON DELETE CASCADE,
                    edge_type TEXT NOT NULL,
                    weight DOUBLE PRECISION DEFAULT 1.0,
                    created_at TIMESTAMPTZ DEFAULT NOW()
                );

                CREATE INDEX IF NOT EXISTS idx_edges_src ON maproom.chunk_edges(src_chunk_id);
                CREATE INDEX IF NOT EXISTS idx_edges_dst ON maproom.chunk_edges(dst_chunk_id);
                "#,
            )
            .await?;

        Ok(())
    }

    /// Create test entities.
    async fn create_test_entities(
        pool: &PgPool,
        repo_path: &std::path::Path,
    ) -> Result<(i64, i64, i64)> {
        let client = pool.get().await?;

        let repo_row = client
            .query_one(
                "INSERT INTO maproom.repos(name, root_path)
                 VALUES ($1, $2)
                 RETURNING id",
                &[&"batch_test_repo", &repo_path.to_string_lossy().as_ref()],
            )
            .await?;
        let repo_id: i64 = repo_row.get(0);

        let worktree_row = client
            .query_one(
                "INSERT INTO maproom.worktrees(repo_id, name, abs_path)
                 VALUES ($1, $2, $3)
                 RETURNING id",
                &[&repo_id, &"main", &repo_path.to_string_lossy().as_ref()],
            )
            .await?;
        let worktree_id: i64 = worktree_row.get(0);

        let commit_row = client
            .query_one(
                "INSERT INTO maproom.commits(repo_id, sha, committed_at)
                 VALUES ($1, $2, NOW())
                 RETURNING id",
                &[&repo_id, &"batch_test_commit"],
            )
            .await?;
        let commit_id: i64 = commit_row.get(0);

        Ok((repo_id, worktree_id, commit_id))
    }

    /// Generate a batch of files with realistic content.
    fn generate_batch_files(&self, count: usize) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();

        for i in 0..count {
            let dir_num = i / 100; // Group into directories of 100 files
            let relpath = format!("src/batch_{}/file_{}.ts", dir_num, i);
            let path = self.temp_dir.path().join(&relpath);

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Generate realistic TypeScript content
            let content = format!(
                r#"// File {}
import {{ Helper }} from '../utils/helper';

export interface Config {{
    enabled: boolean;
    threshold: number;
    metadata: Record<string, unknown>;
}}

export class Processor {{
    private config: Config;

    constructor(config: Config) {{
        this.config = config;
    }}

    public async process(data: string): Promise<string> {{
        if (!this.config.enabled) {{
            return data;
        }}

        const helper = new Helper();
        const result = await helper.transform(data);

        if (result.length > this.config.threshold) {{
            console.warn('Result exceeds threshold');
        }}

        return result;
    }}

    public getConfig(): Config {{
        return {{ ...this.config }};
    }}
}}

export const DEFAULT_CONFIG: Config = {{
    enabled: true,
    threshold: 1000,
    metadata: {{ version: '1.0.0' }},
}};
"#,
                i
            );

            fs::write(&path, content)?;
            paths.push(path);
        }

        Ok(paths)
    }

    /// Count total files in database.
    async fn count_files(&self) -> Result<i64> {
        let client = self.pool.get().await?;
        let row = client
            .query_one(
                "SELECT COUNT(*) FROM maproom.files WHERE worktree_id = $1",
                &[&self.worktree_id],
            )
            .await?;
        Ok(row.get(0))
    }

    /// Count total chunks.
    async fn count_all_chunks(&self) -> Result<i64> {
        let client = self.pool.get().await?;
        let row = client
            .query_one(
                "SELECT COUNT(*) FROM maproom.chunks c
                 JOIN maproom.files f ON c.file_id = f.id
                 WHERE f.worktree_id = $1",
                &[&self.worktree_id],
            )
            .await?;
        Ok(row.get(0))
    }

    /// Clean up test data.
    async fn cleanup(&self) -> Result<()> {
        let client = self.pool.get().await?;

        client
            .execute(
                "DELETE FROM maproom.files WHERE repo_id = $1",
                &[&self.repo_id],
            )
            .await?;

        client
            .execute(
                "DELETE FROM maproom.commits WHERE repo_id = $1",
                &[&self.repo_id],
            )
            .await?;

        client
            .execute(
                "DELETE FROM maproom.worktrees WHERE repo_id = $1",
                &[&self.repo_id],
            )
            .await?;

        client
            .execute("DELETE FROM maproom.repos WHERE id = $1", &[&self.repo_id])
            .await?;

        Ok(())
    }
}

/// Performance metrics for batch operations.
struct BatchMetrics {
    total_files: usize,
    total_duration_secs: f64,
    throughput_files_per_sec: f64,
    avg_time_per_file_ms: f64,
}

impl BatchMetrics {
    fn new(total_files: usize, duration: std::time::Duration) -> Self {
        let duration_secs = duration.as_secs_f64();
        let throughput = total_files as f64 / duration_secs;
        let avg_per_file = (duration_secs * 1000.0) / total_files as f64;

        Self {
            total_files,
            total_duration_secs: duration_secs,
            throughput_files_per_sec: throughput,
            avg_time_per_file_ms: avg_per_file,
        }
    }

    fn print(&self) {
        println!("\n=== Batch Processing Metrics ===");
        println!("Total files:           {}", self.total_files);
        println!("Total duration:        {:.2}s", self.total_duration_secs);
        println!("Throughput:            {:.2} files/sec", self.throughput_files_per_sec);
        println!("Avg time per file:     {:.2}ms", self.avg_time_per_file_ms);
        println!("================================\n");
    }
}

#[tokio::test]
async fn test_batch_1000_files() {
    let repo = BatchTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    let batch_size = 1000;
    println!("Generating {} files...", batch_size);

    let paths = repo
        .generate_batch_files(batch_size)
        .expect("Failed to generate batch files");

    println!("Processing batch of {} files...", batch_size);
    let start = Instant::now();

    // Process all files
    for path in &paths {
        let hash = FileHasher::hash_file(path).expect("Failed to hash file");
        let task = UpdateTask::new(
            path.clone(),
            ChangeType::New(hash),
            Trigger::Save,
        );

        processor
            .process(task)
            .await
            .expect("Failed to process file");
    }

    let duration = start.elapsed();
    let metrics = BatchMetrics::new(batch_size, duration);
    metrics.print();

    // Verify all files were indexed
    let file_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(
        file_count, batch_size as i64,
        "Not all files were indexed"
    );

    // Verify chunks were created
    let chunk_count = repo.count_all_chunks().await.expect("Failed to count chunks");
    assert!(
        chunk_count >= batch_size as i64,
        "Expected at least {} chunks, found {}",
        batch_size,
        chunk_count
    );

    // Performance assertion: should process at least 10 files/sec
    assert!(
        metrics.throughput_files_per_sec >= 10.0,
        "Throughput too low: {:.2} files/sec (expected >= 10)",
        metrics.throughput_files_per_sec
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
#[ignore] // Run with `cargo test --ignored` for extended tests
async fn test_batch_5000_files() {
    let repo = BatchTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    let batch_size = 5000;
    println!("Generating {} files...", batch_size);

    let paths = repo
        .generate_batch_files(batch_size)
        .expect("Failed to generate batch files");

    println!("Processing batch of {} files...", batch_size);
    let start = Instant::now();

    // Process all files
    for path in &paths {
        let hash = FileHasher::hash_file(path).expect("Failed to hash file");
        let task = UpdateTask::new(
            path.clone(),
            ChangeType::New(hash),
            Trigger::Save,
        );

        processor
            .process(task)
            .await
            .expect("Failed to process file");
    }

    let duration = start.elapsed();
    let metrics = BatchMetrics::new(batch_size, duration);
    metrics.print();

    // Verify all files were indexed
    let file_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(
        file_count, batch_size as i64,
        "Not all files were indexed"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_batch_modifications() {
    let repo = BatchTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    let batch_size = 500;

    // First, create all files
    println!("Creating {} files...", batch_size);
    let paths = repo
        .generate_batch_files(batch_size)
        .expect("Failed to generate files");

    for path in &paths {
        let hash = FileHasher::hash_file(path).expect("Failed to hash file");
        let task = UpdateTask::new(
            path.clone(),
            ChangeType::New(hash),
            Trigger::Save,
        );
        processor
            .process(task)
            .await
            .expect("Failed to create file");
    }

    let initial_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(initial_count, batch_size as i64);

    // Now modify all files in a batch
    println!("Modifying {} files...", batch_size);
    let start = Instant::now();

    for (i, path) in paths.iter().enumerate() {
        // Read old content for hash
        let old_content = fs::read_to_string(path).expect("Failed to read file");
        let old_hash = FileHasher::hash_bytes(old_content.as_bytes());

        // Modify content
        let new_content = format!("{}\n// Modified at index {}\n", old_content, i);
        fs::write(path, &new_content).expect("Failed to write file");
        let new_hash = FileHasher::hash_file(path).expect("Failed to hash file");

        // Process modification
        let task = UpdateTask::new(
            path.clone(),
            ChangeType::Modified {
                old: old_hash,
                new: new_hash,
            },
            Trigger::Save,
        );
        processor
            .process(task)
            .await
            .expect("Failed to process modification");
    }

    let duration = start.elapsed();
    let metrics = BatchMetrics::new(batch_size, duration);
    println!("\n=== Batch Modification Metrics ===");
    metrics.print();

    // Verify file count unchanged
    let final_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(
        final_count, batch_size as i64,
        "File count changed during modifications"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_batch_deletions() {
    let repo = BatchTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    let batch_size = 500;

    // Create files
    println!("Creating {} files for deletion test...", batch_size);
    let paths = repo
        .generate_batch_files(batch_size)
        .expect("Failed to generate files");

    let mut file_hashes = Vec::new();
    for path in &paths {
        let hash = FileHasher::hash_file(path).expect("Failed to hash file");
        file_hashes.push(hash.clone());

        let task = UpdateTask::new(
            path.clone(),
            ChangeType::New(hash),
            Trigger::Save,
        );
        processor
            .process(task)
            .await
            .expect("Failed to create file");
    }

    let initial_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(initial_count, batch_size as i64);

    // Delete all files in batch
    println!("Deleting {} files...", batch_size);
    let start = Instant::now();

    for (path, hash) in paths.iter().zip(file_hashes.iter()) {
        let task = UpdateTask::new(
            path.clone(),
            ChangeType::Deleted(hash.clone()),
            Trigger::Save,
        );
        processor
            .process(task)
            .await
            .expect("Failed to process deletion");
    }

    let duration = start.elapsed();
    let metrics = BatchMetrics::new(batch_size, duration);
    println!("\n=== Batch Deletion Metrics ===");
    metrics.print();

    // Verify all files deleted
    let final_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(final_count, 0, "Not all files were deleted");

    // Verify all chunks deleted (CASCADE)
    let chunk_count = repo.count_all_chunks().await.expect("Failed to count chunks");
    assert_eq!(chunk_count, 0, "Chunks not deleted with files");

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_batch_accuracy() {
    let repo = BatchTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    let batch_size = 200;

    // Create and index files
    let paths = repo
        .generate_batch_files(batch_size)
        .expect("Failed to generate files");

    for path in &paths {
        let hash = FileHasher::hash_file(path).expect("Failed to hash file");
        let task = UpdateTask::new(
            path.clone(),
            ChangeType::New(hash),
            Trigger::Save,
        );
        processor
            .process(task)
            .await
            .expect("Failed to process file");
    }

    // Verify index accuracy
    let client = repo.pool.get().await.expect("Failed to get client");

    // Check that all files have valid hashes stored
    let files_with_hash: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.files
             WHERE worktree_id = $1 AND blake3_hash IS NOT NULL AND blake3_hash != ''",
            &[&repo.worktree_id],
        )
        .await
        .expect("Failed to count files with hash")
        .get(0);

    assert_eq!(
        files_with_hash, batch_size as i64,
        "Not all files have valid hashes"
    );

    // Verify all files have chunks
    let files_without_chunks: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.files f
             WHERE f.worktree_id = $1
             AND NOT EXISTS (
                 SELECT 1 FROM maproom.chunks c WHERE c.file_id = f.id
             )",
            &[&repo.worktree_id],
        )
        .await
        .expect("Failed to count files without chunks")
        .get(0);

    assert_eq!(
        files_without_chunks, 0,
        "Found {} files without chunks",
        files_without_chunks
    );

    // Verify chunk integrity (all chunks reference valid files)
    let invalid_chunks: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks c
             WHERE c.file_id NOT IN (SELECT id FROM maproom.files)",
            &[],
        )
        .await
        .expect("Failed to count invalid chunks")
        .get(0);

    assert_eq!(invalid_chunks, 0, "Found invalid chunk references");

    repo.cleanup().await.expect("Failed to cleanup");
}

#[tokio::test]
async fn test_batch_memory_usage() {
    // Note: This test verifies the system can handle large batches without OOM
    // Actual memory measurement would require platform-specific APIs

    let repo = BatchTestRepo::new()
        .await
        .expect("Failed to create test repo");
    let processor = IncrementalProcessor::new(repo.pool.clone(), repo.temp_dir.path().to_path_buf());

    let batch_size = 1000;

    // Generate and process files one at a time to avoid holding all in memory
    println!("Testing memory usage with {} files...", batch_size);

    for i in 0..batch_size {
        let relpath = format!("src/memory_test/file_{}.ts", i);
        let path = repo.temp_dir.path().join(&relpath);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create directory");
        }

        let content = format!("export const value = {};", i);
        fs::write(&path, content).expect("Failed to write file");

        let hash = FileHasher::hash_file(&path).expect("Failed to hash file");
        let task = UpdateTask::new(path, ChangeType::New(hash), Trigger::Save);

        processor
            .process(task)
            .await
            .expect("Failed to process file");

        // If we reach here without OOM, memory usage is acceptable
    }

    let file_count = repo.count_files().await.expect("Failed to count files");
    assert_eq!(
        file_count, batch_size as i64,
        "Memory test: not all files processed"
    );

    repo.cleanup().await.expect("Failed to cleanup");
}
