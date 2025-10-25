use crewchief_maproom::indexer::parser;

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
    let add_fn = chunks.iter().find(|c| c.symbol_name == Some("add".to_string())).unwrap();
    assert_eq!(add_fn.kind, "func");
    assert!(add_fn.signature.as_ref().unwrap().contains("pub"));
    assert!(add_fn.signature.as_ref().unwrap().contains("i32"));
    assert_eq!(add_fn.docstring, Some("This is a doc comment for a function".to_string()));

    // Check metadata
    let metadata = add_fn.metadata.as_ref().unwrap();
    assert_eq!(metadata["visibility"].as_str(), Some("pub"));
    assert_eq!(metadata["is_async"].as_bool(), Some(false));

    // Check second function (private)
    let private_fn = chunks.iter().find(|c| c.symbol_name == Some("private_function".to_string())).unwrap();
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
    let point = chunks.iter().find(|c| c.symbol_name == Some("Point".to_string())).unwrap();
    assert_eq!(point.kind, "struct");
    assert_eq!(point.docstring, Some("A simple struct".to_string()));
    let point_metadata = point.metadata.as_ref().unwrap();
    assert_eq!(point_metadata["visibility"].as_str(), Some("pub"));

    // Check Container struct (with generics)
    let container = chunks.iter().find(|c| c.symbol_name == Some("Container".to_string())).unwrap();
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
    let option = chunks.iter().find(|c| c.symbol_name == Some("Option".to_string())).unwrap();
    assert_eq!(option.kind, "enum");
    assert_eq!(option.docstring, Some("An enum representing options".to_string()));
    assert!(option.signature.is_some());

    // Check Color enum
    let color = chunks.iter().find(|c| c.symbol_name == Some("Color".to_string())).unwrap();
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
    let draw = chunks.iter().find(|c| c.symbol_name == Some("Draw".to_string())).unwrap();
    assert_eq!(draw.kind, "trait");
    assert_eq!(draw.docstring, Some("A trait for things that can be drawn".to_string()));

    // Check Clone trait (with generics)
    let clone_trait = chunks.iter().find(|c| c.symbol_name == Some("Clone".to_string())).unwrap();
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
    let inherent_impl = impl_blocks.iter()
        .find(|c| c.symbol_name.as_ref().map(|s| s.contains("impl Point")).unwrap_or(false))
        .unwrap();
    assert_eq!(inherent_impl.kind, "impl");

    // Check trait impl
    let trait_impl = impl_blocks.iter()
        .find(|c| c.symbol_name.as_ref().map(|s| s.contains("Draw")).unwrap_or(false))
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
    let max_val = chunks.iter().find(|c| c.symbol_name == Some("MAX_VALUE".to_string())).unwrap();
    assert_eq!(max_val.kind, "constant");
    assert_eq!(max_val.docstring, Some("Maximum value".to_string()));
    assert!(max_val.signature.is_some());
    assert!(max_val.signature.as_ref().unwrap().contains("i32"));

    // Check static
    let counter = chunks.iter().find(|c| c.symbol_name == Some("COUNTER".to_string())).unwrap();
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
    let utils = modules.iter()
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
    assert!(chunks.iter().any(|c| c.kind == "struct" && c.symbol_name == Some("Config".to_string())));

    // Verify trait
    assert!(chunks.iter().any(|c| c.kind == "trait" && c.symbol_name == Some("Configurable".to_string())));

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
    assert!(chunks.iter().any(|c| c.kind == "func" && c.symbol_name == Some("main".to_string())));
}
