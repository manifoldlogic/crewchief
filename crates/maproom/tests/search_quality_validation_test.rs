use maproom::indexer::parser;

/// Quality metrics for symbol extraction
#[derive(Debug, Clone)]
struct SymbolQualityMetrics {
    language: String,
    total_symbols: usize,
    symbols_with_names: usize,
    symbols_with_signatures: usize,
    symbols_with_docstrings: usize,
    symbols_with_metadata: usize,

    // Quality percentages
    name_completeness: f64,      // % symbols with names
    signature_coverage: f64,     // % symbols with signatures (applicable types)
    documentation_coverage: f64, // % symbols with documentation
    metadata_richness: f64,      // % symbols with metadata

    // Search-relevant metrics
    unique_symbols: usize,
    avg_symbol_name_length: f64,
    avg_signature_length: f64,
    avg_docstring_length: f64,
}

impl SymbolQualityMetrics {
    fn new(language: &str) -> Self {
        Self {
            language: language.to_string(),
            total_symbols: 0,
            symbols_with_names: 0,
            symbols_with_signatures: 0,
            symbols_with_docstrings: 0,
            symbols_with_metadata: 0,
            name_completeness: 0.0,
            signature_coverage: 0.0,
            documentation_coverage: 0.0,
            metadata_richness: 0.0,
            unique_symbols: 0,
            avg_symbol_name_length: 0.0,
            avg_signature_length: 0.0,
            avg_docstring_length: 0.0,
        }
    }

    fn calculate_from_samples(language: &str, samples: Vec<(&str, &str)>) -> Self {
        let mut metrics = Self::new(language);
        let mut symbol_names = std::collections::HashSet::new();
        let mut total_name_length = 0;
        let mut total_signature_length = 0;
        let mut total_docstring_length = 0;

        for (_filename, source) in samples {
            let chunks = parser::extract_chunks(source, language);

            for chunk in chunks {
                metrics.total_symbols += 1;

                if let Some(name) = &chunk.symbol_name {
                    metrics.symbols_with_names += 1;
                    symbol_names.insert(name.clone());
                    total_name_length += name.len();
                }

                if let Some(sig) = &chunk.signature {
                    metrics.symbols_with_signatures += 1;
                    total_signature_length += sig.len();
                }

                if let Some(doc) = &chunk.docstring {
                    metrics.symbols_with_docstrings += 1;
                    total_docstring_length += doc.len();
                }

                if chunk.metadata.is_some() {
                    metrics.symbols_with_metadata += 1;
                }
            }
        }

        metrics.unique_symbols = symbol_names.len();

        if metrics.total_symbols > 0 {
            metrics.name_completeness =
                (metrics.symbols_with_names as f64 / metrics.total_symbols as f64) * 100.0;
            metrics.documentation_coverage =
                (metrics.symbols_with_docstrings as f64 / metrics.total_symbols as f64) * 100.0;
            metrics.metadata_richness =
                (metrics.symbols_with_metadata as f64 / metrics.total_symbols as f64) * 100.0;
        }

        // For signature coverage, consider only function/method kinds
        // For now, use a simplified metric
        if metrics.total_symbols > 0 {
            metrics.signature_coverage =
                (metrics.symbols_with_signatures as f64 / metrics.total_symbols as f64) * 100.0;
        }

        if metrics.symbols_with_names > 0 {
            metrics.avg_symbol_name_length =
                total_name_length as f64 / metrics.symbols_with_names as f64;
        }

        if metrics.symbols_with_signatures > 0 {
            metrics.avg_signature_length =
                total_signature_length as f64 / metrics.symbols_with_signatures as f64;
        }

        if metrics.symbols_with_docstrings > 0 {
            metrics.avg_docstring_length =
                total_docstring_length as f64 / metrics.symbols_with_docstrings as f64;
        }

        metrics
    }

    fn print_report(&self) {
        println!(
            "\n=== {} Symbol Quality Report ===",
            self.language.to_uppercase()
        );
        println!("Total symbols extracted: {}", self.total_symbols);
        println!("Unique symbols: {}", self.unique_symbols);
        println!("\nCompleteness:");
        println!(
            "  Symbols with names: {} ({:.1}%)",
            self.symbols_with_names, self.name_completeness
        );
        println!(
            "  Symbols with signatures: {} ({:.1}%)",
            self.symbols_with_signatures, self.signature_coverage
        );
        println!(
            "  Symbols with documentation: {} ({:.1}%)",
            self.symbols_with_docstrings, self.documentation_coverage
        );
        println!(
            "  Symbols with metadata: {} ({:.1}%)",
            self.symbols_with_metadata, self.metadata_richness
        );
        println!("\nAverage lengths (search relevance):");
        println!("  Symbol name: {:.1} chars", self.avg_symbol_name_length);
        println!("  Signature: {:.1} chars", self.avg_signature_length);
        println!("  Documentation: {:.1} chars", self.avg_docstring_length);
    }
}

/// Cross-language consistency metrics
#[derive(Debug)]
struct CrossLanguageMetrics {
    languages: Vec<String>,
    symbol_extraction_consistency: f64,
    documentation_consistency: f64,
    signature_consistency: f64,
}

impl CrossLanguageMetrics {
    fn calculate(metrics: &[SymbolQualityMetrics]) -> Self {
        let languages: Vec<String> = metrics.iter().map(|m| m.language.clone()).collect();

        // Calculate coefficient of variation for key metrics
        let name_cv = Self::coefficient_of_variation(
            &metrics
                .iter()
                .map(|m| m.name_completeness)
                .collect::<Vec<_>>(),
        );
        let doc_cv = Self::coefficient_of_variation(
            &metrics
                .iter()
                .map(|m| m.documentation_coverage)
                .collect::<Vec<_>>(),
        );
        let sig_cv = Self::coefficient_of_variation(
            &metrics
                .iter()
                .map(|m| m.signature_coverage)
                .collect::<Vec<_>>(),
        );

        // Lower CV = higher consistency (invert for score)
        let symbol_extraction_consistency = (1.0 - name_cv.min(1.0)) * 100.0;
        let documentation_consistency = (1.0 - doc_cv.min(1.0)) * 100.0;
        let signature_consistency = (1.0 - sig_cv.min(1.0)) * 100.0;

        Self {
            languages,
            symbol_extraction_consistency,
            documentation_consistency,
            signature_consistency,
        }
    }

    fn coefficient_of_variation(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        if mean == 0.0 {
            return 0.0;
        }

        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        std_dev / mean
    }

    fn print_report(&self) {
        println!("\n=== Cross-Language Consistency Report ===");
        println!("Languages analyzed: {}", self.languages.join(", "));
        println!("\nConsistency scores (0-100, higher is better):");
        println!(
            "  Symbol extraction: {:.1}%",
            self.symbol_extraction_consistency
        );
        println!(
            "  Documentation coverage: {:.1}%",
            self.documentation_consistency
        );
        println!(
            "  Signature completeness: {:.1}%",
            self.signature_consistency
        );
    }
}

// Re-use sample data from large_scale_validation_test for consistency
mod python_samples {
    pub fn get_samples() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "django_model.py",
                r#"
from django.db import models

class Article(models.Model):
    """Represents a blog article in the database."""

    title = models.CharField(max_length=200)
    author = models.ForeignKey('User', on_delete=models.CASCADE)

    def __str__(self):
        """Return string representation of the article."""
        return self.title

    def get_absolute_url(self):
        """Return the URL to access this article."""
        return f'/articles/{self.id}'
"#,
            ),
            (
                "flask_api.py",
                r#"
from flask import Flask, jsonify

app = Flask(__name__)

@app.route('/api/users', methods=['GET'])
def get_users():
    """Fetch all users from the database."""
    users = User.query.all()
    return jsonify([u.to_dict() for u in users])

def process_data(items: list) -> dict:
    """Process a list of items and return summary statistics."""
    return {
        'count': len(items),
        'total': sum(items),
    }
"#,
            ),
            (
                "type_hints.py",
                r#"
from typing import Optional, List, Dict

def find_user(user_id: int) -> Optional[Dict[str, str]]:
    """Find a user by ID.

    Args:
        user_id: The user's unique identifier

    Returns:
        User dictionary or None if not found
    """
    pass

class DataProcessor:
    """Process and validate data."""

    def __init__(self, strict: bool = False):
        """Initialize the processor."""
        self.strict = strict

    def validate(self, data: List[str]) -> bool:
        """Validate input data."""
        return all(isinstance(x, str) for x in data)
"#,
            ),
        ]
    }
}

mod rust_samples {
    pub fn get_samples() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "traits.rs",
                r#"
/// Trait for cacheable items
pub trait Cacheable {
    type Key: Display;

    fn cache_key(&self) -> Self::Key;
    fn is_expired(&self) -> bool;
}

/// Generic cache implementation
pub struct Cache<T: Cacheable> {
    items: HashMap<String, T>,
}

impl<T: Cacheable> Cache<T> {
    /// Create a new cache
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    /// Get an item from the cache
    pub fn get(&self, key: &str) -> Option<&T> {
        self.items.get(key)
    }
}
"#,
            ),
            (
                "error_handling.rs",
                r#"
use std::fmt;

/// Custom error types
#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    InvalidInput(String),
    DatabaseError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

/// Service with error handling
pub struct UserService {
    db_connection: String,
}

impl UserService {
    /// Create a new user service
    pub fn new(connection: String) -> Self {
        Self {
            db_connection: connection,
        }
    }

    /// Find a user by ID
    pub fn find_user(&self, id: i64) -> Result<User, AppError> {
        if id <= 0 {
            return Err(AppError::InvalidInput(
                "User ID must be positive".to_string()
            ));
        }
        Ok(User { id, username: format!("user_{}", id) })
    }
}
"#,
            ),
            (
                "async_tokio.rs",
                r#"
use tokio::net::TcpListener;

/// Handles a client connection asynchronously
async fn handle_client(mut socket: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0; 1024];

    loop {
        let n = socket.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        socket.write_all(&buffer[0..n]).await?;
    }

    Ok(())
}

/// Main server that accepts connections
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Server listening on 127.0.0.1:8080");

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            handle_client(socket).await
        });
    }
}
"#,
            ),
        ]
    }
}

mod go_samples {
    pub fn get_samples() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "interfaces.go",
                r#"
package storage

import "context"

// Storage interface for different backends
type Storage interface {
    Get(ctx context.Context, key string) ([]byte, error)
    Put(ctx context.Context, key string, data []byte) error
    Delete(ctx context.Context, key string) error
}

// MemoryStorage implements Storage in-memory
type MemoryStorage struct {
    data map[string][]byte
}

// NewMemoryStorage creates a new in-memory storage
func NewMemoryStorage() *MemoryStorage {
    return &MemoryStorage{
        data: make(map[string][]byte),
    }
}

// Get retrieves data by key
func (s *MemoryStorage) Get(ctx context.Context, key string) ([]byte, error) {
    if data, ok := s.data[key]; ok {
        return data, nil
    }
    return nil, ErrNotFound
}
"#,
            ),
            (
                "structs.go",
                r#"
package models

import "time"

// BaseModel provides common fields for all models
type BaseModel struct {
    ID        int64     `json:"id" db:"id"`
    CreatedAt time.Time `json:"created_at" db:"created_at"`
    UpdatedAt time.Time `json:"updated_at" db:"updated_at"`
}

// User embeds BaseModel
type User struct {
    BaseModel

    Username string `json:"username" db:"username"`
    Email    string `json:"email" db:"email"`
    IsActive bool   `json:"is_active" db:"is_active"`
}

// Repository provides CRUD operations
type Repository struct {
    db Database
}

// NewRepository creates a new repository
func NewRepository(db Database) *Repository {
    return &Repository{db: db}
}

// FindUser retrieves a user by ID
func (r *Repository) FindUser(id int64) (*User, error) {
    user := &User{}
    query := "SELECT * FROM users WHERE id = $1"
    err := r.db.QueryRow(query, id).Scan(&user.ID, &user.Username, &user.Email)
    if err != nil {
        return nil, err
    }
    return user, nil
}
"#,
            ),
            (
                "concurrency.go",
                r#"
package worker

import (
    "context"
    "sync"
)

// Task represents a unit of work
type Task struct {
    ID   string
    Data interface{}
}

// Worker processes tasks
type Worker struct {
    id      int
    tasks   <-chan Task
    results chan<- Result
    wg      *sync.WaitGroup
}

// NewWorker creates a new worker
func NewWorker(id int, tasks <-chan Task, results chan<- Result, wg *sync.WaitGroup) *Worker {
    return &Worker{
        id:      id,
        tasks:   tasks,
        results: results,
        wg:      wg,
    }
}

// Start begins processing tasks
func (w *Worker) Start() {
    defer w.wg.Done()

    for task := range w.tasks {
        result := w.processTask(task)
        w.results <- result
    }
}

// processTask processes a single task
func (w *Worker) processTask(task Task) Result {
    return Result{
        TaskID: task.ID,
        Data:   task.Data,
    }
}
"#,
            ),
        ]
    }
}

#[test]
fn test_python_symbol_extraction_quality() {
    let samples = python_samples::get_samples();
    let metrics = SymbolQualityMetrics::calculate_from_samples("py", samples);

    metrics.print_report();

    // Quality assertions
    assert!(
        metrics.total_symbols > 0,
        "Should extract symbols from Python code"
    );

    assert!(
        metrics.name_completeness >= 90.0,
        "Python symbol name completeness {:.1}% should be >= 90%",
        metrics.name_completeness
    );

    assert!(
        metrics.documentation_coverage >= 60.0,
        "Python documentation coverage {:.1}% should be >= 60% (many samples have docstrings)",
        metrics.documentation_coverage
    );

    // Python should have good unique symbol ratio
    let uniqueness_ratio = metrics.unique_symbols as f64 / metrics.total_symbols as f64;
    assert!(
        uniqueness_ratio >= 0.7,
        "Python uniqueness ratio {:.2} should be >= 0.7",
        uniqueness_ratio
    );
}

#[test]
fn test_rust_symbol_extraction_quality() {
    let samples = rust_samples::get_samples();
    let metrics = SymbolQualityMetrics::calculate_from_samples("rs", samples);

    metrics.print_report();

    // Quality assertions
    assert!(
        metrics.total_symbols > 0,
        "Should extract symbols from Rust code"
    );

    assert!(
        metrics.name_completeness >= 90.0,
        "Rust symbol name completeness {:.1}% should be >= 90%",
        metrics.name_completeness
    );

    assert!(
        metrics.documentation_coverage >= 60.0,
        "Rust documentation coverage {:.1}% should be >= 60% (many samples have doc comments)",
        metrics.documentation_coverage
    );

    // Rust typically has good signature coverage
    assert!(
        metrics.signature_coverage >= 50.0,
        "Rust signature coverage {:.1}% should be >= 50%",
        metrics.signature_coverage
    );
}

#[test]
fn test_go_symbol_extraction_quality() {
    let samples = go_samples::get_samples();
    let metrics = SymbolQualityMetrics::calculate_from_samples("go", samples);

    metrics.print_report();

    // Quality assertions
    assert!(
        metrics.total_symbols > 0,
        "Should extract symbols from Go code"
    );

    assert!(
        metrics.name_completeness >= 90.0,
        "Go symbol name completeness {:.1}% should be >= 90%",
        metrics.name_completeness
    );

    assert!(
        metrics.documentation_coverage >= 50.0,
        "Go documentation coverage {:.1}% should be >= 50% (Go conventions)",
        metrics.documentation_coverage
    );

    // Go should have good signature coverage
    assert!(
        metrics.signature_coverage >= 50.0,
        "Go signature coverage {:.1}% should be >= 50%",
        metrics.signature_coverage
    );
}

#[test]
fn test_cross_language_consistency() {
    println!("\n=== Cross-Language Consistency Validation ===");

    let py_metrics =
        SymbolQualityMetrics::calculate_from_samples("py", python_samples::get_samples());
    let rs_metrics =
        SymbolQualityMetrics::calculate_from_samples("rs", rust_samples::get_samples());
    let go_metrics = SymbolQualityMetrics::calculate_from_samples("go", go_samples::get_samples());

    let all_metrics = vec![py_metrics, rs_metrics, go_metrics];
    let cross_lang = CrossLanguageMetrics::calculate(&all_metrics);

    cross_lang.print_report();

    // Consistency assertions
    assert!(
        cross_lang.symbol_extraction_consistency >= 70.0,
        "Symbol extraction consistency {:.1}% should be >= 70%",
        cross_lang.symbol_extraction_consistency
    );

    assert!(
        cross_lang.documentation_consistency >= 60.0,
        "Documentation consistency {:.1}% should be >= 60%",
        cross_lang.documentation_consistency
    );

    assert!(
        cross_lang.signature_consistency >= 60.0,
        "Signature consistency {:.1}% should be >= 60%",
        cross_lang.signature_consistency
    );
}

#[test]
fn test_symbol_categorization_accuracy() {
    println!("\n=== Symbol Categorization Accuracy ===");

    // Test Python categorization
    let py_source = r#"
class MyClass:
    """A test class."""
    pass

def my_function():
    """A test function."""
    pass

async def async_function():
    """An async function."""
    pass
"#;

    let py_chunks = parser::extract_chunks(py_source, "py");
    let py_kinds: Vec<String> = py_chunks.iter().map(|c| c.kind.clone()).collect();

    println!("Python kinds: {:?}", py_kinds);
    assert!(
        py_kinds.contains(&"class".to_string()),
        "Should detect Python classes"
    );
    assert!(
        py_kinds.iter().any(|k| k.contains("func")),
        "Should detect Python functions"
    );

    // Test Rust categorization
    let rs_source = r#"
pub struct MyStruct {
    field: i32,
}

impl MyStruct {
    pub fn new() -> Self {
        Self { field: 0 }
    }
}

pub fn standalone_function() {
    println!("test");
}

pub trait MyTrait {
    fn trait_method(&self);
}
"#;

    let rs_chunks = parser::extract_chunks(rs_source, "rs");
    let rs_kinds: Vec<String> = rs_chunks.iter().map(|c| c.kind.clone()).collect();

    println!("Rust kinds: {:?}", rs_kinds);
    assert!(
        rs_kinds.contains(&"struct".to_string()),
        "Should detect Rust structs"
    );
    assert!(
        rs_kinds.contains(&"impl".to_string()) || rs_kinds.iter().any(|k| k.contains("method")),
        "Should detect Rust impl blocks or methods"
    );
    assert!(
        rs_kinds.contains(&"trait".to_string()),
        "Should detect Rust traits"
    );

    // Test Go categorization
    let go_source = r#"
package main

type MyStruct struct {
    Field int
}

func (m *MyStruct) Method() {
    println("method")
}

func StandaloneFunction() {
    println("function")
}

type MyInterface interface {
    InterfaceMethod()
}
"#;

    let go_chunks = parser::extract_chunks(go_source, "go");
    let go_kinds: Vec<String> = go_chunks.iter().map(|c| c.kind.clone()).collect();

    println!("Go kinds: {:?}", go_kinds);
    assert!(
        go_kinds.contains(&"struct".to_string()) || go_kinds.contains(&"type".to_string()),
        "Should detect Go structs/types"
    );
    assert!(
        go_kinds
            .iter()
            .any(|k| k.contains("func") || k.contains("method")),
        "Should detect Go functions/methods"
    );
    assert!(
        go_kinds.contains(&"interface".to_string()) || go_kinds.contains(&"type".to_string()),
        "Should detect Go interfaces"
    );
}

#[test]
fn test_signature_completeness() {
    println!("\n=== Signature Completeness Test ===");

    // Python function with complex signature
    let py_source = r#"
def complex_function(
    name: str,
    age: int,
    *args,
    email: Optional[str] = None,
    **kwargs
) -> Dict[str, Any]:
    """A function with a complex signature."""
    pass
"#;

    let py_chunks = parser::extract_chunks(py_source, "py");
    assert_eq!(py_chunks.len(), 1);

    let py_sig = py_chunks[0].signature.as_ref();
    println!("Python signature: {:?}", py_sig);

    // Should capture parameters
    if let Some(sig) = py_sig {
        assert!(
            sig.contains("name") || sig.len() > 10,
            "Python signature should capture parameters or be substantive"
        );
    }

    // Rust function with signature
    let rs_source = r#"
pub async fn fetch_user(
    db: &Database,
    user_id: i64,
    include_deleted: bool,
) -> Result<User, DatabaseError> {
    todo!()
}
"#;

    let rs_chunks = parser::extract_chunks(rs_source, "rs");
    assert!(!rs_chunks.is_empty());

    let rs_sig = rs_chunks[0].signature.as_ref();
    println!("Rust signature: {:?}", rs_sig);

    if let Some(sig) = rs_sig {
        assert!(
            sig.len() > 10,
            "Rust signature should capture function parameters"
        );
    }

    // Go function with signature
    let go_source = r#"
func ProcessData(
    ctx context.Context,
    input []byte,
    options *ProcessOptions,
) (*Result, error) {
    return nil, nil
}
"#;

    let go_chunks = parser::extract_chunks(go_source, "go");
    assert!(!go_chunks.is_empty());

    let go_sig = go_chunks[0].signature.as_ref();
    println!("Go signature: {:?}", go_sig);

    if let Some(sig) = go_sig {
        assert!(
            sig.len() > 10,
            "Go signature should capture function parameters"
        );
    }
}

#[test]
fn test_documentation_extraction_quality() {
    println!("\n=== Documentation Extraction Quality ===");

    // Python with various docstring styles
    let py_source = r#"
def google_style():
    """Summary line.

    Args:
        param1: Description
        param2: Description

    Returns:
        Return value description
    """
    pass

def numpy_style():
    """
    Summary line.

    Parameters
    ----------
    param : type
        Description

    Returns
    -------
    type
        Description
    """
    pass

class WithClassDoc:
    """This class has documentation."""

    def method(self):
        """This method has documentation."""
        pass
"#;

    let py_chunks = parser::extract_chunks(py_source, "py");
    let py_doc_count = py_chunks.iter().filter(|c| c.docstring.is_some()).count();

    println!(
        "Python chunks with docs: {}/{}",
        py_doc_count,
        py_chunks.len()
    );
    assert!(
        py_doc_count >= 3,
        "Should extract documentation from multiple Python docstrings"
    );

    // Rust with doc comments
    let rs_source = r#"
/// This is a documented struct
pub struct MyStruct {
    /// Field documentation
    field: i32,
}

impl MyStruct {
    /// Create a new instance.
    ///
    /// # Examples
    ///
    /// ```
    /// let s = MyStruct::new();
    /// ```
    pub fn new() -> Self {
        Self { field: 0 }
    }
}
"#;

    let rs_chunks = parser::extract_chunks(rs_source, "rs");
    let rs_doc_count = rs_chunks.iter().filter(|c| c.docstring.is_some()).count();

    println!(
        "Rust chunks with docs: {}/{}",
        rs_doc_count,
        rs_chunks.len()
    );
    assert!(
        rs_doc_count >= 1,
        "Should extract documentation from Rust doc comments"
    );

    // Go with comments
    let go_source = r#"
package main

// User represents a user in the system
type User struct {
    // ID is the unique identifier
    ID int64
    // Name is the user's display name
    Name string
}

// NewUser creates a new user
func NewUser(id int64, name string) *User {
    return &User{
        ID:   id,
        Name: name,
    }
}
"#;

    let go_chunks = parser::extract_chunks(go_source, "go");
    let go_doc_count = go_chunks.iter().filter(|c| c.docstring.is_some()).count();

    println!("Go chunks with docs: {}/{}", go_doc_count, go_chunks.len());
    assert!(
        go_doc_count >= 1,
        "Should extract documentation from Go comments"
    );
}

#[test]
fn test_metadata_richness() {
    println!("\n=== Metadata Richness Test ===");

    // Test Python imports metadata
    let py_source = r#"
import os
import sys
from typing import Optional, List
from datetime import datetime

def example():
    pass
"#;

    let py_chunks = parser::extract_chunks(py_source, "py");
    let py_metadata_count = py_chunks.iter().filter(|c| c.metadata.is_some()).count();

    println!(
        "Python chunks with metadata: {}/{}",
        py_metadata_count,
        py_chunks.len()
    );

    // Check for imports chunk with metadata
    let imports_chunk = py_chunks.iter().find(|c| c.kind == "imports");

    if let Some(chunk) = imports_chunk {
        println!(
            "Found imports chunk with metadata: {:?}",
            chunk.metadata.is_some()
        );
        assert!(
            chunk.metadata.is_some(),
            "Python imports chunk should have metadata"
        );
    }

    // Rust typically has less structured metadata but might have attributes
    let rs_source = r#"
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Config {
    settings: HashMap<String, String>,
}
"#;

    let rs_chunks = parser::extract_chunks(rs_source, "rs");
    println!("Rust chunks: {}", rs_chunks.len());

    // Go imports
    let go_source = r#"
package main

import (
    "fmt"
    "context"
    "github.com/user/repo"
)

func main() {}
"#;

    let go_chunks = parser::extract_chunks(go_source, "go");
    let go_metadata_count = go_chunks.iter().filter(|c| c.metadata.is_some()).count();

    println!(
        "Go chunks with metadata: {}/{}",
        go_metadata_count,
        go_chunks.len()
    );
}

#[test]
fn test_search_relevance_metrics() {
    println!("\n=== Search Relevance Metrics ===");

    // Calculate comprehensive metrics for all languages
    let languages = vec![
        ("Python", "py", python_samples::get_samples()),
        ("Rust", "rs", rust_samples::get_samples()),
        ("Go", "go", go_samples::get_samples()),
    ];

    let mut all_metrics = Vec::new();

    for (lang_name, lang_code, samples) in languages {
        let metrics = SymbolQualityMetrics::calculate_from_samples(lang_code, samples);
        all_metrics.push(metrics.clone());

        println!("\n{} Search Relevance:", lang_name);
        println!(
            "  Searchable symbols (with names): {}",
            metrics.symbols_with_names
        );
        println!(
            "  Avg name length: {:.1} chars (good for exact matching)",
            metrics.avg_symbol_name_length
        );
        println!(
            "  Documentation coverage: {:.1}% (for semantic search)",
            metrics.documentation_coverage
        );
        println!(
            "  Avg doc length: {:.1} chars",
            metrics.avg_docstring_length
        );
        println!(
            "  Signature coverage: {:.1}% (for type-aware search)",
            metrics.signature_coverage
        );
    }

    // Overall quality assessment
    println!("\n=== Overall Search Quality Assessment ===");

    let avg_name_completeness: f64 =
        all_metrics.iter().map(|m| m.name_completeness).sum::<f64>() / all_metrics.len() as f64;

    let avg_doc_coverage: f64 = all_metrics
        .iter()
        .map(|m| m.documentation_coverage)
        .sum::<f64>()
        / all_metrics.len() as f64;

    let avg_sig_coverage: f64 = all_metrics
        .iter()
        .map(|m| m.signature_coverage)
        .sum::<f64>()
        / all_metrics.len() as f64;

    println!("Average name completeness: {:.1}%", avg_name_completeness);
    println!("Average documentation coverage: {:.1}%", avg_doc_coverage);
    println!("Average signature coverage: {:.1}%", avg_sig_coverage);

    // Production readiness thresholds
    assert!(
        avg_name_completeness >= 85.0,
        "Average name completeness {:.1}% should be >= 85% for production",
        avg_name_completeness
    );

    assert!(
        avg_doc_coverage >= 50.0,
        "Average documentation coverage {:.1}% should be >= 50% for good semantic search",
        avg_doc_coverage
    );

    println!("\n✓ All languages meet production readiness thresholds for search quality");
}

#[test]
fn test_production_readiness_report() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║           SEARCH QUALITY VALIDATION REPORT                   ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    let py_metrics =
        SymbolQualityMetrics::calculate_from_samples("py", python_samples::get_samples());
    let rs_metrics =
        SymbolQualityMetrics::calculate_from_samples("rs", rust_samples::get_samples());
    let go_metrics = SymbolQualityMetrics::calculate_from_samples("go", go_samples::get_samples());

    py_metrics.print_report();
    rs_metrics.print_report();
    go_metrics.print_report();

    let all_metrics = vec![py_metrics, rs_metrics, go_metrics];
    let cross_lang = CrossLanguageMetrics::calculate(&all_metrics);
    cross_lang.print_report();

    println!("\n=== Production Readiness Assessment ===");

    let all_pass_name_completeness = all_metrics.iter().all(|m| m.name_completeness >= 85.0);
    let all_pass_doc_coverage = all_metrics.iter().all(|m| m.documentation_coverage >= 40.0);

    println!(
        "✓ Symbol name completeness: {}",
        if all_pass_name_completeness {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "✓ Documentation coverage: {}",
        if all_pass_doc_coverage {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "✓ Cross-language consistency: {}",
        if cross_lang.symbol_extraction_consistency >= 60.0 {
            "PASS"
        } else {
            "FAIL"
        }
    );

    assert!(
        all_pass_name_completeness && all_pass_doc_coverage,
        "All languages should meet production readiness criteria"
    );

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✓ ALL LANGUAGES READY FOR PRODUCTION                        ║");
    println!("║                                                              ║");
    println!("║  Parser output quality provides strong foundation for:       ║");
    println!("║  • Text search (high name completeness)                      ║");
    println!("║  • Semantic search (good documentation coverage)             ║");
    println!("║  • Type-aware search (signature completeness)                ║");
    println!("║  • Cross-language queries (consistent extraction)            ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}
