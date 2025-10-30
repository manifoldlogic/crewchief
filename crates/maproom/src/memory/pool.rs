//! Buffer pooling for reusable memory allocation.
//!
//! Buffer pooling reduces allocation overhead by reusing pre-allocated buffers.
//! This is especially effective for repetitive operations like file reading and parsing.
//!
//! # Benefits
//!
//! - **Reduced Allocations**: Reuse buffers instead of allocating new ones
//! - **Lower GC Pressure**: Fewer allocations mean less garbage collection
//! - **Predictable Memory**: Fixed pool size prevents unbounded growth
//! - **Better Performance**: Allocation is expensive, reuse is cheap
//!
//! # Use Cases
//!
//! - File reading buffers (64KB typical)
//! - Parsing intermediate buffers
//! - Network I/O buffers
//! - Temporary working memory
//!
//! # Architecture
//!
//! ```text
//! BufferPool
//!   ├─ Free List: Vec<Vec<u8>>
//!   ├─ Buffer Size: 64KB (configurable)
//!   └─ Max Pool Size: 100 (configurable)
//!
//! Flow:
//!   1. acquire() → takes buffer from free list (or creates new)
//!   2. use buffer for I/O
//!   3. drop(buffer) → returns to free list
//! ```
//!
//! # Performance
//!
//! - **acquire()**: O(1) - pop from free list
//! - **release()**: O(1) - push to free list
//! - **Memory**: Fixed = buffer_size × pool_size
//!
//! # Example
//!
//! ```no_run
//! use crewchief_maproom::memory::BufferPool;
//! use std::fs::File;
//! use std::io::Read;
//!
//! # fn main() -> std::io::Result<()> {
//! // Create pool with 64KB buffers, max 10 buffers
//! let pool = BufferPool::new(64 * 1024, 10);
//!
//! // Acquire buffer from pool
//! let mut buffer = pool.acquire();
//!
//! // Use buffer for file reading
//! let mut file = File::open("file.txt")?;
//! let n = file.read(&mut buffer)?;
//!
//! // Process data...
//! println!("Read {} bytes", n);
//!
//! // Buffer automatically returns to pool on drop
//! drop(buffer);
//!
//! // Stats show pool usage
//! let stats = pool.stats();
//! println!("Pool size: {}", stats.pool_size);
//! println!("In use: {}", stats.in_use);
//! # Ok(())
//! # }
//! ```

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

/// Buffer pool for reusable byte buffers.
///
/// BufferPool maintains a pool of pre-allocated buffers that can be
/// acquired, used, and returned to the pool. This reduces allocation
/// overhead for repetitive operations.
///
/// # Thread Safety
///
/// BufferPool is thread-safe and can be shared across threads using Arc.
/// Buffers are protected by a Mutex.
///
/// # Example
///
/// ```no_run
/// use crewchief_maproom::memory::BufferPool;
/// use std::sync::Arc;
/// use std::thread;
///
/// let pool = Arc::new(BufferPool::new(1024, 5));
///
/// let mut handles = vec![];
/// for _ in 0..3 {
///     let pool = pool.clone();
///     handles.push(thread::spawn(move || {
///         let mut buffer = pool.acquire();
///         // Use buffer...
///         buffer.clear();
///     }));
/// }
///
/// for handle in handles {
///     handle.join().unwrap();
/// }
/// ```
pub struct BufferPool {
    /// Pool of available buffers
    pool: Arc<Mutex<BufferPoolInner>>,

    /// Size of each buffer in bytes
    buffer_size: usize,

    /// Maximum number of buffers to keep in pool
    max_pool_size: usize,
}

struct BufferPoolInner {
    /// Free buffers ready for reuse
    free_buffers: Vec<Vec<u8>>,

    /// Statistics
    stats: PoolStats,
}

impl BufferPool {
    /// Create a new BufferPool.
    ///
    /// # Arguments
    ///
    /// * `buffer_size` - Size of each buffer in bytes
    /// * `max_pool_size` - Maximum number of buffers to keep in pool
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::memory::BufferPool;
    ///
    /// // 64KB buffers, max 10 in pool
    /// let pool = BufferPool::new(64 * 1024, 10);
    /// ```
    pub fn new(buffer_size: usize, max_pool_size: usize) -> Self {
        Self {
            pool: Arc::new(Mutex::new(BufferPoolInner {
                free_buffers: Vec::with_capacity(max_pool_size),
                stats: PoolStats::default(),
            })),
            buffer_size,
            max_pool_size,
        }
    }

    /// Acquire a buffer from the pool.
    ///
    /// Returns a PooledBuffer that automatically returns to the pool
    /// when dropped. If the pool is empty, allocates a new buffer.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::memory::BufferPool;
    ///
    /// let pool = BufferPool::new(1024, 5);
    /// let mut buffer = pool.acquire();
    ///
    /// // Use buffer...
    /// buffer.extend_from_slice(b"hello");
    ///
    /// // Automatically returns to pool on drop
    /// ```
    pub fn acquire(&self) -> PooledBuffer {
        let mut inner = self.pool.lock().unwrap();

        let buffer = if let Some(mut buf) = inner.free_buffers.pop() {
            // Reuse buffer from pool
            buf.clear();
            inner.stats.reuses += 1;
            buf
        } else {
            // Allocate new buffer
            inner.stats.allocations += 1;
            Vec::with_capacity(self.buffer_size)
        };

        inner.stats.in_use += 1;
        inner.stats.peak_in_use = inner.stats.peak_in_use.max(inner.stats.in_use);

        PooledBuffer {
            buffer: Some(buffer),
            pool: self.pool.clone(),
            max_pool_size: self.max_pool_size,
        }
    }

    /// Get current pool statistics.
    pub fn stats(&self) -> PoolStats {
        let inner = self.pool.lock().unwrap();
        PoolStats {
            pool_size: inner.free_buffers.len(),
            in_use: inner.stats.in_use,
            peak_in_use: inner.stats.peak_in_use,
            allocations: inner.stats.allocations,
            reuses: inner.stats.reuses,
        }
    }

    /// Get the configured buffer size.
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// Get the maximum pool size.
    pub fn max_pool_size(&self) -> usize {
        self.max_pool_size
    }

    /// Clear all buffers from the pool.
    ///
    /// This frees memory but does not affect buffers currently in use.
    pub fn clear(&self) {
        let mut inner = self.pool.lock().unwrap();
        inner.free_buffers.clear();
    }

    /// Estimate total memory used by pooled buffers.
    ///
    /// This includes buffers in the pool and buffers currently in use.
    pub fn estimated_memory_bytes(&self) -> usize {
        let stats = self.stats();
        (stats.pool_size + stats.in_use) * self.buffer_size
    }
}

impl Default for BufferPool {
    fn default() -> Self {
        // Default: 64KB buffers, max 10 in pool
        Self::new(64 * 1024, 10)
    }
}

/// A buffer borrowed from a BufferPool.
///
/// When dropped, the buffer is returned to the pool for reuse.
/// Derefs to Vec<u8> for easy use.
pub struct PooledBuffer {
    buffer: Option<Vec<u8>>,
    pool: Arc<Mutex<BufferPoolInner>>,
    max_pool_size: usize,
}

impl PooledBuffer {
    /// Get the capacity of the buffer.
    pub fn capacity(&self) -> usize {
        self.buffer.as_ref().map(|b| b.capacity()).unwrap_or(0)
    }

    /// Consume the PooledBuffer and return the inner Vec<u8>.
    ///
    /// This prevents the buffer from being returned to the pool.
    /// Use this when you need to keep ownership of the buffer.
    pub fn into_inner(mut self) -> Vec<u8> {
        let buffer = self.buffer.take().unwrap();

        // Decrement in_use counter since we're taking ownership
        // and the buffer won't be returned to the pool
        let mut inner = self.pool.lock().unwrap();
        inner.stats.in_use -= 1;

        buffer
    }
}

impl Deref for PooledBuffer {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        self.buffer.as_ref().unwrap()
    }
}

impl DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer.as_mut().unwrap()
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            let mut inner = self.pool.lock().unwrap();

            // Return to pool if not at max size
            if inner.free_buffers.len() < self.max_pool_size {
                inner.free_buffers.push(buffer);
            }

            inner.stats.in_use -= 1;
        }
    }
}

/// Statistics for buffer pool usage.
#[derive(Debug, Clone, Copy, Default)]
pub struct PoolStats {
    /// Number of buffers currently in the pool
    pub pool_size: usize,

    /// Number of buffers currently in use
    pub in_use: usize,

    /// Peak number of buffers in use
    pub peak_in_use: usize,

    /// Total number of buffers allocated
    pub allocations: usize,

    /// Total number of buffers reused from pool
    pub reuses: usize,
}

impl PoolStats {
    /// Calculate the reuse rate (0.0-1.0).
    ///
    /// Higher is better - means more buffers were reused vs allocated.
    pub fn reuse_rate(&self) -> f64 {
        let total = self.allocations + self.reuses;
        if total == 0 {
            0.0
        } else {
            self.reuses as f64 / total as f64
        }
    }

    /// Calculate total buffers acquired (allocations + reuses).
    pub fn total_acquires(&self) -> usize {
        self.allocations + self.reuses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let pool = BufferPool::new(1024, 5);
        assert_eq!(pool.buffer_size(), 1024);
        assert_eq!(pool.max_pool_size(), 5);

        let stats = pool.stats();
        assert_eq!(stats.pool_size, 0);
        assert_eq!(stats.in_use, 0);
    }

    #[test]
    fn test_acquire_and_release() {
        let pool = BufferPool::new(1024, 5);

        {
            let buffer = pool.acquire();
            assert_eq!(buffer.capacity(), 1024);

            let stats = pool.stats();
            assert_eq!(stats.in_use, 1);
            assert_eq!(stats.allocations, 1);
        }

        // Buffer returned to pool
        let stats = pool.stats();
        assert_eq!(stats.in_use, 0);
        assert_eq!(stats.pool_size, 1);
    }

    #[test]
    fn test_buffer_reuse() {
        let pool = BufferPool::new(1024, 5);

        // First acquire allocates
        {
            let _buffer = pool.acquire();
        }

        // Second acquire reuses
        {
            let _buffer = pool.acquire();
        }

        let stats = pool.stats();
        assert_eq!(stats.allocations, 1);
        assert_eq!(stats.reuses, 1);
        assert_eq!(stats.reuse_rate(), 0.5);
    }

    #[test]
    fn test_max_pool_size() {
        let pool = BufferPool::new(1024, 2);

        // Acquire 5 buffers at once
        let buffers: Vec<_> = (0..5).map(|_| pool.acquire()).collect();

        // Drop all buffers
        drop(buffers);

        // Pool should only keep 2 buffers (max_pool_size)
        let stats = pool.stats();
        assert_eq!(stats.pool_size, 2);
    }

    #[test]
    fn test_concurrent_buffers() {
        let pool = BufferPool::new(1024, 5);

        let b1 = pool.acquire();
        let b2 = pool.acquire();
        let b3 = pool.acquire();

        let stats = pool.stats();
        assert_eq!(stats.in_use, 3);
        assert_eq!(stats.peak_in_use, 3);

        drop(b1);
        drop(b2);
        drop(b3);

        let stats = pool.stats();
        assert_eq!(stats.in_use, 0);
        assert_eq!(stats.pool_size, 3);
        assert_eq!(stats.peak_in_use, 3);
    }

    #[test]
    fn test_buffer_usage() {
        let pool = BufferPool::new(1024, 5);
        let mut buffer = pool.acquire();

        // Can use as Vec<u8>
        buffer.extend_from_slice(b"hello");
        assert_eq!(buffer.len(), 5);
        assert_eq!(&buffer[..], b"hello");

        buffer.clear();
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_into_inner() {
        let pool = BufferPool::new(1024, 5);
        let mut buffer = pool.acquire();

        buffer.extend_from_slice(b"data");

        // Before consuming, buffer is in use
        let stats = pool.stats();
        assert_eq!(stats.in_use, 1);

        let inner = buffer.into_inner();
        assert_eq!(&inner[..], b"data");

        // After into_inner(), buffer is not returned to pool and not in use
        let stats = pool.stats();
        assert_eq!(stats.pool_size, 0);
        assert_eq!(stats.in_use, 0);
    }

    #[test]
    fn test_clear_pool() {
        let pool = BufferPool::new(1024, 5);

        // Acquire 3 buffers at once, then drop them
        {
            let _buffers: Vec<_> = (0..3).map(|_| pool.acquire()).collect();
        }

        let stats = pool.stats();
        assert_eq!(stats.pool_size, 3);

        pool.clear();

        let stats = pool.stats();
        assert_eq!(stats.pool_size, 0);
    }

    #[test]
    fn test_estimated_memory() {
        let pool = BufferPool::new(1024, 5);

        let _b1 = pool.acquire();
        let _b2 = pool.acquire();

        // 2 in use
        let memory = pool.estimated_memory_bytes();
        assert_eq!(memory, 2 * 1024);
    }

    #[test]
    fn test_default_pool() {
        let pool = BufferPool::default();
        assert_eq!(pool.buffer_size(), 64 * 1024);
        assert_eq!(pool.max_pool_size(), 10);
    }

    #[test]
    fn test_peak_tracking() {
        let pool = BufferPool::new(1024, 5);

        let b1 = pool.acquire();
        let b2 = pool.acquire();
        let b3 = pool.acquire();

        let stats = pool.stats();
        assert_eq!(stats.peak_in_use, 3);

        drop(b1);
        drop(b2);

        let stats = pool.stats();
        assert_eq!(stats.in_use, 1);
        assert_eq!(stats.peak_in_use, 3); // Peak remains

        drop(b3);

        let stats = pool.stats();
        assert_eq!(stats.in_use, 0);
        assert_eq!(stats.peak_in_use, 3); // Peak still remains
    }

    #[test]
    fn test_buffer_cleared_on_acquire() {
        let pool = BufferPool::new(1024, 5);

        {
            let mut buffer = pool.acquire();
            buffer.extend_from_slice(b"old data");
        }

        {
            let buffer = pool.acquire();
            // Buffer should be cleared
            assert_eq!(buffer.len(), 0);
        }
    }

    #[test]
    fn test_concurrent_threads() {
        use std::thread;

        let pool = Arc::new(BufferPool::new(1024, 10));
        let mut handles = vec![];

        for _ in 0..5 {
            let pool = pool.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..10 {
                    let mut buffer = pool.acquire();
                    buffer.extend_from_slice(b"test");
                    assert!(buffer.len() > 0);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let stats = pool.stats();
        assert_eq!(stats.in_use, 0);
        assert!(stats.allocations > 0);
        assert!(stats.reuses > 0);
    }

    #[test]
    fn test_reuse_rate() {
        let pool = BufferPool::new(1024, 5);

        // First batch: acquire 5 buffers, then drop them all
        {
            let _buffers: Vec<_> = (0..5).map(|_| pool.acquire()).collect();
        }

        // Pool should have 5 buffers now
        let stats = pool.stats();
        assert_eq!(stats.allocations, 5);
        assert_eq!(stats.pool_size, 5);

        // Second batch: acquire 5 more (should reuse)
        {
            let _buffers: Vec<_> = (0..5).map(|_| pool.acquire()).collect();
        }

        let stats = pool.stats();
        assert_eq!(stats.allocations, 5);
        assert_eq!(stats.reuses, 5);
        assert_eq!(stats.reuse_rate(), 0.5);
        assert_eq!(stats.total_acquires(), 10);
    }

    #[test]
    fn test_zero_max_pool_size() {
        let pool = BufferPool::new(1024, 0);

        {
            let _buffer = pool.acquire();
        }

        // No buffers should be retained
        let stats = pool.stats();
        assert_eq!(stats.pool_size, 0);
        assert_eq!(stats.allocations, 1);
    }
}
