# INC_INDEX Architecture: Incremental Indexing Pipeline

## System Design

### Core Architecture
```
File System Events → Change Detector → Update Queue → Processor → Database
         ↓                   ↓             ↓            ↓
      Watcher            Hashing      Prioritization  Incremental
```

## Components

### 1. File Watcher
```rust
use notify::{Watcher, RecursiveMode, watcher};

pub struct FileWatcher {
    watcher: RecommendedWatcher,
    tx: mpsc::Sender<FileEvent>,
    ignore_patterns: GlobSet,
}

impl FileWatcher {
    pub fn watch(&mut self, path: &Path) -> Result<()> {
        self.watcher.watch(path, RecursiveMode::Recursive)?;

        // Event processing loop
        loop {
            match rx.recv() {
                Ok(DebouncedEvent::Write(path)) |
                Ok(DebouncedEvent::Create(path)) => {
                    if !self.should_ignore(&path) {
                        self.tx.send(FileEvent::Modified(path))?;
                    }
                },
                Ok(DebouncedEvent::Remove(path)) => {
                    self.tx.send(FileEvent::Deleted(path))?;
                },
                Ok(DebouncedEvent::Rename(from, to)) => {
                    self.tx.send(FileEvent::Renamed(from, to))?;
                }
            }
        }
    }
}
```

### 2. Change Detector
```rust
pub struct ChangeDetector {
    hash_cache: HashMap<PathBuf, ContentHash>,
    db: DatabaseConnection,
}

impl ChangeDetector {
    pub async fn detect_changes(&mut self, path: &Path) -> ChangeType {
        let new_hash = self.hash_file(path).await?;

        // Check against cache
        if let Some(old_hash) = self.hash_cache.get(path) {
            if old_hash == &new_hash {
                return ChangeType::None;
            }
        }

        // Check against database
        let db_hash = self.get_db_hash(path).await?;

        match (db_hash, new_hash) {
            (None, hash) => ChangeType::New(hash),
            (Some(old), new) if old != new => ChangeType::Modified(old, new),
            _ => ChangeType::None,
        }
    }

    fn hash_file(&self, path: &Path) -> ContentHash {
        let content = fs::read(path)?;
        blake3::hash(&content)
    }
}
```

### 3. Update Queue
```rust
pub struct UpdateQueue {
    queue: PriorityQueue<UpdateTask>,
    processing: HashSet<PathBuf>,
}

impl UpdateQueue {
    pub fn enqueue(&mut self, task: UpdateTask) {
        let priority = self.calculate_priority(&task);

        // Dedup and merge
        if let Some(existing) = self.queue.get_mut(&task.path) {
            existing.merge(task);
        } else {
            self.queue.push(task, priority);
        }
    }

    fn calculate_priority(&self, task: &UpdateTask) -> Priority {
        // Recent files higher priority
        // Active worktrees higher priority
        // User-triggered higher than auto
        match task.trigger {
            Trigger::User => Priority::High,
            Trigger::Save => Priority::Medium,
            Trigger::Auto => Priority::Low,
        }
    }
}
```

### 4. Incremental Processor
```rust
pub struct IncrementalProcessor {
    parser_factory: ParserFactory,
    db: DatabaseConnection,
}

impl IncrementalProcessor {
    pub async fn process(&self, task: UpdateTask) -> Result<()> {
        match task.change_type {
            ChangeType::New(hash) => {
                self.index_new_file(&task.path, hash).await?
            },
            ChangeType::Modified(old_hash, new_hash) => {
                self.update_file(&task.path, old_hash, new_hash).await?
            },
            ChangeType::Deleted => {
                self.remove_file(&task.path).await?
            },
        }

        // Update relationships
        self.update_edges(&task.path).await?;

        Ok(())
    }

    async fn update_file(&self, path: &Path,
                        old_hash: ContentHash,
                        new_hash: ContentHash) -> Result<()> {
        // Start transaction
        let tx = self.db.begin().await?;

        // Delete old chunks
        sqlx::query!("DELETE FROM maproom.chunks
                     WHERE file_id IN (
                       SELECT id FROM maproom.files
                       WHERE relpath = $1 AND content_hash = $2
                     )", path, old_hash)
            .execute(&tx).await?;

        // Parse and insert new chunks
        let chunks = self.parse_file(path)?;
        for chunk in chunks {
            self.insert_chunk(&tx, chunk).await?;
        }

        // Update file record
        sqlx::query!("UPDATE maproom.files
                     SET content_hash = $1, last_modified = NOW()
                     WHERE relpath = $2",
                     new_hash, path)
            .execute(&tx).await?;

        tx.commit().await?;
        Ok(())
    }
}
```

### 5. Edge Updater
```rust
pub struct EdgeUpdater {
    db: DatabaseConnection,
}

impl EdgeUpdater {
    pub async fn update_edges(&self, changed_file: &Path) -> Result<()> {
        // Find affected chunks
        let chunks = self.get_file_chunks(changed_file).await?;

        // Delete old edges
        for chunk_id in &chunks {
            sqlx::query!("DELETE FROM maproom.chunk_edges
                         WHERE src_chunk_id = $1 OR dst_chunk_id = $1",
                         chunk_id)
                .execute(&self.db).await?;
        }

        // Recompute edges
        let new_edges = self.compute_edges(&chunks).await?;

        // Insert new edges
        for edge in new_edges {
            self.insert_edge(edge).await?;
        }

        Ok(())
    }
}
```

### 6. Watch Command
```rust
pub struct WatchCommand {
    watcher: FileWatcher,
    detector: ChangeDetector,
    queue: Arc<Mutex<UpdateQueue>>,
    processor: IncrementalProcessor,
}

impl WatchCommand {
    pub async fn run(&mut self, worktree: &str) -> Result<()> {
        let path = self.get_worktree_path(worktree)?;

        // Start watcher thread
        let queue = self.queue.clone();
        thread::spawn(move || {
            self.watcher.watch(&path, queue)
        });

        // Process queue
        loop {
            let task = self.queue.lock().await.pop();

            if let Some(task) = task {
                // Process with retry
                for attempt in 0..3 {
                    if self.processor.process(task).await.is_ok() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                }
            } else {
                // No tasks, wait
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}
```

### 7. Merkle Tree Optimization
```rust
pub struct MerkleIndex {
    tree: HashMap<PathBuf, MerkleNode>,
}

struct MerkleNode {
    hash: ContentHash,
    children: Vec<PathBuf>,
    parent: Option<PathBuf>,
}

impl MerkleIndex {
    pub fn find_changed(&self, new_tree: &MerkleIndex) -> Vec<PathBuf> {
        let mut changed = Vec::new();

        for (path, node) in &self.tree {
            if let Some(new_node) = new_tree.tree.get(path) {
                if node.hash != new_node.hash {
                    changed.push(path.clone());
                }
            } else {
                // Deleted
                changed.push(path.clone());
            }
        }

        // Find new files
        for path in new_tree.tree.keys() {
            if !self.tree.contains_key(path) {
                changed.push(path.clone());
            }
        }

        changed
    }
}
```

## Database Schema

```sql
-- Add indexing metadata
ALTER TABLE maproom.files ADD COLUMN indexed_at TIMESTAMPTZ;
ALTER TABLE maproom.files ADD COLUMN index_version INT DEFAULT 1;

-- Queue for pending updates
CREATE TABLE maproom.index_queue (
  id BIGSERIAL PRIMARY KEY,
  worktree_id BIGINT REFERENCES maproom.worktrees(id),
  file_path TEXT NOT NULL,
  change_type TEXT NOT NULL,
  priority INT DEFAULT 0,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  processing_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  error TEXT
);

CREATE INDEX idx_queue_pending ON maproom.index_queue(priority DESC)
  WHERE completed_at IS NULL AND processing_at IS NULL;
```

## Configuration

```yaml
incremental:
  watch:
    debounce_ms: 500
    ignore_patterns:
      - "*.log"
      - ".git/**"
      - "node_modules/**"
      - "dist/**"

  processing:
    batch_size: 10
    parallel_workers: 4
    retry_attempts: 3
    retry_delay_ms: 1000

  merkle:
    enabled: false  # Experimental
    checkpoint_interval: 300  # seconds
```

## Performance Considerations

- Debounce file events (500ms default)
- Batch process changes
- Priority queue for important files
- Parallel processing where possible
- Connection pooling for database
- Periodic garbage collection

## Error Handling

- Failed updates go to dead letter queue
- Automatic retry with exponential backoff
- Periodic reconciliation with full scan
- Alert on repeated failures
- Graceful degradation to full indexing