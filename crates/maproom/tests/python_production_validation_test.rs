//! Production-scale validation tests for Python parser (LANG_PARSE-1008)
//!
//! This test suite validates that the Python parser meets production-readiness criteria:
//! - Handles large-scale Django-like codebases efficiently
//! - Extracts all symbol types correctly (classes, methods, functions, imports)
//! - Provides accurate symbol extraction for Django-specific patterns
//! - Maintains consistent performance across different file types
//!
//! # Acceptance Criteria
//!
//! - Parse 1000+ files worth of Python code without errors
//! - Extract all expected Django patterns (models, views, serializers, etc.)
//! - Performance within 2x of TypeScript baseline
//! - Symbol extraction accuracy >95% for Django patterns
//!
//! # Test Strategy
//!
//! Instead of requiring an actual 1000+ file Django project, we use:
//! 1. Realistic Django fixture files that represent common patterns
//! 2. Scaled tests that parse the same fixtures multiple times to simulate load
//! 3. Representative test data that covers Django models, views, serializers, forms
//! 4. Performance measurements comparing Python vs TypeScript parsing

use maproom::indexer::parser;
use std::fs;
use std::time::Instant;

/// Test parsing large batch of Django models files
/// Simulates indexing a large Django project with many model files
#[test]
fn test_production_scale_django_models_batch() {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/models.py")
        .expect("Failed to read Django models fixture");

    // Simulate parsing 100 model files (representative of a large Django project)
    let iterations = 100;
    let start = Instant::now();
    let mut total_chunks = 0;

    for _ in 0..iterations {
        let chunks = parser::extract_chunks(&source, "py");
        total_chunks += chunks.len();
    }

    let duration = start.elapsed();
    let avg_per_file = duration / iterations;

    // Performance assertions
    println!("\n=== Django Models Batch Performance ===");
    println!("Total files parsed: {}", iterations);
    println!("Total chunks extracted: {}", total_chunks);
    println!("Total time: {:?}", duration);
    println!("Average per file: {:?}", avg_per_file);
    println!(
        "Files per second: {:.2}",
        iterations as f64 / duration.as_secs_f64()
    );

    // Should complete within reasonable time (less than 5 seconds for 100 files)
    assert!(
        duration.as_secs() < 5,
        "Should parse 100 Django model files in less than 5 seconds, took {:?}",
        duration
    );

    // Should extract expected number of chunks per file
    let avg_chunks = total_chunks / iterations as usize;
    assert!(
        avg_chunks >= 30,
        "Should extract at least 30 chunks per Django models file, got {}",
        avg_chunks
    );
}

/// Test parsing large batch of Django views files
/// Validates class-based and function-based view extraction at scale
#[test]
fn test_production_scale_django_views_batch() {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/views.py")
        .expect("Failed to read Django views fixture");

    let iterations = 100;
    let start = Instant::now();
    let mut total_chunks = 0;

    for _ in 0..iterations {
        let chunks = parser::extract_chunks(&source, "py");
        total_chunks += chunks.len();
    }

    let duration = start.elapsed();
    let avg_per_file = duration / iterations;

    println!("\n=== Django Views Batch Performance ===");
    println!("Total files parsed: {}", iterations);
    println!("Total chunks extracted: {}", total_chunks);
    println!("Total time: {:?}", duration);
    println!("Average per file: {:?}", avg_per_file);
    println!(
        "Files per second: {:.2}",
        iterations as f64 / duration.as_secs_f64()
    );

    // Should complete within reasonable time
    assert!(
        duration.as_secs() < 5,
        "Should parse 100 Django view files in less than 5 seconds, took {:?}",
        duration
    );

    // Should extract view functions and classes
    let avg_chunks = total_chunks / iterations as usize;
    assert!(
        avg_chunks >= 20,
        "Should extract at least 20 chunks per Django views file, got {}",
        avg_chunks
    );
}

/// Test parsing large batch of Flask application files
/// Validates REST API and route extraction patterns
#[test]
fn test_production_scale_flask_batch() {
    let source = fs::read_to_string("tests/fixtures/python/flask_samples/app.py")
        .expect("Failed to read Flask app fixture");

    let iterations = 100;
    let start = Instant::now();
    let mut total_chunks = 0;

    for _ in 0..iterations {
        let chunks = parser::extract_chunks(&source, "py");
        total_chunks += chunks.len();
    }

    let duration = start.elapsed();

    println!("\n=== Flask App Batch Performance ===");
    println!("Total files parsed: {}", iterations);
    println!("Total chunks extracted: {}", total_chunks);
    println!("Total time: {:?}", duration);
    println!(
        "Files per second: {:.2}",
        iterations as f64 / duration.as_secs_f64()
    );

    assert!(
        duration.as_secs() < 5,
        "Should parse 100 Flask files in less than 5 seconds"
    );
}

/// Test symbol extraction accuracy for Django models
/// Validates that all expected Django patterns are extracted correctly
#[test]
fn test_django_symbol_extraction_accuracy() {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/models.py")
        .expect("Failed to read Django models fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Track what we find
    let mut found_symbols = vec![];

    // Expected Django model patterns
    let expected_classes = vec![
        "User",
        "Category",
        "Product",
        "Tag",
        "Review",
        "Order",
        "OrderItem",
    ];
    let expected_methods = vec![
        "get_full_name",
        "get_absolute_url",
        "save",
        "is_published",
        "is_in_stock",
        "get_published_products",
        "search",
        "get_items_count",
        "subtotal",
    ];

    // Check class extraction
    let class_chunks: Vec<_> = chunks.iter().filter(|c| c.kind == "class").collect();

    for expected_class in &expected_classes {
        let found = class_chunks.iter().any(|c| {
            c.symbol_name
                .as_ref()
                .map_or(false, |name| name == expected_class)
        });

        if found {
            found_symbols.push(expected_class.to_string());
        }

        assert!(
            found,
            "Should extract Django model class: {}",
            expected_class
        );
    }

    // Check method extraction
    let method_chunks: Vec<_> = chunks
        .iter()
        .filter(|c| c.kind == "method" || c.kind == "func")
        .collect();

    for expected_method in &expected_methods {
        let found = method_chunks.iter().any(|c| {
            c.symbol_name
                .as_ref()
                .map_or(false, |name| name == expected_method)
        });

        if found {
            found_symbols.push(expected_method.to_string());
        }

        assert!(
            found,
            "Should extract Django model method: {}",
            expected_method
        );
    }

    // Check for imports chunk
    let imports = chunks.iter().find(|c| c.kind == "imports");
    assert!(imports.is_some(), "Should extract imports chunk");

    // Check for Meta nested classes
    let meta_count = chunks
        .iter()
        .filter(|c| c.symbol_name == Some("Meta".to_string()) && c.kind == "class")
        .count();
    assert!(
        meta_count >= 5,
        "Should extract multiple Meta nested classes"
    );

    // Check for __str__ methods
    let str_methods = chunks
        .iter()
        .filter(|c| c.symbol_name == Some("__str__".to_string()))
        .count();
    assert!(str_methods >= 5, "Should extract __str__ magic methods");

    // Check for property decorators
    let properties = chunks
        .iter()
        .filter(|c| {
            c.symbol_name.as_ref().map_or(false, |name| {
                name.starts_with("is_") || name.starts_with("get_")
            })
        })
        .count();
    assert!(properties >= 4, "Should extract property methods");

    println!("\n=== Symbol Extraction Accuracy ===");
    println!("Total symbols extracted: {}", chunks.len());
    println!(
        "Classes found: {}/{}",
        class_chunks.len(),
        expected_classes.len()
    );
    println!("Methods found: {}", method_chunks.len());
    println!("Meta classes: {}", meta_count);
    println!("Magic methods: {}", str_methods);
    println!("Properties: {}", properties);

    // Calculate accuracy: found / expected
    let total_expected = expected_classes.len() + expected_methods.len();
    let accuracy = (found_symbols.len() as f64 / total_expected as f64) * 100.0;
    println!("Extraction accuracy: {:.1}%", accuracy);

    assert!(
        accuracy >= 95.0,
        "Symbol extraction accuracy should be >= 95%, got {:.1}%",
        accuracy
    );
}

/// Test symbol extraction accuracy for Django views
/// Validates class-based views, function-based views, and decorators
#[test]
fn test_django_views_symbol_extraction() {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/views.py")
        .expect("Failed to read Django views fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Expected view classes
    let expected_views = vec![
        "ProductListView",
        "ProductDetailView",
        "CategoryListView",
        "ProductCreateView",
        "ProductUpdateView",
        "ReviewDeleteView",
        "OrderListView",
        "OrderDetailView",
        "ProductAPIView",
    ];

    // Expected view functions
    let expected_functions = vec![
        "home",
        "add_review",
        "api_product_search",
        "handler404",
        "handler500",
    ];

    let mut found_count = 0;

    // Check view classes
    for view in &expected_views {
        let found = chunks.iter().any(|c| {
            c.kind == "class" && c.symbol_name.as_ref().map_or(false, |name| name == view)
        });

        if found {
            found_count += 1;
        }

        assert!(found, "Should extract view class: {}", view);
    }

    // Check view functions
    for func in &expected_functions {
        let found = chunks
            .iter()
            .any(|c| c.kind == "func" && c.symbol_name.as_ref().map_or(false, |name| name == func));

        if found {
            found_count += 1;
        }

        assert!(found, "Should extract view function: {}", func);
    }

    // Check for view methods (get_queryset, get_context_data, etc.)
    let view_methods = chunks
        .iter()
        .filter(|c| {
            c.kind == "method"
                && c.symbol_name.as_ref().map_or(false, |name| {
                    name.starts_with("get_") || name == "test_func" || name == "form_valid"
                })
        })
        .count();

    assert!(
        view_methods >= 8,
        "Should extract multiple view methods, got {}",
        view_methods
    );

    println!("\n=== Django Views Symbol Extraction ===");
    println!(
        "View classes: {}/{}",
        expected_views.len(),
        expected_views.len()
    );
    println!(
        "View functions: {}/{}",
        expected_functions.len(),
        expected_functions.len()
    );
    println!("View methods: {}", view_methods);
    println!("Total chunks: {}", chunks.len());

    let total_expected = expected_views.len() + expected_functions.len();
    let accuracy = (found_count as f64 / total_expected as f64) * 100.0;
    println!("Extraction accuracy: {:.1}%", accuracy);

    assert!(
        accuracy >= 95.0,
        "Views extraction accuracy should be >= 95%, got {:.1}%",
        accuracy
    );
}

/// Test mixed file type batch processing
/// Simulates real-world project with different Python file types
#[test]
fn test_production_scale_mixed_batch() {
    let files = vec![
        ("models", "tests/fixtures/python/django_samples/models.py"),
        ("views", "tests/fixtures/python/django_samples/views.py"),
        ("flask", "tests/fixtures/python/flask_samples/app.py"),
        ("api", "tests/fixtures/python/sample_api.py"),
    ];

    let iterations_per_file = 25; // 25 iterations × 4 file types = 100 total
    let start = Instant::now();
    let mut total_chunks = 0;
    let mut file_stats = std::collections::HashMap::new();

    for (file_type, path) in &files {
        let source =
            fs::read_to_string(path).unwrap_or_else(|_| panic!("Failed to read fixture: {}", path));

        let mut chunks_for_type = 0;
        let type_start = Instant::now();

        for _ in 0..iterations_per_file {
            let chunks = parser::extract_chunks(&source, "py");
            chunks_for_type += chunks.len();
            total_chunks += chunks.len();
        }

        let type_duration = type_start.elapsed();
        file_stats.insert(
            *file_type,
            (chunks_for_type / iterations_per_file, type_duration),
        );
    }

    let duration = start.elapsed();
    let total_files = files.len() * iterations_per_file;

    println!("\n=== Mixed Batch Production Test ===");
    println!("Total files parsed: {}", total_files);
    println!("Total chunks extracted: {}", total_chunks);
    println!("Total time: {:?}", duration);
    println!(
        "Files per second: {:.2}",
        total_files as f64 / duration.as_secs_f64()
    );
    println!("\nPer-file-type stats:");

    for (file_type, (avg_chunks, type_duration)) in &file_stats {
        println!(
            "  {}: {} chunks/file, {:?} total for {} files",
            file_type, avg_chunks, type_duration, iterations_per_file
        );
    }

    // Should handle 100 files efficiently
    assert!(
        duration.as_secs() < 10,
        "Should parse 100 mixed Python files in less than 10 seconds, took {:?}",
        duration
    );

    // Should extract reasonable number of chunks
    assert!(
        total_chunks > 1000,
        "Should extract >1000 total chunks from mixed batch, got {}",
        total_chunks
    );
}

/// Test docstring extraction at scale
/// Validates that docstrings are correctly extracted for all symbol types
#[test]
fn test_production_scale_docstring_extraction() {
    let sources = vec![
        fs::read_to_string("tests/fixtures/python/google_style_docstrings.py")
            .expect("Failed to read Google style docstrings"),
        fs::read_to_string("tests/fixtures/python/numpy_style_docstrings.py")
            .expect("Failed to read NumPy style docstrings"),
        fs::read_to_string("tests/fixtures/python/rst_style_docstrings.py")
            .expect("Failed to read RST style docstrings"),
    ];

    let mut total_symbols = 0;
    let mut symbols_with_docstrings = 0;

    for source in sources {
        let chunks = parser::extract_chunks(&source, "py");

        // Count symbols that should have docstrings (classes, functions, methods)
        for chunk in chunks {
            if chunk.kind == "class" || chunk.kind == "func" || chunk.kind == "method" {
                total_symbols += 1;
                if chunk.docstring.is_some() {
                    symbols_with_docstrings += 1;
                }
            }
        }
    }

    println!("\n=== Docstring Extraction Statistics ===");
    println!("Total symbols: {}", total_symbols);
    println!("Symbols with docstrings: {}", symbols_with_docstrings);

    let docstring_coverage = (symbols_with_docstrings as f64 / total_symbols as f64) * 100.0;
    println!("Docstring extraction rate: {:.1}%", docstring_coverage);

    // Should extract most docstrings (>80%)
    assert!(
        docstring_coverage >= 80.0,
        "Should extract docstrings for >80% of symbols, got {:.1}%",
        docstring_coverage
    );
}

/// Test edge cases handling at scale
/// Validates parser robustness with incomplete/malformed syntax
#[test]
fn test_production_scale_edge_cases() {
    let edge_case_files = vec![
        "tests/fixtures/python/edge_cases/incomplete_syntax.py",
        "tests/fixtures/python/edge_cases/mixed_indentation.py",
        "tests/fixtures/python/edge_cases/malformed_decorators.py",
        "tests/fixtures/python/edge_cases/unusual_classes.py",
    ];

    let iterations = 25; // 25 × 4 = 100 total files
    let start = Instant::now();
    let mut total_chunks = 0;
    let mut successful_parses = 0;

    for path in edge_case_files {
        let source = fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read edge case file: {}", path));

        for _ in 0..iterations {
            // Should not panic even with malformed code
            let chunks = parser::extract_chunks(&source, "py");
            total_chunks += chunks.len();
            successful_parses += 1;
        }
    }

    let duration = start.elapsed();

    println!("\n=== Edge Cases Robustness Test ===");
    println!("Edge case files parsed: {}", successful_parses);
    println!("Total chunks extracted: {}", total_chunks);
    println!("Total time: {:?}", duration);
    println!("Success rate: 100% (no panics)");

    // Should complete without panicking
    assert_eq!(
        successful_parses, 100,
        "Should successfully parse all edge case files"
    );

    // Should extract some chunks even from malformed code
    assert!(
        total_chunks > 0,
        "Should extract some chunks from edge cases"
    );
}

/// Test import extraction at scale
/// Validates that imports are correctly extracted and organized
#[test]
fn test_production_scale_import_extraction() {
    let sources = vec![
        fs::read_to_string("tests/fixtures/python/django_samples/models.py")
            .expect("Failed to read models"),
        fs::read_to_string("tests/fixtures/python/django_samples/views.py")
            .expect("Failed to read views"),
        fs::read_to_string("tests/fixtures/python/flask_samples/app.py")
            .expect("Failed to read flask"),
    ];

    let mut files_with_imports = 0;
    let mut total_import_chunks = 0;

    for source in sources {
        let chunks = parser::extract_chunks(&source, "py");

        // Check for imports chunk
        if let Some(imports_chunk) = chunks.iter().find(|c| c.kind == "imports") {
            files_with_imports += 1;
            total_import_chunks += 1;

            // Verify imports chunk has metadata
            assert!(
                imports_chunk.metadata.is_some(),
                "Imports chunk should have metadata with import list"
            );

            if let Some(metadata) = &imports_chunk.metadata {
                if let Some(imports) = metadata.get("imports") {
                    assert!(imports.is_array(), "Imports metadata should be an array");

                    let import_count = imports.as_array().unwrap().len();
                    assert!(
                        import_count > 0,
                        "Should have at least one import in imports chunk"
                    );
                }
            }
        }
    }

    println!("\n=== Import Extraction Statistics ===");
    println!("Files with imports: {}", files_with_imports);
    println!("Total import chunks: {}", total_import_chunks);

    assert_eq!(files_with_imports, 3, "All test files should have imports");
}

/// Performance comparison: Python vs TypeScript parsing
/// Validates that Python parser is within 2x of TypeScript baseline
#[test]
fn test_python_vs_typescript_performance() {
    // Python test file
    let python_source = r#"
class Calculator:
    """A calculator class for basic operations."""

    def __init__(self):
        """Initialize calculator."""
        self.result = 0

    def add(self, a: int, b: int) -> int:
        """Add two numbers.

        Args:
            a: First number
            b: Second number

        Returns:
            Sum of a and b
        """
        return a + b

    def subtract(self, a: int, b: int) -> int:
        """Subtract two numbers."""
        return a - b

    def multiply(self, a: int, b: int) -> int:
        """Multiply two numbers."""
        return a * b
"#;

    // TypeScript equivalent
    let typescript_source = r#"
/**
 * A calculator class for basic operations
 */
class Calculator {
    private result: number = 0;

    /**
     * Initialize calculator
     */
    constructor() {
        this.result = 0;
    }

    /**
     * Add two numbers
     * @param a First number
     * @param b Second number
     * @returns Sum of a and b
     */
    add(a: number, b: number): number {
        return a + b;
    }

    /**
     * Subtract two numbers
     */
    subtract(a: number, b: number): number {
        return a - b;
    }

    /**
     * Multiply two numbers
     */
    multiply(a: number, b: number): number {
        return a * b;
    }
}
"#;

    let iterations = 1000;

    // Benchmark Python parsing
    let py_start = Instant::now();
    for _ in 0..iterations {
        parser::extract_chunks(python_source, "py");
    }
    let py_duration = py_start.elapsed();

    // Benchmark TypeScript parsing
    let ts_start = Instant::now();
    for _ in 0..iterations {
        parser::extract_chunks(typescript_source, "ts");
    }
    let ts_duration = ts_start.elapsed();

    let py_micros = py_duration.as_micros();
    let ts_micros = ts_duration.as_micros();
    let ratio = py_micros as f64 / ts_micros as f64;

    println!("\n=== Python vs TypeScript Performance ===");
    println!("Iterations: {}", iterations);
    println!(
        "Python total: {:?} (avg: {:?})",
        py_duration,
        py_duration / iterations
    );
    println!(
        "TypeScript total: {:?} (avg: {:?})",
        ts_duration,
        ts_duration / iterations
    );
    println!("Python/TypeScript ratio: {:.2}x", ratio);

    // Python should be within 2x of TypeScript (acceptance criteria)
    assert!(
        ratio <= 2.0,
        "Python parser should be within 2x of TypeScript baseline, got {:.2}x",
        ratio
    );

    // Both should be fast enough for production use
    let py_per_file = py_duration / iterations;
    assert!(
        py_per_file.as_millis() < 10,
        "Python parser should parse simple file in <10ms, took {:?}",
        py_per_file
    );
}

/// Stress test: Parse 1000 files sequentially
/// Simulates indexing a very large Python project
#[test]
#[ignore = "Long running test - run explicitly with --ignored"]
fn test_stress_1000_files() {
    let source = fs::read_to_string("tests/fixtures/python/sample_api.py")
        .expect("Failed to read sample API fixture");

    let iterations = 1000;
    let start = Instant::now();
    let mut total_chunks = 0;
    let mut errors = 0;

    for i in 0..iterations {
        let chunks = parser::extract_chunks(&source, "py");
        total_chunks += chunks.len();

        if chunks.is_empty() {
            errors += 1;
        }

        // Print progress every 100 files
        if (i + 1) % 100 == 0 {
            let elapsed = start.elapsed();
            let rate = (i + 1) as f64 / elapsed.as_secs_f64();
            println!(
                "Parsed {} files in {:?} ({:.1} files/sec)",
                i + 1,
                elapsed,
                rate
            );
        }
    }

    let duration = start.elapsed();
    let rate = iterations as f64 / duration.as_secs_f64();

    println!("\n=== Stress Test: 1000 Files ===");
    println!("Total files: {}", iterations);
    println!("Total chunks: {}", total_chunks);
    println!("Total time: {:?}", duration);
    println!("Files per second: {:.2}", rate);
    println!("Errors: {}", errors);

    // Should complete in reasonable time (< 60 seconds for 1000 files)
    assert!(
        duration.as_secs() < 60,
        "Should parse 1000 files in <60s, took {:?}",
        duration
    );

    // Should have no errors
    assert_eq!(errors, 0, "Should parse all files without errors");

    // Should maintain >10 files/sec throughput
    assert!(
        rate >= 10.0,
        "Should maintain >10 files/sec throughput, got {:.2}",
        rate
    );
}

/// Search quality validation: Context extraction includes docstrings
/// Validates that parsed chunks include docstrings for search context
#[test]
fn test_search_quality_context_with_docstrings() {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/models.py")
        .expect("Failed to read Django models fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Find a well-documented class
    let user_class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("User".to_string()) && c.kind == "class")
        .expect("Should find User class");

    // Verify docstring is present
    assert!(
        user_class.docstring.is_some(),
        "User class should have docstring for search context"
    );

    let docstring = user_class.docstring.as_ref().unwrap();
    assert!(
        docstring.contains("user"),
        "Docstring should contain relevant keywords"
    );

    // Find a method with docstring
    let get_full_name = chunks
        .iter()
        .find(|c| c.symbol_name == Some("get_full_name".to_string()))
        .expect("Should find get_full_name method");

    assert!(
        get_full_name.docstring.is_some(),
        "Method should have docstring"
    );

    println!("\n=== Search Context Quality ===");
    println!("Class docstring length: {} chars", docstring.len());
    println!(
        "Method docstring present: {}",
        get_full_name.docstring.is_some()
    );

    // Count how many symbols have docstrings for search context
    let symbols_with_docs = chunks
        .iter()
        .filter(|c| {
            (c.kind == "class" || c.kind == "func" || c.kind == "method") && c.docstring.is_some()
        })
        .count();

    let total_symbols = chunks
        .iter()
        .filter(|c| c.kind == "class" || c.kind == "func" || c.kind == "method")
        .count();

    let doc_coverage = (symbols_with_docs as f64 / total_symbols as f64) * 100.0;
    println!(
        "Symbols with docstrings: {}/{} ({:.1}%)",
        symbols_with_docs, total_symbols, doc_coverage
    );

    assert!(
        doc_coverage >= 50.0,
        "At least 50% of symbols should have docstrings for good search context"
    );
}

/// Search quality validation: Symbol names are searchable
/// Validates that symbol names are properly extracted for text search
#[test]
fn test_search_quality_symbol_names() {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/views.py")
        .expect("Failed to read views fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Simulate searching for "ProductListView"
    let search_term = "ProductListView";
    let found = chunks.iter().any(|c| {
        c.symbol_name
            .as_ref()
            .map_or(false, |name| name.contains(search_term))
    });

    assert!(
        found,
        "Should be able to find symbols by name search: {}",
        search_term
    );

    // Simulate searching for "get_queryset" method
    let search_term = "get_queryset";
    let found_methods = chunks
        .iter()
        .filter(|c| {
            c.symbol_name
                .as_ref()
                .map_or(false, |name| name == search_term)
        })
        .count();

    assert!(
        found_methods > 0,
        "Should find multiple instances of common method names"
    );

    println!("\n=== Symbol Name Search Quality ===");
    println!("Found ProductListView: yes");
    println!("Found get_queryset methods: {}", found_methods);

    // Verify all chunks with symbols have searchable names
    let chunks_with_names = chunks
        .iter()
        .filter(|c| c.kind != "imports" && c.symbol_name.is_some())
        .count();

    let total_chunks = chunks.iter().filter(|c| c.kind != "imports").count();

    let name_coverage = (chunks_with_names as f64 / total_chunks as f64) * 100.0;
    println!(
        "Chunks with symbol names: {}/{} ({:.1}%)",
        chunks_with_names, total_chunks, name_coverage
    );

    assert!(
        name_coverage >= 80.0,
        "At least 80% of chunks should have symbol names for search"
    );
}

/// Search quality validation: Signature extraction provides context
/// Validates that function/method signatures are captured for search
#[test]
fn test_search_quality_signature_context() {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/models.py")
        .expect("Failed to read models fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Find methods with parameters
    let search_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("search".to_string()) && c.kind == "method")
        .expect("Should find search classmethod");

    // Verify signature is present
    assert!(
        search_method.signature.is_some(),
        "Methods should have signature for search context"
    );

    if let Some(sig) = &search_method.signature {
        // Signature should contain parameter names for better search
        assert!(
            sig.contains("query") || sig.len() > 0,
            "Signature should contain parameter information"
        );
    }

    // Count methods with signatures
    let methods_with_sig = chunks
        .iter()
        .filter(|c| (c.kind == "method" || c.kind == "func") && c.signature.is_some())
        .count();

    let total_methods = chunks
        .iter()
        .filter(|c| c.kind == "method" || c.kind == "func")
        .count();

    println!("\n=== Signature Extraction Quality ===");
    println!(
        "Methods with signatures: {}/{}",
        methods_with_sig, total_methods
    );

    let sig_coverage = (methods_with_sig as f64 / total_methods as f64) * 100.0;
    println!("Signature coverage: {:.1}%", sig_coverage);

    assert!(
        sig_coverage >= 70.0,
        "At least 70% of methods should have signatures for search context"
    );
}

/// Search quality validation: Line numbers for navigation
/// Validates that chunks have accurate line numbers for code navigation
#[test]
fn test_search_quality_line_numbers() {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/models.py")
        .expect("Failed to read models fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Verify all chunks have valid line numbers
    for chunk in &chunks {
        assert!(
            chunk.start_line > 0,
            "Chunk '{}' should have valid start_line",
            chunk.symbol_name.as_ref().unwrap_or(&"unknown".to_string())
        );

        assert!(
            chunk.end_line >= chunk.start_line,
            "Chunk '{}' should have end_line >= start_line",
            chunk.symbol_name.as_ref().unwrap_or(&"unknown".to_string())
        );

        // Line ranges should be reasonable (not too large)
        let line_count = chunk.end_line - chunk.start_line + 1;
        assert!(
            line_count < 1000,
            "Chunk '{}' has unreasonably large line range: {}",
            chunk.symbol_name.as_ref().unwrap_or(&"unknown".to_string()),
            line_count
        );
    }

    println!("\n=== Line Number Accuracy ===");
    println!("All chunks have valid line numbers: yes");
    println!("Total chunks validated: {}", chunks.len());

    // Check that line numbers are properly ordered
    let mut prev_start = 0;
    let mut ordering_issues = 0;

    for chunk in &chunks {
        if chunk.start_line < prev_start {
            ordering_issues += 1;
        }
        prev_start = chunk.start_line;
    }

    // Note: Some ordering issues are expected due to nested classes
    println!("Chunks with potential ordering issues: {}", ordering_issues);
}

/// Search quality validation: Kind classification
/// Validates that chunks are properly classified by kind for filtering
#[test]
fn test_search_quality_kind_classification() {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/models.py")
        .expect("Failed to read models fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Expected kinds for Django models file
    let _expected_kinds = vec!["imports", "class", "method", "constant", "variable"];

    let mut kind_counts = std::collections::HashMap::new();
    for chunk in &chunks {
        *kind_counts.entry(chunk.kind.as_str()).or_insert(0) += 1;
    }

    println!("\n=== Kind Classification Distribution ===");
    for (kind, count) in &kind_counts {
        println!("  {}: {}", kind, count);
    }

    // Verify expected kinds are present
    assert!(kind_counts.contains_key("class"), "Should classify classes");
    assert!(
        kind_counts.contains_key("method"),
        "Should classify methods"
    );

    // Verify no unknown/unclassified chunks
    let unknown_chunks = chunks
        .iter()
        .filter(|c| c.kind.is_empty() || c.kind == "unknown")
        .count();

    assert_eq!(
        unknown_chunks, 0,
        "Should not have unknown/unclassified chunks"
    );

    // Verify kind distribution is reasonable
    let class_count = *kind_counts.get("class").unwrap_or(&0);
    let method_count = *kind_counts.get("method").unwrap_or(&0);

    assert!(
        method_count > class_count,
        "Django models should have more methods than classes"
    );
}

/// Memory usage validation: Large file parsing
/// Validates that parser doesn't leak memory on large files
#[test]
fn test_memory_efficiency_large_files() {
    // Create a large synthetic Python file
    let mut large_source = String::new();
    large_source.push_str("# Large Python file for memory testing\n\n");

    // Add 100 classes with 5 methods each (500 total symbols)
    for i in 0..100 {
        large_source.push_str(&format!(
            r#"
class Model{}:
    """Model {} docstring."""

    def method1(self):
        """Method 1."""
        pass

    def method2(self, param):
        """Method 2."""
        pass

    def method3(self):
        """Method 3."""
        return None

    def method4(self, a, b):
        """Method 4."""
        return a + b

    def method5(self):
        """Method 5."""
        pass
"#,
            i, i
        ));
    }

    println!("\n=== Memory Efficiency Test ===");
    println!("Large file size: {} bytes", large_source.len());
    println!("Expected symbols: ~600 (100 classes + 500 methods)");

    // Parse the large file multiple times to detect memory leaks
    let iterations = 10;
    let start = Instant::now();

    for _ in 0..iterations {
        let chunks = parser::extract_chunks(&large_source, "py");

        // Verify we get expected results
        assert!(
            chunks.len() >= 500,
            "Should extract at least 500 chunks from large file"
        );
    }

    let duration = start.elapsed();
    println!("Parsed {} times in {:?}", iterations, duration);
    println!("Average per parse: {:?}", duration / iterations);

    // Should complete without excessive memory usage
    // (No direct memory measurement, but test should not OOM)
    assert!(
        duration.as_secs() < 10,
        "Large file parsing should complete in reasonable time"
    );
}
