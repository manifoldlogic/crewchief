//! Memory optimization module for Maproom.
//!
//! This module provides memory-efficient data structures and techniques to minimize
//! memory usage while maintaining performance:
//!
//! - **String Interning**: Deduplicates repeated strings (paths, symbols, languages)
//! - **Vector Quantization**: Compresses embeddings from f32 to i8 (4x reduction)
//! - **Buffer Pooling**: Reuses buffers for file reading and parsing
//! - **Memory Metrics**: Tracks allocations and memory usage
//!
//! # Performance Target
//!
//! Memory usage <500MB for 100k chunks with:
//! - String interning for paths and symbols
//! - Quantized embeddings (f32 → i8)
//! - Pooled buffers for I/O operations
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │          Memory Optimization                │
//! ├─────────────────────────────────────────────┤
//! │                                             │
//! │  String Interning    Vector Quantization    │
//! │  ┌──────────────┐   ┌──────────────┐       │
//! │  │ Interner     │   │ Quantizer    │       │
//! │  │ HashMap      │   │ f32 → i8     │       │
//! │  │ Arc<str>     │   │ 4x reduction │       │
//! │  └──────────────┘   └──────────────┘       │
//! │                                             │
//! │  Buffer Pooling      Memory Metrics         │
//! │  ┌──────────────┐   ┌──────────────┐       │
//! │  │ Pool         │   │ Allocations  │       │
//! │  │ Reusable     │   │ Usage        │       │
//! │  │ Vec<u8>      │   │ Peak         │       │
//! │  └──────────────┘   └──────────────┘       │
//! │                                             │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use maproom::memory::{
//!     StringInterner, quantize_embedding, BufferPool, get_memory_metrics
//! };
//! use std::sync::Arc;
//!
//! # fn main() {
//! // String interning
//! let interner = StringInterner::new();
//! let path1 = interner.intern("src/main.rs");
//! let path2 = interner.intern("src/main.rs"); // Same Arc
//! assert!(Arc::ptr_eq(&path1, &path2));
//!
//! // Vector quantization
//! let embedding = vec![0.5, -0.3, 0.8];
//! let quantized = quantize_embedding(&embedding);
//! assert_eq!(quantized.len(), embedding.len());
//!
//! // Buffer pooling
//! let pool = BufferPool::new(1024 * 64, 10);
//! let mut buffer = pool.acquire();
//! // Use buffer for reading...
//! drop(buffer); // Returns to pool
//!
//! // Memory metrics
//! let metrics = get_memory_metrics();
//! println!("Current usage: {} bytes", metrics.current_bytes());
//! # }
//! ```

pub mod interner;
pub mod metrics;
pub mod pool;
pub mod quantization;

pub use interner::{get_global_interner, StringInterner};
pub use metrics::{get_memory_metrics, MemoryMetrics};
pub use pool::{BufferPool, PooledBuffer};
pub use quantization::{dequantize_embedding, quantize_embedding};
