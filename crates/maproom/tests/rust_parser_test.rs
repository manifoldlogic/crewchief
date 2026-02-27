use maproom::indexer::parser;

#[test]
fn test_rust_function_extraction() {
    let source = r#"
/// This is a doc comment for a function
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn private_function() {
    println!("private");
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract 2 functions
    assert_eq!(chunks.len(), 2);

    // Check first function (public)
    let add_fn = chunks
        .iter()
        .find(|c| c.symbol_name == Some("add".to_string()))
        .unwrap();
    assert_eq!(add_fn.kind, "func");
    assert!(add_fn.signature.as_ref().unwrap().contains("pub"));
    assert!(add_fn.signature.as_ref().unwrap().contains("i32"));
    assert_eq!(
        add_fn.docstring,
        Some("This is a doc comment for a function".to_string())
    );

    // Check metadata
    let metadata = add_fn.metadata.as_ref().unwrap();
    assert_eq!(metadata["visibility"].as_str(), Some("pub"));
    assert_eq!(metadata["is_async"].as_bool(), Some(false));

    // Check second function (private)
    let private_fn = chunks
        .iter()
        .find(|c| c.symbol_name == Some("private_function".to_string()))
        .unwrap();
    assert_eq!(private_fn.kind, "func");
    let private_metadata = private_fn.metadata.as_ref().unwrap();
    assert_eq!(private_metadata["visibility"].as_str(), Some("private"));
}

#[test]
fn test_rust_struct_extraction() {
    let source = r#"
/// A simple struct
pub struct Point {
    x: f64,
    y: f64,
}

/// A generic struct
struct Container<T> {
    value: T,
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract 2 structs
    assert_eq!(chunks.len(), 2);

    // Check Point struct
    let point = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Point".to_string()))
        .unwrap();
    assert_eq!(point.kind, "struct");
    assert_eq!(point.docstring, Some("A simple struct".to_string()));
    let point_metadata = point.metadata.as_ref().unwrap();
    assert_eq!(point_metadata["visibility"].as_str(), Some("pub"));

    // Check Container struct (with generics)
    let container = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Container".to_string()))
        .unwrap();
    assert_eq!(container.kind, "struct");
    assert!(container.signature.is_some());
    assert!(container.signature.as_ref().unwrap().contains("T"));
}

#[test]
fn test_rust_enum_extraction() {
    let source = r#"
/// An enum representing options
pub enum Option<T> {
    Some(T),
    None,
}

enum Color {
    Red,
    Green,
    Blue,
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract 2 enums
    assert_eq!(chunks.len(), 2);

    // Check Option enum
    let option = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Option".to_string()))
        .unwrap();
    assert_eq!(option.kind, "enum");
    assert_eq!(
        option.docstring,
        Some("An enum representing options".to_string())
    );
    assert!(option.signature.is_some());

    // Check Color enum
    let color = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Color".to_string()))
        .unwrap();
    assert_eq!(color.kind, "enum");
}

#[test]
fn test_rust_trait_extraction() {
    let source = r#"
/// A trait for things that can be drawn
pub trait Draw {
    fn draw(&self);
}

/// A generic trait
trait Clone<T> {
    fn clone(&self) -> T;
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract 2 traits
    assert_eq!(chunks.len(), 2);

    // Check Draw trait
    let draw = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Draw".to_string()))
        .unwrap();
    assert_eq!(draw.kind, "trait");
    assert_eq!(
        draw.docstring,
        Some("A trait for things that can be drawn".to_string())
    );

    // Check Clone trait (with generics)
    let clone_trait = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Clone".to_string()))
        .unwrap();
    assert_eq!(clone_trait.kind, "trait");
    assert!(clone_trait.signature.is_some());
}

#[test]
fn test_rust_impl_block_extraction() {
    let source = r#"
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }
}

impl Draw for Point {
    fn draw(&self) {
        println!("Drawing point");
    }
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract struct + 2 impl blocks (including their methods)
    let impl_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "impl").collect();
    assert_eq!(impl_blocks.len(), 2);

    // Check inherent impl
    let inherent_impl = impl_blocks
        .iter()
        .find(|c| {
            c.symbol_name
                .as_ref()
                .map(|s| s.contains("impl Point"))
                .unwrap_or(false)
        })
        .unwrap();
    assert_eq!(inherent_impl.kind, "impl");

    // Check trait impl
    let trait_impl = impl_blocks
        .iter()
        .find(|c| {
            c.symbol_name
                .as_ref()
                .map(|s| s.contains("Draw"))
                .unwrap_or(false)
        })
        .unwrap();
    assert_eq!(trait_impl.kind, "impl");
    assert!(trait_impl.symbol_name.as_ref().unwrap().contains("Draw"));
}

#[test]
fn test_rust_constant_extraction() {
    let source = r#"
/// Maximum value
pub const MAX_VALUE: i32 = 100;

static COUNTER: i32 = 0;
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract 1 const and 1 static
    assert_eq!(chunks.len(), 2);

    // Check const
    let max_val = chunks
        .iter()
        .find(|c| c.symbol_name == Some("MAX_VALUE".to_string()))
        .unwrap();
    assert_eq!(max_val.kind, "constant");
    assert_eq!(max_val.docstring, Some("Maximum value".to_string()));
    assert!(max_val.signature.is_some());
    assert!(max_val.signature.as_ref().unwrap().contains("i32"));

    // Check static
    let counter = chunks
        .iter()
        .find(|c| c.symbol_name == Some("COUNTER".to_string()))
        .unwrap();
    assert_eq!(counter.kind, "static");
}

#[test]
fn test_rust_macro_extraction() {
    let source = r#"
/// A simple macro
macro_rules! say_hello {
    () => {
        println!("Hello!");
    };
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract 1 macro
    assert_eq!(chunks.len(), 1);

    let macro_chunk = &chunks[0];
    assert_eq!(macro_chunk.kind, "macro");
    assert_eq!(macro_chunk.symbol_name, Some("say_hello".to_string()));
    assert_eq!(macro_chunk.signature, Some("macro_rules!".to_string()));
    assert_eq!(macro_chunk.docstring, Some("A simple macro".to_string()));
}

#[test]
fn test_rust_async_function() {
    let source = r#"
pub async fn fetch_data() -> Result<String, Error> {
    Ok("data".to_string())
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    assert_eq!(chunks.len(), 1);
    let async_fn = &chunks[0];
    assert_eq!(async_fn.kind, "func");
    assert!(async_fn.signature.as_ref().unwrap().contains("async"));

    let metadata = async_fn.metadata.as_ref().unwrap();
    assert_eq!(metadata["is_async"].as_bool(), Some(true));
}

#[test]
fn test_rust_unsafe_function() {
    let source = r#"
pub unsafe fn raw_pointer_access(ptr: *const i32) -> i32 {
    *ptr
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    assert_eq!(chunks.len(), 1);
    let unsafe_fn = &chunks[0];
    assert_eq!(unsafe_fn.kind, "func");
    assert!(unsafe_fn.signature.as_ref().unwrap().contains("unsafe"));

    let metadata = unsafe_fn.metadata.as_ref().unwrap();
    assert_eq!(metadata["is_unsafe"].as_bool(), Some(true));
}

#[test]
fn test_rust_module_extraction() {
    let source = r#"
/// Public module
pub mod utils {
    pub fn helper() {}
}

mod private_mod {
    fn internal() {}
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract 2 modules + their functions
    let modules: Vec<_> = chunks.iter().filter(|c| c.kind == "module").collect();
    assert_eq!(modules.len(), 2);

    // Check public module
    let utils = modules
        .iter()
        .find(|c| c.symbol_name == Some("utils".to_string()))
        .unwrap();
    assert_eq!(utils.docstring, Some("Public module".to_string()));
    let utils_metadata = utils.metadata.as_ref().unwrap();
    assert_eq!(utils_metadata["visibility"].as_str(), Some("pub"));
}

#[test]
fn test_rust_complex_file() {
    let source = r#"
//! This is a module-level doc comment

use std::collections::HashMap;

/// Configuration struct
pub struct Config {
    name: String,
    values: HashMap<String, String>,
}

impl Config {
    /// Creates a new Config
    pub fn new(name: String) -> Self {
        Config {
            name,
            values: HashMap::new(),
        }
    }
}

/// A trait for configurable items
pub trait Configurable {
    fn configure(&mut self, config: &Config);
}

impl Configurable for Config {
    fn configure(&mut self, config: &Config) {
        self.name = config.name.clone();
    }
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should have: struct, impl, trait, trait impl (and their nested items)
    assert!(chunks.len() >= 4);

    // Verify struct
    assert!(chunks
        .iter()
        .any(|c| c.kind == "struct" && c.symbol_name == Some("Config".to_string())));

    // Verify trait
    assert!(chunks
        .iter()
        .any(|c| c.kind == "trait" && c.symbol_name == Some("Configurable".to_string())));

    // Verify impl blocks
    let impl_blocks: Vec<_> = chunks.iter().filter(|c| c.kind == "impl").collect();
    assert!(impl_blocks.len() >= 2);
}

#[test]
fn test_rust_malformed_code_doesnt_crash() {
    // Test that malformed code doesn't panic
    let malformed = r#"
pub fn incomplete(
"#;

    let chunks = parser::extract_chunks(malformed, "rs");
    // Should handle gracefully - might return empty or partial results
    // The key is it doesn't crash (this line just ensures the code runs)
    let _ = chunks.len();
}

#[test]
fn test_rust_macro_invocation_handling() {
    // Test that macro invocations don't cause issues
    let source = r#"
fn main() {
    println!("Hello, world!");
    vec![1, 2, 3];
    assert_eq!(1, 1);
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract the main function
    assert!(chunks
        .iter()
        .any(|c| c.kind == "func" && c.symbol_name == Some("main".to_string())));
}

#[test]
fn test_rust_function_with_generics() {
    let source = r#"
/// A generic function with type bounds
pub fn process<T: Clone + Send>(value: T) -> T {
    value.clone()
}

/// Function with multiple type parameters
fn combine<T, U>(a: T, b: U) -> (T, U) {
    (a, b)
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract 2 functions
    assert_eq!(chunks.len(), 2);

    // Check first function with generic bounds
    let process_fn = chunks
        .iter()
        .find(|c| c.symbol_name == Some("process".to_string()))
        .unwrap();
    assert_eq!(process_fn.kind, "func");
    assert!(process_fn
        .signature
        .as_ref()
        .unwrap()
        .contains("<T: Clone + Send>"));

    let metadata = process_fn.metadata.as_ref().unwrap();
    assert_eq!(metadata["generics"].as_str(), Some("<T: Clone + Send>"));

    // Check second function with multiple type parameters
    let combine_fn = chunks
        .iter()
        .find(|c| c.symbol_name == Some("combine".to_string()))
        .unwrap();
    assert_eq!(combine_fn.kind, "func");
    assert!(combine_fn.signature.as_ref().unwrap().contains("<T, U>"));

    let combine_metadata = combine_fn.metadata.as_ref().unwrap();
    assert_eq!(combine_metadata["generics"].as_str(), Some("<T, U>"));
}

#[test]
fn test_rust_function_with_where_clause() {
    let source = r#"
/// Function with where clause
pub fn compare<T>(a: T, b: T) -> bool
where
    T: PartialEq + Clone,
{
    a == b
}

/// Complex where clause
fn process_data<T, U>(data: T, processor: U) -> String
where
    T: Display,
    U: Fn(T) -> String,
{
    processor(data)
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract 2 functions
    assert_eq!(chunks.len(), 2);

    // Check first function with where clause
    let compare_fn = chunks
        .iter()
        .find(|c| c.symbol_name == Some("compare".to_string()))
        .unwrap();
    assert_eq!(compare_fn.kind, "func");

    let metadata = compare_fn.metadata.as_ref().unwrap();
    assert!(metadata.get("where_clause").is_some());
    let where_clause = metadata["where_clause"].as_str().unwrap();
    assert!(where_clause.contains("T: PartialEq + Clone"));

    // Check second function with complex where clause
    let process_fn = chunks
        .iter()
        .find(|c| c.symbol_name == Some("process_data".to_string()))
        .unwrap();
    assert_eq!(process_fn.kind, "func");

    let process_metadata = process_fn.metadata.as_ref().unwrap();
    assert!(process_metadata.get("where_clause").is_some());
    let process_where = process_metadata["where_clause"].as_str().unwrap();
    assert!(process_where.contains("T: Display"));
    assert!(process_where.contains("U: Fn(T) -> String"));
}

#[test]
fn test_rust_struct_with_where_clause() {
    let source = r#"
/// A struct with where clause
pub struct Container<T>
where
    T: Clone + Send,
{
    value: T,
}

/// Generic struct with multiple constraints
struct Pair<T, U>
where
    T: Display,
    U: Clone,
{
    first: T,
    second: U,
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract 2 structs
    assert_eq!(chunks.len(), 2);

    // Check first struct with where clause
    let container = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Container".to_string()))
        .unwrap();
    assert_eq!(container.kind, "struct");

    let metadata = container.metadata.as_ref().unwrap();
    assert_eq!(metadata["generics"].as_str(), Some("<T>"));
    assert!(metadata.get("where_clause").is_some());

    // Check second struct with multiple constraints
    let pair = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Pair".to_string()))
        .unwrap();
    assert_eq!(pair.kind, "struct");

    let pair_metadata = pair.metadata.as_ref().unwrap();
    assert_eq!(pair_metadata["generics"].as_str(), Some("<T, U>"));
    assert!(pair_metadata.get("where_clause").is_some());
}

#[test]
fn test_rust_use_statements() {
    let source = r#"
use std::collections::HashMap;
use std::io::{Read, Write};
pub use super::*;
use crate::config::Config;

fn test_function() {}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract 4 use statements + 1 function
    let use_statements: Vec<_> = chunks.iter().filter(|c| c.kind == "use").collect();
    assert_eq!(use_statements.len(), 4);

    // Check simple use statement
    let hashmap_use = use_statements
        .iter()
        .find(|c| {
            c.symbol_name
                .as_ref()
                .map(|s| s.contains("HashMap"))
                .unwrap_or(false)
        })
        .unwrap();
    assert_eq!(hashmap_use.kind, "use");
    assert!(hashmap_use
        .signature
        .as_ref()
        .unwrap()
        .contains("std::collections::HashMap"));

    // Check use with braces
    let io_use = use_statements
        .iter()
        .find(|c| {
            c.symbol_name
                .as_ref()
                .map(|s| s.contains("{Read, Write}"))
                .unwrap_or(false)
        })
        .unwrap();
    assert_eq!(io_use.kind, "use");

    // Check pub use
    let pub_use = use_statements
        .iter()
        .find(|c| {
            c.symbol_name
                .as_ref()
                .map(|s| s.contains("super::*"))
                .unwrap_or(false)
        })
        .unwrap();
    assert_eq!(pub_use.kind, "use");
    let pub_use_metadata = pub_use.metadata.as_ref().unwrap();
    assert_eq!(pub_use_metadata["visibility"].as_str(), Some("pub"));

    // Check crate use
    let config_use = use_statements
        .iter()
        .find(|c| {
            c.symbol_name
                .as_ref()
                .map(|s| s.contains("config::Config"))
                .unwrap_or(false)
        })
        .unwrap();
    assert_eq!(config_use.kind, "use");
}

#[test]
fn test_rust_enum_with_generics_and_where() {
    let source = r#"
/// A result type with where clause
pub enum Result<T, E>
where
    T: Clone,
    E: std::error::Error,
{
    Ok(T),
    Err(E),
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract 1 enum
    assert_eq!(chunks.len(), 1);

    let result_enum = &chunks[0];
    assert_eq!(result_enum.kind, "enum");
    assert_eq!(result_enum.symbol_name, Some("Result".to_string()));

    let metadata = result_enum.metadata.as_ref().unwrap();
    assert_eq!(metadata["generics"].as_str(), Some("<T, E>"));
    assert!(metadata.get("where_clause").is_some());
}

#[test]
fn test_rust_trait_with_generics() {
    let source = r#"
/// A trait with associated types and where clause
pub trait Iterator<Item>
where
    Item: Clone,
{
    fn next(&mut self) -> Option<Item>;
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract 1 trait
    assert_eq!(chunks.len(), 1);

    let iterator_trait = &chunks[0];
    assert_eq!(iterator_trait.kind, "trait");
    assert_eq!(iterator_trait.symbol_name, Some("Iterator".to_string()));

    let metadata = iterator_trait.metadata.as_ref().unwrap();
    assert_eq!(metadata["generics"].as_str(), Some("<Item>"));
    assert!(metadata.get("where_clause").is_some());
}

#[test]
fn test_rust_comprehensive_extraction() {
    // Test a comprehensive Rust file with all features combined
    let source = r#"
use std::collections::HashMap;
use std::fmt::Display;

/// A comprehensive test structure
pub struct Container<T, U>
where
    T: Clone + Send,
    U: Display,
{
    items: Vec<T>,
    metadata: HashMap<String, U>,
}

impl<T, U> Container<T, U>
where
    T: Clone + Send,
    U: Display,
{
    /// Creates a new container
    pub fn new() -> Self {
        Container {
            items: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Processes items with a generic function
    pub fn process<F>(items: Vec<T>, processor: F) -> Vec<T>
    where
        F: Fn(T) -> T,
        T: Clone,
    {
        items.into_iter().map(processor).collect()
    }
}
"#;

    let chunks = parser::extract_chunks(source, "rs");

    // Should extract: 2 use statements + 1 struct + 1 impl + 2 functions (new + process)
    assert!(chunks.len() >= 5);

    // Verify use statements
    let use_statements: Vec<_> = chunks.iter().filter(|c| c.kind == "use").collect();
    assert_eq!(use_statements.len(), 2);

    // Verify struct with generics and where clause
    let struct_chunk = chunks
        .iter()
        .find(|c| c.kind == "struct" && c.symbol_name == Some("Container".to_string()))
        .unwrap();
    let struct_metadata = struct_chunk.metadata.as_ref().unwrap();
    assert_eq!(struct_metadata["generics"].as_str(), Some("<T, U>"));
    assert!(struct_metadata.get("where_clause").is_some());
    let struct_where = struct_metadata["where_clause"].as_str().unwrap();
    assert!(struct_where.contains("T: Clone + Send"));
    assert!(struct_where.contains("U: Display"));

    // Verify impl block
    let impl_chunks: Vec<_> = chunks.iter().filter(|c| c.kind == "impl").collect();
    assert_eq!(impl_chunks.len(), 1);

    // Verify function with where clause inside impl
    let process_fn = chunks
        .iter()
        .find(|c| c.symbol_name == Some("process".to_string()))
        .unwrap();
    assert_eq!(process_fn.kind, "func");
    let process_metadata = process_fn.metadata.as_ref().unwrap();
    assert_eq!(process_metadata["generics"].as_str(), Some("<F>"));
    assert!(process_metadata.get("where_clause").is_some());

    // Verify new function
    let new_fn = chunks
        .iter()
        .find(|c| c.symbol_name == Some("new".to_string()))
        .unwrap();
    assert_eq!(new_fn.kind, "func");
    assert!(new_fn.signature.as_ref().unwrap().contains("pub"));
}
