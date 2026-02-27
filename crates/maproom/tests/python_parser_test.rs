use maproom::indexer::parser;

#[test]
fn test_python_parser_simple_function() {
    let source = r#"
def hello_world():
    """Say hello to the world."""
    print("Hello, world!")
"#;

    let chunks = parser::extract_chunks(source, "py");

    assert_eq!(chunks.len(), 1, "Expected 1 chunk for a simple function");

    let func = &chunks[0];
    assert_eq!(func.symbol_name, Some("hello_world".to_string()));
    assert_eq!(func.kind, "func");
    assert_eq!(func.signature, Some("()".to_string()));
    assert_eq!(func.docstring, Some("Say hello to the world.".to_string()));
    assert_eq!(func.start_line, 2);
    assert_eq!(func.end_line, 4);
}

#[test]
fn test_python_parser_function_with_params() {
    let source = r#"
def greet(name: str, age: int) -> str:
    """Greet a person by name and age."""
    return f"Hello {name}, you are {age} years old"
"#;

    let chunks = parser::extract_chunks(source, "py");

    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    assert_eq!(func.symbol_name, Some("greet".to_string()));
    assert_eq!(func.kind, "func");
    assert!(func.signature.is_some());
    assert!(func.signature.as_ref().unwrap().contains("name"));
    assert!(func.signature.as_ref().unwrap().contains("age"));
    assert_eq!(
        func.docstring,
        Some("Greet a person by name and age.".to_string())
    );
}

#[test]
fn test_python_parser_simple_class() {
    let source = r#"
class Person:
    """A simple person class."""

    def __init__(self, name):
        """Initialize a person."""
        self.name = name

    def greet(self):
        """Say hello."""
        return f"Hello, I'm {self.name}"
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should have: 1 class + 2 methods (including __init__)
    assert_eq!(chunks.len(), 3, "Expected 3 chunks: 1 class + 2 methods");

    // First chunk should be the class
    let class = &chunks[0];
    assert_eq!(class.symbol_name, Some("Person".to_string()));
    assert_eq!(class.kind, "class");
    assert_eq!(class.docstring, Some("A simple person class.".to_string()));

    // Second chunk should be __init__ method
    let init = &chunks[1];
    assert_eq!(init.symbol_name, Some("__init__".to_string()));
    assert_eq!(init.kind, "method");
    assert_eq!(init.docstring, Some("Initialize a person.".to_string()));

    // Third chunk should be greet method
    let greet = &chunks[2];
    assert_eq!(greet.symbol_name, Some("greet".to_string()));
    assert_eq!(greet.kind, "method");
    assert_eq!(greet.docstring, Some("Say hello.".to_string()));
}

#[test]
fn test_python_parser_class_with_inheritance() {
    let source = r#"
class Animal:
    """Base animal class."""
    pass

class Dog(Animal):
    """A dog is an animal."""
    def bark(self):
        return "Woof!"
"#;

    let chunks = parser::extract_chunks(source, "py");

    assert!(chunks.len() >= 2, "Expected at least 2 chunks");

    // Find the Dog class
    let dog = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Dog".to_string()))
        .expect("Should find Dog class");

    assert_eq!(dog.kind, "class");
    assert!(dog.signature.is_some());
    assert!(dog.signature.as_ref().unwrap().contains("Animal"));
    assert_eq!(dog.docstring, Some("A dog is an animal.".to_string()));
}

#[test]
fn test_python_parser_multiple_functions() {
    let source = r#"
def add(a, b):
    """Add two numbers."""
    return a + b

def subtract(a, b):
    """Subtract two numbers."""
    return a - b

def multiply(a, b):
    """Multiply two numbers."""
    return a * b
"#;

    let chunks = parser::extract_chunks(source, "py");

    assert_eq!(chunks.len(), 3, "Expected 3 functions");

    let names: Vec<_> = chunks
        .iter()
        .filter_map(|c| c.symbol_name.as_ref().map(|s| s.as_str()))
        .collect();

    assert!(names.contains(&"add"));
    assert!(names.contains(&"subtract"));
    assert!(names.contains(&"multiply"));
}

#[test]
fn test_python_parser_handles_malformed_syntax() {
    let source = r#"
def broken_function(
    # Missing closing parenthesis
    print("This is broken"
"#;

    // Parser should not panic, even with malformed code
    let _chunks = parser::extract_chunks(source, "py");

    // The result may vary (empty or partial), but should not panic
    // This test primarily ensures error handling works - the fact that we reached here means success
}

#[test]
fn test_python_parser_empty_file() {
    let source = "";

    let chunks = parser::extract_chunks(source, "py");

    // Empty file should return empty chunks
    assert_eq!(chunks.len(), 0);
}

#[test]
fn test_python_parser_comments_only() {
    let source = r#"
# This is a comment
# Another comment
# Yet another comment
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Comments only should return empty chunks
    assert_eq!(chunks.len(), 0);
}

#[test]
fn test_python_parser_imports() {
    let source = r#"
import os
from pathlib import Path

def use_imports():
    """Use the imported modules."""
    return Path(os.getcwd())
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should extract both the imports chunk and the function
    assert_eq!(chunks.len(), 2);

    // Find the function chunk
    let func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("use_imports".to_string()))
        .expect("Should find use_imports function");
    assert_eq!(func.kind, "func");

    // Verify imports chunk exists
    let imports_chunk = chunks
        .iter()
        .find(|c| c.kind == "imports")
        .expect("Should have imports chunk");
    assert!(imports_chunk.metadata.is_some());
}

#[test]
fn test_python_parser_nested_functions() {
    let source = r#"
def outer():
    """Outer function."""
    def inner():
        """Inner function."""
        return "inner"
    return inner()
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should extract both outer and inner functions
    assert_eq!(chunks.len(), 2);

    let names: Vec<_> = chunks
        .iter()
        .filter_map(|c| c.symbol_name.as_ref())
        .map(|s| s.as_str())
        .collect();

    assert!(names.contains(&"outer"));
    assert!(names.contains(&"inner"));
}

#[test]
fn test_python_parser_decorators() {
    let source = r#"
@property
def name(self):
    """Get the name property."""
    return self._name

@name.setter
def name(self, value):
    """Set the name property."""
    self._name = value
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should extract both functions (decorators are part of the function node)
    assert_eq!(chunks.len(), 2);

    for chunk in &chunks {
        assert_eq!(chunk.symbol_name, Some("name".to_string()));
        assert_eq!(chunk.kind, "func");
    }
}

#[test]
fn test_python_parser_async_function() {
    let source = r#"
async def fetch_data():
    """Asynchronously fetch data."""
    return await some_api_call()
"#;

    let chunks = parser::extract_chunks(source, "py");

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].symbol_name, Some("fetch_data".to_string()));
    assert_eq!(chunks[0].kind, "async_func");
    assert_eq!(
        chunks[0].docstring,
        Some("Asynchronously fetch data.".to_string())
    );

    // Verify metadata
    if let Some(metadata) = &chunks[0].metadata {
        let is_async = metadata.get("is_async").unwrap().as_bool().unwrap();
        assert!(is_async);
    }
}
