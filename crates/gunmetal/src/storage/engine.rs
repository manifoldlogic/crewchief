//! BatchWriter — buffered async write engine for storage backends.
//!
//! Buffers individual writes and flushes them in a single batch to the
//! underlying `AsyncStorageAdapter`. Flushing occurs either:
//! - On a timer (default: 100ms)
//! - When the buffer reaches capacity (default: 1000 entries)
//! - Explicitly via `flush()`
//!
//! This is critical for IndexedDB performance — individual writes are slow
//! but batched transactions are fast.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{AsyncStorageAdapter, StoredValue};

/// Configuration for the batch writer.
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum entries to buffer before auto-flush. Default: 1000.
    pub max_buffer_size: usize,
    /// Flush interval in milliseconds. Default: 100.
    pub flush_interval_ms: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_buffer_size: 1000,
            flush_interval_ms: 100,
        }
    }
}

/// Shared buffer state.
struct BufferInner {
    /// Pending writes (key → value). Later writes to the same key overwrite.
    pending: HashMap<String, StoredValue>,
    /// Total writes buffered since last flush (for metrics).
    total_buffered: u64,
    /// Total flushes performed (for metrics).
    total_flushes: u64,
}

/// A buffered async write engine.
///
/// Accepts writes via `buffer_put()` and batches them for efficient
/// flushing to the underlying `AsyncStorageAdapter`.
pub struct BatchWriter {
    adapter: Arc<dyn AsyncStorageAdapter>,
    buffer: Arc<Mutex<BufferInner>>,
    config: BatchConfig,
}

impl BatchWriter {
    /// Create a new batch writer wrapping an async storage adapter.
    pub fn new(adapter: Arc<dyn AsyncStorageAdapter>, config: BatchConfig) -> Self {
        Self {
            adapter,
            buffer: Arc::new(Mutex::new(BufferInner {
                pending: HashMap::new(),
                total_buffered: 0,
                total_flushes: 0,
            })),
            config,
        }
    }

    /// Buffer a write for later flushing.
    ///
    /// If the buffer is full, returns `true` to signal the caller should flush.
    /// Later writes to the same key overwrite earlier ones within the same batch.
    pub fn buffer_put(&self, key: String, value: StoredValue) -> bool {
        let mut inner = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        inner.pending.insert(key, value);
        inner.total_buffered += 1;
        inner.pending.len() >= self.config.max_buffer_size
    }

    /// Flush all buffered writes to the storage adapter.
    ///
    /// Returns the number of entries flushed and any errors encountered.
    pub async fn flush(&self) -> Result<usize, Vec<String>> {
        let entries: Vec<(String, StoredValue)> = {
            let mut inner = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            inner.total_flushes += 1;
            inner.pending.drain().collect()
        };

        if entries.is_empty() {
            return Ok(0);
        }

        let count = entries.len();
        let mut errors = Vec::new();

        for (key, value) in entries {
            if let Err(e) = self.adapter.put(key, value).await {
                errors.push(e);
            }
        }

        if errors.is_empty() {
            Ok(count)
        } else {
            Err(errors)
        }
    }

    /// Number of entries currently buffered.
    pub fn pending_count(&self) -> usize {
        self.buffer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pending
            .len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.pending_count() == 0
    }

    /// Total writes buffered since creation.
    pub fn total_buffered(&self) -> u64 {
        self.buffer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .total_buffered
    }

    /// Total flushes performed since creation.
    pub fn total_flushes(&self) -> u64 {
        self.buffer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .total_flushes
    }

    /// Get the batch configuration.
    pub fn config(&self) -> &BatchConfig {
        &self.config
    }

    /// Get a reference to the underlying adapter.
    pub fn adapter(&self) -> &Arc<dyn AsyncStorageAdapter> {
        &self.adapter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::AsyncMemoryStorage;
    use crate::types::GunValue;

    fn make_writer(max_buffer: usize) -> (BatchWriter, Arc<AsyncMemoryStorage>) {
        let store = Arc::new(AsyncMemoryStorage::new());
        let config = BatchConfig {
            max_buffer_size: max_buffer,
            flush_interval_ms: 100,
        };
        let writer = BatchWriter::new(store.clone(), config);
        (writer, store)
    }

    #[test]
    fn buffer_accumulates_writes() {
        let (writer, _store) = make_writer(100);

        writer.buffer_put(
            "a\x1Bx".to_string(),
            StoredValue {
                value: GunValue::Number(1.0),
                state: 100.0,
            },
        );
        writer.buffer_put(
            "b\x1By".to_string(),
            StoredValue {
                value: GunValue::Number(2.0),
                state: 100.0,
            },
        );

        assert_eq!(writer.pending_count(), 2);
        assert_eq!(writer.total_buffered(), 2);
    }

    #[test]
    fn buffer_deduplicates_same_key() {
        let (writer, _store) = make_writer(100);

        writer.buffer_put(
            "a\x1Bx".to_string(),
            StoredValue {
                value: GunValue::Number(1.0),
                state: 100.0,
            },
        );
        writer.buffer_put(
            "a\x1Bx".to_string(),
            StoredValue {
                value: GunValue::Number(2.0),
                state: 200.0,
            },
        );

        // Same key → only 1 pending entry (latest value wins)
        assert_eq!(writer.pending_count(), 1);
        assert_eq!(writer.total_buffered(), 2); // but 2 total buffered
    }

    #[test]
    fn buffer_signals_full() {
        let (writer, _store) = make_writer(3);

        let full1 = writer.buffer_put(
            "a".to_string(),
            StoredValue {
                value: GunValue::Null,
                state: 1.0,
            },
        );
        let full2 = writer.buffer_put(
            "b".to_string(),
            StoredValue {
                value: GunValue::Null,
                state: 1.0,
            },
        );
        let full3 = writer.buffer_put(
            "c".to_string(),
            StoredValue {
                value: GunValue::Null,
                state: 1.0,
            },
        );

        assert!(!full1);
        assert!(!full2);
        assert!(full3); // 3 >= max_buffer_size(3) → signals full
    }

    // Async tests using a simple block_on

    fn block_on<F: std::future::Future<Output = T>, T>(f: F) -> T {
        // Minimal single-threaded executor for tests
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

        fn dummy_raw_waker() -> RawWaker {
            fn no_op(_: *const ()) {}
            fn clone(p: *const ()) -> RawWaker {
                RawWaker::new(p, &VTABLE)
            }
            const VTABLE: RawWakerVTable =
                RawWakerVTable::new(clone, no_op, no_op, no_op);
            RawWaker::new(std::ptr::null(), &VTABLE)
        }

        let waker = unsafe { Waker::from_raw(dummy_raw_waker()) };
        let mut cx = Context::from_waker(&waker);
        let mut f = std::pin::pin!(f);

        loop {
            match f.as_mut().poll(&mut cx) {
                Poll::Ready(val) => return val,
                Poll::Pending => {
                    // Our AsyncMemoryStorage futures are always ready,
                    // so we should never get here.
                    panic!("unexpected Pending in test executor");
                }
            }
        }
    }

    #[test]
    fn flush_writes_to_storage() {
        let (writer, store) = make_writer(100);

        writer.buffer_put(
            "mark\x1Bname".to_string(),
            StoredValue {
                value: GunValue::Text("Mark".into()),
                state: 100.0,
            },
        );
        writer.buffer_put(
            "mark\x1Bage".to_string(),
            StoredValue {
                value: GunValue::Number(30.0),
                state: 100.0,
            },
        );

        let flushed = block_on(writer.flush()).unwrap();
        assert_eq!(flushed, 2);
        assert_eq!(writer.pending_count(), 0);
        assert_eq!(writer.total_flushes(), 1);

        // Verify data in storage
        let name = block_on(store.get("mark\x1Bname".to_string())).unwrap().unwrap();
        assert_eq!(name.value, GunValue::Text("Mark".into()));
    }

    #[test]
    fn flush_empty_is_noop() {
        let (writer, _store) = make_writer(100);
        let flushed = block_on(writer.flush()).unwrap();
        assert_eq!(flushed, 0);
    }

    #[test]
    fn flush_clears_buffer() {
        let (writer, _store) = make_writer(100);

        writer.buffer_put(
            "a".to_string(),
            StoredValue {
                value: GunValue::Null,
                state: 1.0,
            },
        );
        assert_eq!(writer.pending_count(), 1);

        block_on(writer.flush()).unwrap();
        assert_eq!(writer.pending_count(), 0);
        assert!(writer.is_empty());
    }

    #[test]
    fn multiple_flushes_increment_counter() {
        let (writer, _store) = make_writer(100);

        writer.buffer_put(
            "a".to_string(),
            StoredValue {
                value: GunValue::Null,
                state: 1.0,
            },
        );
        block_on(writer.flush()).unwrap();

        writer.buffer_put(
            "b".to_string(),
            StoredValue {
                value: GunValue::Null,
                state: 1.0,
            },
        );
        block_on(writer.flush()).unwrap();

        assert_eq!(writer.total_flushes(), 2);
        assert_eq!(writer.total_buffered(), 2);
    }
}
