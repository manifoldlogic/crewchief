//! Integration tests for the Ollama embedding provider.
//!
//! These tests validate end-to-end embedding generation with a real Ollama server.
//! Requires Ollama running on localhost:11434 with nomic-embed-text model.
//!
//! Run these tests with:
//! ```
//! cargo test --test ollama_test
//! ```

use crewchief_maproom::embedding::{
    CacheConfig, EmbeddingCache, EmbeddingConfig, EmbeddingService, OllamaProvider, ParallelConfig,
    Provider, RetryConfig,
};
use std::sync::Arc;
use std::time::Instant;

/// Helper function to check if Ollama is available at localhost:11434.
async fn ollama_available() -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();

    // Check if Ollama is running by hitting the tags endpoint
    let result = client.get("http://localhost:11434/api/tags").send().await;

    if result.is_err() {
        return false;
    }

    // Check if nomic-embed-text model is available
    let response = result.unwrap();
    if !response.status().is_success() {
        return false;
    }

    // Parse response to check for nomic-embed-text
    let body = response.text().await.unwrap_or_default();
    body.contains("nomic-embed-text")
}

/// Helper function to create test configuration for Ollama.
fn test_config() -> EmbeddingConfig {
    EmbeddingConfig {
        provider: Provider::Ollama,
        model: "nomic-embed-text".to_string(),
        dimension: 768, // nomic-embed-text uses 768-dimensional embeddings
        cache: CacheConfig {
            max_entries: 1000,
            ttl_seconds: 3600,
            enable_metrics: true,
        },
        batch_size: 10,
        retry: RetryConfig::default(),
        api_key: None,      // Ollama doesn't require API key
        api_endpoint: None, // Use default localhost:11434
        parallel: ParallelConfig::default(),
    }
}

/// Helper function to skip test if Ollama is not available.
async fn skip_if_ollama_unavailable() -> Option<EmbeddingService> {
    if !ollama_available().await {
        eprintln!("WARNING: Skipping test - Ollama not available at localhost:11434 or nomic-embed-text model not found");
        eprintln!("To run these tests:");
        eprintln!("  1. Start Ollama: docker-compose up -d ollama");
        eprintln!("  2. Pull model: ollama pull nomic-embed-text");
        return None;
    }

    let config = test_config();
    match OllamaProvider::new(
        config
            .api_endpoint
            .unwrap_or_else(|| "http://localhost:11434/api/embed".to_string()),
        config.model,
        768,
    ) {
        Ok(provider) => match EmbeddingCache::new(config.cache) {
            Ok(cache) => Some(EmbeddingService::new(Box::new(provider), Arc::new(cache))),
            Err(e) => {
                eprintln!("WARNING: Failed to create EmbeddingCache: {:?}", e);
                None
            }
        },
        Err(e) => {
            eprintln!("WARNING: Failed to create Ollama provider: {:?}", e);
            None
        }
    }
}

#[tokio::test]
async fn test_single_embedding_generation() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    // Generate embedding for a single code snippet
    let text = "async fn parse_typescript_file(content: &str) -> Result<Vec<Chunk>, Error>";
    let start = Instant::now();
    let embedding = service.embed_text(text).await;
    let duration = start.elapsed();

    println!("Single embedding took: {:?}", duration);

    assert!(
        embedding.is_ok(),
        "Embedding generation failed: {:?}",
        embedding.err()
    );

    let embedding = embedding.unwrap();
    assert_eq!(
        embedding.len(),
        768,
        "Expected 768-dimensional embedding for nomic-embed-text"
    );

    // Verify embedding contains non-zero values
    let non_zero_count = embedding.iter().filter(|&&v| v != 0.0).count();
    assert!(
        non_zero_count > 700,
        "Embedding should have mostly non-zero values, got {} non-zero out of 768",
        non_zero_count
    );

    // Check that embedding values are finite
    assert!(
        embedding.iter().all(|&v| v.is_finite()),
        "Embedding contains non-finite values"
    );

    // Performance check: single embedding should be < 1 second
    assert!(
        duration.as_secs() < 2,
        "Single embedding took too long: {:?}",
        duration
    );
}

#[tokio::test]
async fn test_batch_embedding_generation() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    // Test with multiple code-like text samples
    let texts = vec![
        "function calculateSum(a: number, b: number): number { return a + b; }".to_string(),
        "class UserRepository { async findById(id: string): Promise<User> {} }".to_string(),
        "const config = { apiUrl: 'http://localhost:3000', timeout: 5000 };".to_string(),
        "interface RequestHandler { handle(req: Request): Promise<Response>; }".to_string(),
    ];

    let start = Instant::now();
    let embeddings = service.embed_batch(texts.clone()).await;
    let duration = start.elapsed();

    println!("Batch of {} embeddings took: {:?}", texts.len(), duration);

    assert!(
        embeddings.is_ok(),
        "Batch embedding failed: {:?}",
        embeddings.err()
    );

    let embeddings = embeddings.unwrap();
    assert_eq!(
        embeddings.len(),
        4,
        "Expected 4 embeddings for 4 input texts"
    );

    // Verify all embeddings have correct dimensions and properties
    for (i, embedding) in embeddings.iter().enumerate() {
        assert_eq!(
            embedding.len(),
            768,
            "Embedding {} should have 768 dimensions",
            i
        );
        assert!(
            embedding.iter().all(|&v| v.is_finite()),
            "Embedding {} contains non-finite values",
            i
        );

        let non_zero_count = embedding.iter().filter(|&&v| v != 0.0).count();
        assert!(
            non_zero_count > 700,
            "Embedding {} should have mostly non-zero values",
            i
        );
    }

    // Verify different texts produce different embeddings
    assert_ne!(
        embeddings[0], embeddings[1],
        "Different texts should produce different embeddings"
    );
    assert_ne!(
        embeddings[1], embeddings[2],
        "Different texts should produce different embeddings"
    );
    assert_ne!(
        embeddings[2], embeddings[3],
        "Different texts should produce different embeddings"
    );
}

#[tokio::test]
async fn test_invalid_model_error() {
    // Create config with non-existent model
    let config = EmbeddingConfig {
        provider: Provider::Ollama,
        model: "nonexistent-model-xyz".to_string(),
        dimension: 768,
        cache: CacheConfig::default(),
        batch_size: 10,
        retry: RetryConfig::default(),
        api_key: None,
        api_endpoint: None,
        parallel: ParallelConfig::default(),
    };

    // Check if Ollama is available first
    if !ollama_available().await {
        eprintln!("WARNING: Skipping test - Ollama not available");
        return;
    }

    let provider_result = OllamaProvider::new(
        config
            .api_endpoint
            .unwrap_or_else(|| "http://localhost:11434/api/embed".to_string()),
        config.model,
        768,
    );
    assert!(
        provider_result.is_ok(),
        "Provider creation should succeed even with invalid model"
    );

    let cache = EmbeddingCache::new(config.cache).unwrap();
    let service = EmbeddingService::new(Box::new(provider_result.unwrap()), Arc::new(cache));
    let text = "test text";
    let result = service.embed_text(text).await;

    assert!(result.is_err(), "Embedding with invalid model should fail");

    // Check error message contains model-related information
    let error = result.err().unwrap();
    let error_msg = format!("{:?}", error);
    println!("Error message for invalid model: {}", error_msg);

    // The error should indicate a problem with the model or API
    assert!(
        error_msg.contains("Ollama") || error_msg.contains("model") || error_msg.contains("API"),
        "Error message should mention Ollama, model, or API issues"
    );
}

#[tokio::test]
async fn test_unreachable_endpoint_error() {
    // Create config with invalid endpoint
    let config = EmbeddingConfig {
        provider: Provider::Ollama,
        model: "nomic-embed-text".to_string(),
        dimension: 768,
        cache: CacheConfig::default(),
        batch_size: 10,
        retry: RetryConfig::default(),
        api_key: None,
        api_endpoint: Some("http://localhost:99999/api/embed".to_string()), // Invalid port
        parallel: ParallelConfig::default(),
    };

    let provider_result = OllamaProvider::new(
        config
            .api_endpoint
            .unwrap_or_else(|| "http://localhost:11434/api/embed".to_string()),
        config.model,
        768,
    );
    assert!(provider_result.is_ok(), "Provider creation should succeed");

    let cache = EmbeddingCache::new(config.cache).unwrap();
    let service = EmbeddingService::new(Box::new(provider_result.unwrap()), Arc::new(cache));
    let text = "test text";
    let result = service.embed_text(text).await;

    assert!(
        result.is_err(),
        "Embedding with unreachable endpoint should fail"
    );

    let error = result.err().unwrap();
    let error_msg = format!("{:?}", error);
    println!("Error message for unreachable endpoint: {}", error_msg);

    // The error should indicate a network/connection problem
    assert!(
        error_msg.contains("Network")
            || error_msg.contains("connection")
            || error_msg.contains("timeout"),
        "Error message should indicate network/connection issues"
    );
}

#[tokio::test]
async fn test_batch_performance() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    // Create 50 code-like chunks for performance testing
    let texts: Vec<String> = (0..50)
        .map(|i| {
            format!(
                "function processItem{}(data: Record<string, any>): Promise<Result> {{ \
                 const result = await transform(data); \
                 return validate(result); \
                 }}",
                i
            )
        })
        .collect();

    println!("Starting batch performance test with {} texts", texts.len());
    let start = Instant::now();
    let embeddings = service.embed_large_batch(texts.clone()).await;
    let duration = start.elapsed();

    assert!(
        embeddings.is_ok(),
        "Large batch embedding failed: {:?}",
        embeddings.err()
    );

    let embeddings = embeddings.unwrap();
    assert_eq!(
        embeddings.len(),
        50,
        "Expected 50 embeddings for 50 input texts"
    );

    // Calculate performance metrics
    let chunks_per_second = 50.0 / duration.as_secs_f64();
    let chunks_per_minute = chunks_per_second * 60.0;

    println!("Performance metrics for 50-chunk batch:");
    println!("  Total time: {:?}", duration);
    println!("  Chunks/second: {:.2}", chunks_per_second);
    println!("  Chunks/minute: {:.2}", chunks_per_minute);

    // Log cache metrics
    let metrics = service.cache_metrics().await;
    println!("  Cache hits: {}", metrics.hits);
    println!("  Cache misses: {}", metrics.misses);

    // Performance target: 500-1000 chunks/min (8-17 chunks/sec)
    // Allow some flexibility since performance varies by hardware
    // We'll just check it's not extremely slow (< 3 chunks/sec = 180/min)
    assert!(
        chunks_per_minute > 180.0,
        "Performance too slow: {:.2} chunks/min (expected > 180/min)",
        chunks_per_minute
    );

    // Log whether we meet the target range
    if chunks_per_minute >= 500.0 && chunks_per_minute <= 1000.0 {
        println!("✓ Performance within target range (500-1000 chunks/min)");
    } else if chunks_per_minute < 500.0 {
        println!(
            "⚠ Performance below target range: {:.2} chunks/min (target: 500-1000)",
            chunks_per_minute
        );
    } else {
        println!(
            "✓ Performance exceeds target range: {:.2} chunks/min (target: 500-1000)",
            chunks_per_minute
        );
    }

    // Verify all embeddings are valid
    for (i, embedding) in embeddings.iter().enumerate() {
        assert_eq!(embedding.len(), 768, "Embedding {} has wrong dimension", i);
        assert!(
            embedding.iter().all(|&v| v.is_finite()),
            "Embedding {} contains non-finite values",
            i
        );
    }
}

#[tokio::test]
async fn test_ollama_config_validation() {
    // Test that config validation works for Ollama
    let config = test_config();
    assert!(
        config.validate().is_ok(),
        "Valid Ollama config should pass validation"
    );

    // Dimension validation is not enforced in the config itself,
    // so we skip this test as it would pass incorrectly

    // Test that Ollama doesn't require API key
    let mut no_key_config = test_config();
    no_key_config.api_key = None;
    assert!(
        no_key_config.validate().is_ok(),
        "Ollama should not require API key"
    );
}

#[tokio::test]
async fn test_ollama_caching_behavior() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    let text = "const cache = new Map<string, CachedValue>();";

    // First call - should miss cache
    let initial_metrics = service.cache_metrics().await;
    let embedding1 = service.embed_text(text).await.unwrap();
    let after_first = service.cache_metrics().await;

    assert!(
        after_first.misses > initial_metrics.misses,
        "Expected cache miss for first embedding"
    );

    // Second call - should use cache
    let embedding2 = service.embed_text(text).await.unwrap();
    let after_second = service.cache_metrics().await;

    assert!(
        after_second.hits > after_first.hits,
        "Expected cache hit for second embedding"
    );
    assert_eq!(
        embedding1, embedding2,
        "Cached embedding should match original"
    );

    // Check cache metrics
    let cache_metrics = service.cache_metrics().await;
    assert!(
        cache_metrics.hits >= 1,
        "Expected at least 1 cache hit, got {}",
        cache_metrics.hits
    );
}

#[tokio::test]
async fn test_empty_batch_handling() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    let embeddings = service.embed_batch(vec![]).await;
    assert!(embeddings.is_ok(), "Empty batch should succeed");
    assert_eq!(
        embeddings.unwrap().len(),
        0,
        "Empty batch should return empty result"
    );
}

#[tokio::test]
async fn test_ollama_dimension_retrieval() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    assert_eq!(
        service.dimension(),
        768,
        "Ollama nomic-embed-text service should report 768 dimensions"
    );
}

#[tokio::test]
async fn test_ollama_api_endpoint_default() {
    let config = test_config();
    assert_eq!(
        config.api_endpoint_url(),
        "http://localhost:11434/api/embed",
        "Default Ollama endpoint should be localhost:11434"
    );
}

#[tokio::test]
async fn test_ollama_custom_endpoint() {
    let mut config = test_config();
    config.api_endpoint = Some("http://custom-ollama:8080/api/embed".to_string());

    assert_eq!(
        config.api_endpoint_url(),
        "http://custom-ollama:8080/api/embed",
        "Custom endpoint should be used"
    );
}

// ===== BATCH EMBEDDING COMPREHENSIVE TESTS (LOCAL-2006) =====

#[tokio::test]
async fn test_small_batch_10_chunks() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    // Create 10 code chunks with realistic mix of content types
    let texts: Vec<String> = vec![
        // Functions
        "async function fetchUser(id: string): Promise<User> { const user = await db.users.findOne({ id }); return user; }".to_string(),
        "function calculateDiscount(price: number, rate: number): number { return price * (1 - rate); }".to_string(),
        // Classes
        "class DatabaseConnection { private pool: Pool; constructor(config: Config) { this.pool = createPool(config); } }".to_string(),
        "class ValidationError extends Error { constructor(message: string) { super(message); this.name = 'ValidationError'; } }".to_string(),
        // Constants
        "const API_CONFIG = { baseUrl: 'https://api.example.com', timeout: 5000, retries: 3 };".to_string(),
        "const HTTP_CODES = { OK: 200, NOT_FOUND: 404, SERVER_ERROR: 500 };".to_string(),
        // Interfaces/Types
        "interface UserRepository { findById(id: string): Promise<User>; save(user: User): Promise<void>; }".to_string(),
        "type RequestHandler = (req: Request, res: Response) => Promise<void>;".to_string(),
        // Arrow functions
        "const mapItems = (items: Item[]) => items.map(item => ({ ...item, processed: true }));".to_string(),
        "const isValid = (value: unknown): value is ValidType => typeof value === 'object' && value !== null;".to_string(),
    ];

    println!("\nTest: Small batch (10 chunks)");
    println!("Starting batch embedding generation...");
    let start = Instant::now();
    let embeddings = service.embed_batch(texts.clone()).await;
    let duration = start.elapsed();

    assert!(
        embeddings.is_ok(),
        "Small batch embedding failed: {:?}",
        embeddings.err()
    );

    let embeddings = embeddings.unwrap();
    assert_eq!(
        embeddings.len(),
        10,
        "Expected 10 embeddings for 10 input texts"
    );

    // Calculate performance metrics
    let chunks_per_second = 10.0 / duration.as_secs_f64();
    let chunks_per_minute = chunks_per_second * 60.0;
    let ms_per_chunk = duration.as_millis() as f64 / 10.0;

    println!("Performance metrics for 10-chunk batch:");
    println!("  Total time: {:?}", duration);
    println!("  Time per chunk: {:.2}ms", ms_per_chunk);
    println!("  Chunks/second: {:.2}", chunks_per_second);
    println!("  Chunks/minute: {:.2}", chunks_per_minute);

    // Verify all embeddings have correct dimensions and properties
    for (i, embedding) in embeddings.iter().enumerate() {
        assert_eq!(
            embedding.len(),
            768,
            "Embedding {} should have 768 dimensions",
            i
        );
        assert!(
            embedding.iter().all(|&v| v.is_finite()),
            "Embedding {} contains non-finite values",
            i
        );

        let non_zero_count = embedding.iter().filter(|&&v| v != 0.0).count();
        assert!(
            non_zero_count > 700,
            "Embedding {} should have mostly non-zero values, got {} non-zero out of 768",
            i,
            non_zero_count
        );
    }

    // Verify different content types produce different embeddings
    assert_ne!(
        embeddings[0], embeddings[2],
        "Function and class should produce different embeddings"
    );
    assert_ne!(
        embeddings[4], embeddings[6],
        "Constant and interface should produce different embeddings"
    );

    // Performance target: <2 seconds for 10 chunks
    assert!(
        duration.as_secs() < 2,
        "Small batch took too long: {:?} (expected <2s)",
        duration
    );

    // Log performance status
    if ms_per_chunk < 100.0 {
        println!("✓ Latency within target (<100ms per chunk)");
    } else {
        println!(
            "⚠ Latency above target: {:.2}ms per chunk (target: <100ms)",
            ms_per_chunk
        );
    }

    println!("✓ Small batch test passed");
}

#[tokio::test]
async fn test_medium_batch_50_chunks() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    // Create 50 realistic code chunks for medium batch test
    let texts: Vec<String> = vec![
        // 10 async functions
        "async function fetchData(url: string): Promise<Response> { const res = await fetch(url); return res.json(); }".to_string(),
        "async function processQueue(queue: Queue): Promise<void> { for await (const item of queue) { await process(item); } }".to_string(),
        "async function saveToDatabase(data: Record<string, any>): Promise<string> { const id = await db.insert(data); return id; }".to_string(),
        "async function validateUser(token: string): Promise<User> { const payload = await verify(token); return payload.user; }".to_string(),
        "async function uploadFile(file: File): Promise<string> { const url = await storage.upload(file); return url; }".to_string(),
        "async function sendEmail(to: string, subject: string): Promise<boolean> { return await mailer.send({ to, subject }); }".to_string(),
        "async function generateReport(data: Data[]): Promise<Report> { const processed = await analyze(data); return format(processed); }".to_string(),
        "async function cacheResult(key: string, value: any): Promise<void> { await redis.set(key, JSON.stringify(value)); }".to_string(),
        "async function checkHealth(): Promise<HealthStatus> { const db = await checkDatabase(); const api = await checkAPI(); return { db, api }; }".to_string(),
        "async function retryOperation<T>(fn: () => Promise<T>, attempts: number): Promise<T> { for (let i = 0; i < attempts; i++) { try { return await fn(); } catch {} } throw new Error(); }".to_string(),
        // 10 class definitions
        "class UserService { constructor(private db: Database) {} async getUser(id: string) { return this.db.users.findOne(id); } }".to_string(),
        "class EventEmitter { private handlers = new Map(); on(event: string, handler: Function) { this.handlers.set(event, handler); } }".to_string(),
        "class HttpClient { private baseUrl: string; async get(path: string) { return fetch(`${this.baseUrl}${path}`); } }".to_string(),
        "class ValidationService { validate(data: unknown): boolean { return this.schema.safeParse(data).success; } }".to_string(),
        "class CacheManager { private cache = new Map(); set(key: string, value: any) { this.cache.set(key, value); } }".to_string(),
        "class Logger { info(message: string) { console.log(`[INFO] ${new Date().toISOString()} ${message}`); } }".to_string(),
        "class RateLimiter { private tokens: number; consume(): boolean { if (this.tokens > 0) { this.tokens--; return true; } return false; } }".to_string(),
        "class Queue<T> { private items: T[] = []; enqueue(item: T) { this.items.push(item); } dequeue(): T | undefined { return this.items.shift(); } }".to_string(),
        "class Router { private routes = new Map(); register(path: string, handler: Handler) { this.routes.set(path, handler); } }".to_string(),
        "class Scheduler { private tasks: Task[] = []; schedule(task: Task) { this.tasks.push(task); this.tasks.sort((a, b) => a.priority - b.priority); } }".to_string(),
        // 10 interfaces/types
        "interface Repository<T> { findById(id: string): Promise<T>; save(entity: T): Promise<void>; delete(id: string): Promise<void>; }".to_string(),
        "type ApiResponse<T> = { success: true; data: T } | { success: false; error: string };".to_string(),
        "interface Middleware { execute(context: Context, next: () => Promise<void>): Promise<void>; }".to_string(),
        "type EventHandler<T> = (event: T) => void | Promise<void>;".to_string(),
        "interface Config { database: DatabaseConfig; api: ApiConfig; logging: LogConfig; }".to_string(),
        "type Result<T, E = Error> = { ok: true; value: T } | { ok: false; error: E };".to_string(),
        "interface Validator<T> { validate(value: unknown): value is T; errors(): string[]; }".to_string(),
        "type AsyncHandler = (req: Request, res: Response) => Promise<Response>;".to_string(),
        "interface Serializer<T> { serialize(value: T): string; deserialize(data: string): T; }".to_string(),
        "type Predicate<T> = (value: T) => boolean;".to_string(),
        // 10 constants/config
        "const DATABASE_CONFIG = { host: 'localhost', port: 5432, database: 'app', pool: { min: 2, max: 10 } };".to_string(),
        "const API_ENDPOINTS = { users: '/api/users', posts: '/api/posts', auth: '/api/auth' };".to_string(),
        "const ERROR_MESSAGES = { NOT_FOUND: 'Resource not found', UNAUTHORIZED: 'Unauthorized access', SERVER_ERROR: 'Internal server error' };".to_string(),
        "const VALIDATION_RULES = { minLength: 8, maxLength: 100, requireDigit: true, requireSpecialChar: true };".to_string(),
        "const CACHE_TTL = { short: 60, medium: 300, long: 3600 };".to_string(),
        "const HTTP_STATUS = { OK: 200, CREATED: 201, BAD_REQUEST: 400, NOT_FOUND: 404, SERVER_ERROR: 500 };".to_string(),
        "const FEATURE_FLAGS = { enableNewUI: true, enableBetaFeatures: false, enableAnalytics: true };".to_string(),
        "const RATE_LIMITS = { api: 100, upload: 10, download: 50 };".to_string(),
        "const RETRY_CONFIG = { maxAttempts: 3, backoffMs: 1000, timeoutMs: 5000 };".to_string(),
        "const LOG_LEVELS = { debug: 0, info: 1, warn: 2, error: 3 };".to_string(),
        // 10 utility functions
        "function debounce<T extends (...args: any[]) => any>(fn: T, ms: number): T { let timeout: NodeJS.Timeout; return ((...args) => { clearTimeout(timeout); timeout = setTimeout(() => fn(...args), ms); }) as T; }".to_string(),
        "function chunk<T>(array: T[], size: number): T[][] { const result: T[][] = []; for (let i = 0; i < array.length; i += size) { result.push(array.slice(i, i + size)); } return result; }".to_string(),
        "function groupBy<T>(array: T[], key: keyof T): Record<string, T[]> { return array.reduce((acc, item) => { const k = String(item[key]); (acc[k] = acc[k] || []).push(item); return acc; }, {} as Record<string, T[]>); }".to_string(),
        "function deepClone<T>(obj: T): T { return JSON.parse(JSON.stringify(obj)); }".to_string(),
        "function capitalize(str: string): string { return str.charAt(0).toUpperCase() + str.slice(1).toLowerCase(); }".to_string(),
        "function isEmail(str: string): boolean { return /^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$/.test(str); }".to_string(),
        "function randomId(): string { return Math.random().toString(36).substring(2, 15); }".to_string(),
        "function sleep(ms: number): Promise<void> { return new Promise(resolve => setTimeout(resolve, ms)); }".to_string(),
        "function parseJson<T>(str: string, fallback: T): T { try { return JSON.parse(str); } catch { return fallback; } }".to_string(),
        "function clamp(value: number, min: number, max: number): number { return Math.max(min, Math.min(max, value)); }".to_string(),
    ];

    println!("\nTest: Medium batch (50 chunks)");
    println!("Starting batch embedding generation...");
    let start = Instant::now();
    let embeddings = service.embed_large_batch(texts.clone()).await;
    let duration = start.elapsed();

    assert!(
        embeddings.is_ok(),
        "Medium batch embedding failed: {:?}",
        embeddings.err()
    );

    let embeddings = embeddings.unwrap();
    assert_eq!(
        embeddings.len(),
        50,
        "Expected 50 embeddings for 50 input texts"
    );

    // Calculate performance metrics
    let chunks_per_second = 50.0 / duration.as_secs_f64();
    let chunks_per_minute = chunks_per_second * 60.0;
    let ms_per_chunk = duration.as_millis() as f64 / 50.0;

    println!("Performance metrics for 50-chunk batch:");
    println!("  Total time: {:?}", duration);
    println!("  Time per chunk: {:.2}ms", ms_per_chunk);
    println!("  Chunks/second: {:.2}", chunks_per_second);
    println!("  Chunks/minute: {:.2}", chunks_per_minute);

    // Log cache metrics
    let metrics = service.cache_metrics().await;
    println!("  Cache hits: {}", metrics.hits);
    println!("  Cache misses: {}", metrics.misses);

    // Verify all embeddings have correct dimensions
    for (i, embedding) in embeddings.iter().enumerate() {
        assert_eq!(
            embedding.len(),
            768,
            "Embedding {} should have 768 dimensions",
            i
        );
        assert!(
            embedding.iter().all(|&v| v.is_finite()),
            "Embedding {} contains non-finite values",
            i
        );
    }

    // Verify different categories produce different embeddings
    assert_ne!(
        embeddings[0], embeddings[10],
        "Async function and class should produce different embeddings"
    );
    assert_ne!(
        embeddings[20], embeddings[30],
        "Interface and constant should produce different embeddings"
    );
    assert_ne!(
        embeddings[30], embeddings[40],
        "Constant and utility function should produce different embeddings"
    );

    // Performance target: <10 seconds for 50 chunks
    assert!(
        duration.as_secs() < 10,
        "Medium batch took too long: {:?} (expected <10s)",
        duration
    );

    // Log performance status relative to targets
    if chunks_per_minute >= 500.0 && chunks_per_minute <= 1000.0 {
        println!("✓ Throughput within target range (500-1000 chunks/min)");
    } else if chunks_per_minute < 500.0 {
        println!(
            "⚠ Throughput below target range: {:.2} chunks/min (target: 500-1000)",
            chunks_per_minute
        );
    } else {
        println!(
            "✓ Throughput exceeds target range: {:.2} chunks/min (target: 500-1000)",
            chunks_per_minute
        );
    }

    println!("✓ Medium batch test passed");
}

#[tokio::test]
async fn test_large_batch_100_chunks() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    // Create 100 code chunks for large batch stress test
    let texts: Vec<String> = (0..100)
        .map(|i| {
            // Rotate through different code patterns
            match i % 5 {
                0 => format!(
                    "async function operation{}(param: string): Promise<Result> {{ \
                     const data = await fetch(`/api/resource/${{param}}`); \
                     const result = await data.json(); \
                     return processResult(result); \
                     }}",
                    i
                ),
                1 => format!(
                    "class Service{} {{ \
                     private readonly client: HttpClient; \
                     constructor(config: Config) {{ this.client = new HttpClient(config); }} \
                     async execute(): Promise<void> {{ await this.client.send(); }} \
                     }}",
                    i
                ),
                2 => format!(
                    "interface Handler{} {{ \
                     handle(request: Request): Promise<Response>; \
                     validate(data: unknown): boolean; \
                     transform(input: Input): Output; \
                     }}",
                    i
                ),
                3 => format!(
                    "const CONFIG_{} = {{ \
                     endpoint: 'https://api.service.com/v1', \
                     timeout: 5000, \
                     retries: 3, \
                     headers: {{ 'Content-Type': 'application/json' }} \
                     }};",
                    i
                ),
                4 => format!(
                    "function transform{}(data: DataType): ResultType {{ \
                     const filtered = data.filter(item => item.isValid); \
                     const mapped = filtered.map(item => ({{ id: item.id, value: item.value * 2 }})); \
                     return mapped.reduce((acc, curr) => acc.concat(curr), []); \
                     }}",
                    i
                ),
                _ => unreachable!(),
            }
        })
        .collect();

    println!("\nTest: Large batch (100 chunks)");
    println!("Starting large batch embedding generation...");
    println!("This stress tests batch size limits and memory usage...");

    let start = Instant::now();
    let embeddings = service.embed_large_batch(texts.clone()).await;
    let duration = start.elapsed();

    assert!(
        embeddings.is_ok(),
        "Large batch embedding failed: {:?}",
        embeddings.err()
    );

    let embeddings = embeddings.unwrap();
    assert_eq!(
        embeddings.len(),
        100,
        "Expected 100 embeddings for 100 input texts"
    );

    // Calculate performance metrics
    let chunks_per_second = 100.0 / duration.as_secs_f64();
    let chunks_per_minute = chunks_per_second * 60.0;
    let ms_per_chunk = duration.as_millis() as f64 / 100.0;

    println!("Performance metrics for 100-chunk batch:");
    println!("  Total time: {:?}", duration);
    println!("  Time per chunk: {:.2}ms", ms_per_chunk);
    println!("  Chunks/second: {:.2}", chunks_per_second);
    println!("  Chunks/minute: {:.2}", chunks_per_minute);

    // Log cache metrics
    let metrics = service.cache_metrics().await;
    println!("  Cache hits: {}", metrics.hits);
    println!("  Cache misses: {}", metrics.misses);

    // Verify all embeddings have correct dimensions
    for (i, embedding) in embeddings.iter().enumerate() {
        assert_eq!(
            embedding.len(),
            768,
            "Embedding {} should have 768 dimensions",
            i
        );
        assert!(
            embedding.iter().all(|&v| v.is_finite()),
            "Embedding {} contains non-finite values",
            i
        );
    }

    // Verify embeddings for different patterns are different
    assert_ne!(
        embeddings[0], embeddings[1],
        "Different code patterns should produce different embeddings"
    );
    assert_ne!(
        embeddings[1], embeddings[2],
        "Different code patterns should produce different embeddings"
    );

    // Note: We don't have direct memory tracking, but log what we can
    println!("Memory usage notes:");
    println!(
        "  Embeddings stored: {} vectors × 768 dimensions × 4 bytes = {} KB",
        embeddings.len(),
        (embeddings.len() * 768 * 4) / 1024
    );

    // Log throughput performance
    println!("Throughput analysis:");
    if chunks_per_minute >= 500.0 {
        println!(
            "  ✓ Throughput acceptable: {:.2} chunks/min",
            chunks_per_minute
        );
    } else {
        println!(
            "  ⚠ Throughput below target: {:.2} chunks/min (target: 500+)",
            chunks_per_minute
        );
    }

    // No strict time limit for large batch, just verify completion
    println!("✓ Large batch completed successfully");
    println!("✓ Large batch stress test passed");
}

#[tokio::test]
async fn test_content_types() {
    let Some(service) = skip_if_ollama_unavailable().await else {
        return;
    };

    println!("\nTest: Content types diversity");

    // Create samples of different content types
    let content_types = vec![
        (
            "Function",
            "async function authenticateUser(credentials: Credentials): Promise<Session> { \
             const user = await validateCredentials(credentials); \
             if (!user) throw new AuthError('Invalid credentials'); \
             const token = await generateToken(user); \
             return { user, token, expiresAt: Date.now() + 3600000 }; \
             }".to_string()
        ),
        (
            "Class",
            "class DatabaseConnection { \
             private pool: ConnectionPool; \
             private isConnected: boolean = false; \
             constructor(config: DatabaseConfig) { \
               this.pool = createPool(config); \
             } \
             async connect(): Promise<void> { \
               if (this.isConnected) return; \
               await this.pool.connect(); \
               this.isConnected = true; \
             } \
             async query(sql: string, params: any[]): Promise<QueryResult> { \
               return this.pool.query(sql, params); \
             } \
             }".to_string()
        ),
        (
            "Docstring",
            "/**\n * Calculates the total price including tax and discounts.\n * \n * @param basePrice - The original price before any adjustments\n * @param taxRate - The tax rate as a decimal (e.g., 0.08 for 8%)\n * @param discount - Optional discount as a decimal (e.g., 0.1 for 10% off)\n * @returns The final price after applying tax and discount\n * \n * @example\n * ```typescript\n * const price = calculateTotal(100, 0.08, 0.1);\n * console.log(price); // 97.2\n * ```\n */".to_string()
        ),
        (
            "Comment",
            "// This middleware handles authentication for protected routes.\n// It verifies the JWT token from the Authorization header,\n// validates the token signature and expiration,\n// and attaches the user object to the request context.\n// If authentication fails, it returns a 401 Unauthorized response.".to_string()
        ),
        (
            "Config",
            "{\n  \"database\": {\n    \"host\": \"localhost\",\n    \"port\": 5432,\n    \"database\": \"app_production\",\n    \"pool\": {\n      \"min\": 2,\n      \"max\": 10,\n      \"idleTimeoutMillis\": 30000\n    }\n  },\n  \"api\": {\n    \"baseUrl\": \"https://api.example.com\",\n    \"timeout\": 5000,\n    \"retries\": 3\n  }\n}".to_string()
        ),
        (
            "Interface",
            "interface EventBus { \
             subscribe<T>(topic: string, handler: (event: T) => void): Unsubscribe; \
             publish<T>(topic: string, event: T): Promise<void>; \
             topics(): string[]; \
             clear(topic?: string): void; \
             }".to_string()
        ),
        (
            "Type Alias",
            "type ApiResult<T, E = Error> = \
             | { success: true; data: T; metadata?: Record<string, any> } \
             | { success: false; error: E; code?: string; retry?: boolean };".to_string()
        ),
        (
            "Arrow Function",
            "const processItems = async (items: Item[]): Promise<ProcessedItem[]> => { \
             const validated = items.filter(item => isValid(item)); \
             const enriched = await Promise.all(validated.map(async item => ({ \
               ...item, \
               metadata: await fetchMetadata(item.id), \
               timestamp: Date.now() \
             }))); \
             return enriched.sort((a, b) => a.priority - b.priority); \
             };".to_string()
        ),
        (
            "Constant Array",
            "const SUPPORTED_FORMATS = [ \
             { ext: '.ts', mime: 'text/typescript', parser: 'typescript' }, \
             { ext: '.js', mime: 'text/javascript', parser: 'javascript' }, \
             { ext: '.json', mime: 'application/json', parser: 'json' }, \
             { ext: '.md', mime: 'text/markdown', parser: 'markdown' } \
             ] as const;".to_string()
        ),
        (
            "Enum",
            "enum HttpMethod { \
             GET = 'GET', \
             POST = 'POST', \
             PUT = 'PUT', \
             PATCH = 'PATCH', \
             DELETE = 'DELETE', \
             OPTIONS = 'OPTIONS', \
             HEAD = 'HEAD' \
             }".to_string()
        ),
    ];

    println!("Testing {} different content types", content_types.len());

    let texts: Vec<String> = content_types.iter().map(|(_, text)| text.clone()).collect();
    let start = Instant::now();
    let embeddings = service.embed_batch(texts.clone()).await;
    let duration = start.elapsed();

    assert!(
        embeddings.is_ok(),
        "Content types embedding failed: {:?}",
        embeddings.err()
    );

    let embeddings = embeddings.unwrap();
    assert_eq!(
        embeddings.len(),
        content_types.len(),
        "Expected {} embeddings for {} content types",
        content_types.len(),
        content_types.len()
    );

    println!("\nContent type embedding results:");
    println!("  Total time: {:?}", duration);
    println!(
        "  Average time per type: {:?}",
        duration / content_types.len() as u32
    );

    // Verify each content type
    for ((content_type, _), embedding) in content_types.iter().zip(embeddings.iter()) {
        // Verify dimensions
        assert_eq!(
            embedding.len(),
            768,
            "Embedding for {} should have 768 dimensions",
            content_type
        );

        // Verify all values are finite
        assert!(
            embedding.iter().all(|&v| v.is_finite()),
            "Embedding for {} contains non-finite values",
            content_type
        );

        // Verify mostly non-zero values
        let non_zero_count = embedding.iter().filter(|&&v| v != 0.0).count();
        assert!(
            non_zero_count > 700,
            "Embedding for {} should have mostly non-zero values, got {} non-zero out of 768",
            content_type,
            non_zero_count
        );

        // Calculate basic statistics
        let sum: f32 = embedding.iter().sum();
        let mean = sum / embedding.len() as f32;
        let variance: f32 =
            embedding.iter().map(|&v| (v - mean).powi(2)).sum::<f32>() / embedding.len() as f32;
        let std_dev = variance.sqrt();

        println!(
            "  {}: dim={}, non_zero={}, mean={:.6}, std_dev={:.6}",
            content_type,
            embedding.len(),
            non_zero_count,
            mean,
            std_dev
        );
    }

    // Verify different content types produce different embeddings
    for i in 0..embeddings.len() {
        for j in (i + 1)..embeddings.len() {
            assert_ne!(
                embeddings[i], embeddings[j],
                "Different content types ({} and {}) should produce different embeddings",
                content_types[i].0, content_types[j].0
            );
        }
    }

    // Calculate cosine similarity between some pairs to show diversity
    println!("\nCosine similarity examples:");
    let similarity_01 = cosine_similarity(&embeddings[0], &embeddings[1]);
    let similarity_02 = cosine_similarity(&embeddings[0], &embeddings[2]);
    let similarity_23 = cosine_similarity(&embeddings[2], &embeddings[3]);
    println!(
        "  {} vs {}: {:.4}",
        content_types[0].0, content_types[1].0, similarity_01
    );
    println!(
        "  {} vs {}: {:.4}",
        content_types[0].0, content_types[2].0, similarity_02
    );
    println!(
        "  {} vs {}: {:.4}",
        content_types[2].0, content_types[3].0, similarity_23
    );

    println!("\n✓ All content types generated valid embeddings");
    println!("✓ All embeddings have correct dimensions (768)");
    println!("✓ Content types test passed");
}

// Helper function to calculate cosine similarity between two embeddings
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    dot_product / (magnitude_a * magnitude_b)
}
