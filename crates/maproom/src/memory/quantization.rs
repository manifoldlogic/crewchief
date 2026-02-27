//! Vector quantization for embedding compression.
//!
//! Vector quantization reduces memory usage by converting high-precision
//! embeddings (f32) to low-precision integers (i8), achieving 4x memory reduction.
//!
//! # Memory Savings
//!
//! - **f32 embeddings**: 1536 dims × 4 bytes = 6,144 bytes per vector
//! - **i8 embeddings**: 1536 dims × 1 byte = 1,536 bytes per vector
//! - **Reduction**: 4x smaller (75% memory savings)
//!
//! # Quantization Method
//!
//! Uses symmetric quantization with scaling factor 127.0:
//! - `quantize: i8 = round(f32 * 127.0)`
//! - `dequantize: f32 = i8 / 127.0`
//!
//! This preserves the range [-1.0, 1.0] with 1/127 precision.
//!
//! # Accuracy Trade-offs
//!
//! - **Quantization error**: ±0.004 (1/127/2)
//! - **Search quality**: Typically <1% degradation in recall@10
//! - **Performance**: Faster distance computation (integer ops vs float)
//!
//! # When to Use
//!
//! - **Storage**: Always quantize for database storage
//! - **Cache**: Use quantized vectors in embedding cache
//! - **Search**: Can use quantized vectors for initial filtering
//! - **Accuracy-critical**: Dequantize for final ranking
//!
//! # Example
//!
//! ```no_run
//! use maproom::memory::{quantize_embedding, dequantize_embedding};
//!
//! // Original embedding (f32)
//! let original = vec![0.5, -0.3, 0.8, 0.0];
//!
//! // Quantize to i8 (4x smaller)
//! let quantized = quantize_embedding(&original);
//! assert_eq!(quantized.len(), original.len());
//!
//! // Dequantize back to f32
//! let restored = dequantize_embedding(&quantized);
//!
//! // Small quantization error
//! for (orig, rest) in original.iter().zip(restored.iter()) {
//!     assert!((orig - rest).abs() < 0.01);
//! }
//! ```

/// Scaling factor for quantization (max i8 value).
const QUANTIZATION_SCALE: f32 = 127.0;

/// Inverse scaling factor for dequantization.
const DEQUANTIZATION_SCALE: f32 = 1.0 / 127.0;

/// Quantize an f32 embedding to i8.
///
/// Converts each dimension from f32 to i8 using symmetric quantization:
/// - Input range: [-1.0, 1.0] (typical for normalized embeddings)
/// - Output range: [-127, 127]
/// - Formula: `i8 = round(f32 * 127.0)`
///
/// Values outside [-1.0, 1.0] are clamped to [-127, 127].
///
/// # Memory Savings
///
/// - Reduces memory by 4x (f32 → i8)
/// - For 1536-dim vectors: 6144 bytes → 1536 bytes
///
/// # Example
///
/// ```no_run
/// use maproom::memory::quantize_embedding;
///
/// let embedding = vec![0.5, -0.3, 0.8];
/// let quantized = quantize_embedding(&embedding);
///
/// assert_eq!(quantized.len(), 3);
/// assert_eq!(quantized[0], 64);  // 0.5 * 127 ≈ 64
/// assert_eq!(quantized[1], -38); // -0.3 * 127 ≈ -38
/// assert_eq!(quantized[2], 102); // 0.8 * 127 ≈ 102
/// ```
pub fn quantize_embedding(embedding: &[f32]) -> Vec<i8> {
    embedding
        .iter()
        .map(|&value| {
            // Scale and round
            let scaled = value * QUANTIZATION_SCALE;
            let rounded = scaled.round();

            // Clamp to i8 range
            if rounded > 127.0 {
                127
            } else if rounded < -127.0 {
                -127
            } else {
                rounded as i8
            }
        })
        .collect()
}

/// Dequantize an i8 embedding back to f32.
///
/// Converts each dimension from i8 to f32 using inverse scaling:
/// - Input range: [-127, 127]
/// - Output range: [-1.0, 1.0]
/// - Formula: `f32 = i8 / 127.0`
///
/// # Precision
///
/// - Quantization precision: 1/127 ≈ 0.0079
/// - Maximum error: ±0.004 (half precision)
///
/// # Example
///
/// ```no_run
/// use maproom::memory::{quantize_embedding, dequantize_embedding};
///
/// let original = vec![0.5, -0.3, 0.8];
/// let quantized = quantize_embedding(&original);
/// let restored = dequantize_embedding(&quantized);
///
/// // Small quantization error
/// assert!((original[0] - restored[0]).abs() < 0.01);
/// assert!((original[1] - restored[1]).abs() < 0.01);
/// assert!((original[2] - restored[2]).abs() < 0.01);
/// ```
pub fn dequantize_embedding(quantized: &[i8]) -> Vec<f32> {
    quantized
        .iter()
        .map(|&value| value as f32 * DEQUANTIZATION_SCALE)
        .collect()
}

/// Calculate cosine similarity between quantized embeddings.
///
/// This computes cosine similarity directly on i8 vectors, avoiding
/// dequantization. The result is the same as computing on f32 vectors
/// (up to quantization error).
///
/// # Formula
///
/// ```text
/// cosine_similarity = dot_product / (norm_a * norm_b)
/// ```
///
/// # Performance
///
/// - Faster than f32 similarity (integer ops)
/// - Same complexity: O(n) where n is embedding dimension
/// - No dequantization overhead
///
/// # Example
///
/// ```ignore
/// use maproom::memory::{quantize_embedding, cosine_similarity_quantized};
///
/// let a = vec![0.5, -0.3, 0.8];
/// let b = vec![0.4, -0.2, 0.9];
///
/// let qa = quantize_embedding(&a);
/// let qb = quantize_embedding(&b);
///
/// let similarity = cosine_similarity_quantized(&qa, &qb);
/// assert!(similarity >= -1.0 && similarity <= 1.0);
/// ```
pub fn cosine_similarity_quantized(a: &[i8], b: &[i8]) -> f32 {
    assert_eq!(a.len(), b.len(), "Embeddings must have the same dimension");

    if a.is_empty() {
        return 0.0;
    }

    // Compute dot product
    let dot_product: i32 = a
        .iter()
        .zip(b.iter())
        .map(|(&x, &y)| x as i32 * y as i32)
        .sum();

    // Compute norms
    let norm_a_sq: i32 = a.iter().map(|&x| x as i32 * x as i32).sum();
    let norm_b_sq: i32 = b.iter().map(|&x| x as i32 * x as i32).sum();

    // Avoid division by zero
    if norm_a_sq == 0 || norm_b_sq == 0 {
        return 0.0;
    }

    // Calculate cosine similarity
    let norm_a = (norm_a_sq as f32).sqrt();
    let norm_b = (norm_b_sq as f32).sqrt();

    dot_product as f32 / (norm_a * norm_b)
}

/// Estimate memory savings from quantization.
///
/// Returns the percentage of memory saved by using i8 instead of f32.
/// For typical embeddings, this is 75% (4x reduction).
///
/// # Example
///
/// ```ignore
/// use maproom::memory::quantization_memory_savings;
///
/// let embedding_dim = 1536;
/// let num_vectors = 100_000;
///
/// let f32_bytes = embedding_dim * num_vectors * 4;
/// let i8_bytes = embedding_dim * num_vectors * 1;
/// let savings = quantization_memory_savings(f32_bytes, i8_bytes);
///
/// assert_eq!(savings, 75.0); // 75% savings
/// ```
pub fn quantization_memory_savings(f32_bytes: usize, i8_bytes: usize) -> f64 {
    if f32_bytes == 0 {
        return 0.0;
    }
    ((f32_bytes - i8_bytes) as f64 / f32_bytes as f64) * 100.0
}

/// Calculate quantization error statistics.
///
/// Computes error metrics between original and quantized embeddings:
/// - **max_error**: Maximum absolute error across all dimensions
/// - **mean_error**: Average absolute error
/// - **rmse**: Root mean squared error
///
/// # Example
///
/// ```ignore
/// use maproom::memory::{quantize_embedding, dequantize_embedding, quantization_error};
///
/// let original = vec![0.5, -0.3, 0.8, 0.1];
/// let quantized = quantize_embedding(&original);
/// let restored = dequantize_embedding(&quantized);
///
/// let error = quantization_error(&original, &restored);
/// println!("Max error: {:.4}", error.max_error);
/// println!("Mean error: {:.4}", error.mean_error);
/// println!("RMSE: {:.4}", error.rmse);
/// ```
pub fn quantization_error(original: &[f32], restored: &[f32]) -> QuantizationError {
    assert_eq!(
        original.len(),
        restored.len(),
        "Vectors must have same length"
    );

    let mut max_error = 0.0f32;
    let mut sum_error = 0.0f32;
    let mut sum_squared_error = 0.0f32;

    for (&orig, &rest) in original.iter().zip(restored.iter()) {
        let error = (orig - rest).abs();
        max_error = max_error.max(error);
        sum_error += error;
        sum_squared_error += error * error;
    }

    let n = original.len() as f32;

    QuantizationError {
        max_error,
        mean_error: sum_error / n,
        rmse: (sum_squared_error / n).sqrt(),
    }
}

/// Quantization error statistics.
#[derive(Debug, Clone, Copy)]
pub struct QuantizationError {
    /// Maximum absolute error across all dimensions
    pub max_error: f32,

    /// Mean absolute error
    pub mean_error: f32,

    /// Root mean squared error
    pub rmse: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantize_basic() {
        let embedding = vec![0.5, -0.5, 1.0, -1.0, 0.0];
        let quantized = quantize_embedding(&embedding);

        assert_eq!(quantized.len(), 5);
        assert_eq!(quantized[0], 64); // 0.5 * 127 ≈ 64
        assert_eq!(quantized[1], -64); // -0.5 * 127 ≈ -64
        assert_eq!(quantized[2], 127); // 1.0 * 127 = 127
        assert_eq!(quantized[3], -127); // -1.0 * 127 = -127
        assert_eq!(quantized[4], 0); // 0.0 * 127 = 0
    }

    #[test]
    fn test_quantize_clamps_out_of_range() {
        let embedding = vec![2.0, -2.0, 1.5, -1.5];
        let quantized = quantize_embedding(&embedding);

        // Should clamp to [-127, 127]
        assert_eq!(quantized[0], 127);
        assert_eq!(quantized[1], -127);
        assert_eq!(quantized[2], 127);
        assert_eq!(quantized[3], -127);
    }

    #[test]
    fn test_dequantize() {
        let quantized = vec![64, -64, 127, -127, 0];
        let dequantized = dequantize_embedding(&quantized);

        assert_eq!(dequantized.len(), 5);
        assert!((dequantized[0] - 0.504).abs() < 0.01);
        assert!((dequantized[1] - (-0.504)).abs() < 0.01);
        assert!((dequantized[2] - 1.0).abs() < 0.01);
        assert!((dequantized[3] - (-1.0)).abs() < 0.01);
        assert!((dequantized[4] - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_roundtrip_error() {
        let original = vec![0.1, 0.2, 0.3, 0.4, 0.5, -0.1, -0.2, -0.3];
        let quantized = quantize_embedding(&original);
        let restored = dequantize_embedding(&quantized);

        // Check roundtrip error is small
        for (orig, rest) in original.iter().zip(restored.iter()) {
            let error = (orig - rest).abs();
            assert!(error < 0.01, "Error too large: {}", error);
        }
    }

    #[test]
    fn test_cosine_similarity_quantized() {
        let a = vec![0.5, -0.3, 0.8];
        let b = vec![0.5, -0.3, 0.8]; // Same vector

        let qa = quantize_embedding(&a);
        let qb = quantize_embedding(&b);

        let similarity = cosine_similarity_quantized(&qa, &qb);

        // Same vector should have similarity ≈ 1.0
        assert!((similarity - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0]; // Orthogonal

        let qa = quantize_embedding(&a);
        let qb = quantize_embedding(&b);

        let similarity = cosine_similarity_quantized(&qa, &qb);

        // Orthogonal vectors should have similarity ≈ 0.0
        assert!(similarity.abs() < 0.01);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0]; // Opposite

        let qa = quantize_embedding(&a);
        let qb = quantize_embedding(&b);

        let similarity = cosine_similarity_quantized(&qa, &qb);

        // Opposite vectors should have similarity ≈ -1.0
        assert!((similarity - (-1.0)).abs() < 0.01);
    }

    #[test]
    #[should_panic(expected = "Embeddings must have the same dimension")]
    fn test_cosine_similarity_different_dimensions() {
        let a = vec![1, 2, 3];
        let b = vec![1, 2];

        cosine_similarity_quantized(&a, &b);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<i8> = vec![];
        let b: Vec<i8> = vec![];

        let similarity = cosine_similarity_quantized(&a, &b);
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0, 0, 0];
        let b = vec![1, 2, 3];

        let similarity = cosine_similarity_quantized(&a, &b);
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_memory_savings() {
        let f32_bytes = 1536 * 100_000 * 4; // 100k vectors, 1536 dims, 4 bytes
        let i8_bytes = 1536 * 100_000 * 1; // 100k vectors, 1536 dims, 1 byte

        let savings = quantization_memory_savings(f32_bytes, i8_bytes);
        assert_eq!(savings, 75.0);
    }

    #[test]
    fn test_quantization_error_stats() {
        let original = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let quantized = quantize_embedding(&original);
        let restored = dequantize_embedding(&quantized);

        let error = quantization_error(&original, &restored);

        assert!(error.max_error < 0.01);
        assert!(error.mean_error < 0.01);
        assert!(error.rmse < 0.01);
    }

    #[test]
    fn test_quantization_error_perfect() {
        // Values that quantize perfectly
        let original = vec![0.0, 1.0 / 127.0, -1.0 / 127.0];
        let quantized = quantize_embedding(&original);
        let restored = dequantize_embedding(&quantized);

        let error = quantization_error(&original, &restored);

        // Should be very close to zero
        assert!(error.max_error < 0.0001);
        assert!(error.mean_error < 0.0001);
        assert!(error.rmse < 0.0001);
    }

    #[test]
    fn test_quantize_normalized_embedding() {
        // Typical normalized embedding from OpenAI
        let embedding = vec![
            0.12, -0.34, 0.56, 0.78, -0.23, 0.45, -0.67, 0.89, -0.12, 0.34,
        ];

        // Normalize to unit length
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized: Vec<f32> = embedding.iter().map(|x| x / norm).collect();

        let quantized = quantize_embedding(&normalized);
        let restored = dequantize_embedding(&quantized);

        // Check error is acceptable
        let error = quantization_error(&normalized, &restored);
        assert!(error.max_error < 0.02);
        assert!(error.mean_error < 0.01);
    }

    #[test]
    fn test_large_embedding() {
        // Test with realistic embedding size (1536 dimensions)
        let embedding: Vec<f32> = (0..1536).map(|i| (i as f32 / 1536.0) * 2.0 - 1.0).collect();

        let quantized = quantize_embedding(&embedding);
        assert_eq!(quantized.len(), 1536);

        let restored = dequantize_embedding(&quantized);
        assert_eq!(restored.len(), 1536);

        // Check memory savings
        let f32_bytes = 1536 * 4;
        let i8_bytes = 1536 * 1;
        let savings = quantization_memory_savings(f32_bytes, i8_bytes);
        assert_eq!(savings, 75.0);
    }
}
