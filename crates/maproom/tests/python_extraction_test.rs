use maproom::indexer::parser;
use std::fs;

// Test functions with parameters and return type hints
#[test]
fn test_python_function_with_type_hints() {
    let source = r#"
def calculate_sum(a: int, b: int) -> int:
    """Calculate the sum of two integers."""
    return a + b
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    assert_eq!(func.symbol_name, Some("calculate_sum".to_string()));
    assert_eq!(func.kind, "func");
    assert!(func.signature.is_some());

    let sig = func.signature.as_ref().unwrap();
    assert!(sig.contains("a: int"));
    assert!(sig.contains("b: int"));
    assert!(sig.contains("-> int"));

    assert_eq!(
        func.docstring,
        Some("Calculate the sum of two integers.".to_string())
    );
}

#[test]
fn test_python_function_complex_type_hints() {
    let source = r#"
from typing import List, Optional

def process_items(items: List[str], default: Optional[str] = None) -> str:
    """Process a list of items."""
    return default or ", ".join(items)
"#;

    let chunks = parser::extract_chunks(source, "py");
    // Now we extract imports as a separate chunk, so we should have 2 chunks
    assert_eq!(chunks.len(), 2);

    // Find the function chunk (not the imports chunk)
    let func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("process_items".to_string()))
        .expect("Should find process_items function");
    assert!(func.signature.as_ref().unwrap().contains("List[str]"));
    assert!(func.signature.as_ref().unwrap().contains("Optional[str]"));
}

// Test classes with docstrings
#[test]
fn test_python_class_with_docstring() {
    let source = r#"
class DataProcessor:
    """
    A class for processing data.

    This class handles various data processing tasks.
    """

    def process(self, data):
        """Process the data."""
        return data
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert!(chunks.len() >= 1);

    let class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("DataProcessor".to_string()))
        .expect("Should find DataProcessor class");

    assert_eq!(class.kind, "class");
    assert!(class.docstring.is_some());
    let docstring = class.docstring.as_ref().unwrap();
    assert!(docstring.contains("processing data"));
}

// Test inheritance relationships
#[test]
fn test_python_class_inheritance() {
    let source = r#"
class Animal:
    """Base animal class."""
    pass

class Mammal(Animal):
    """Mammals are warm-blooded animals."""
    pass

class Dog(Mammal):
    """Dogs are loyal mammals."""
    def bark(self):
        return "Woof!"
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Find classes and check inheritance
    let mammal = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Mammal".to_string()))
        .expect("Should find Mammal class");

    assert!(mammal.signature.is_some());
    assert!(mammal.signature.as_ref().unwrap().contains("Animal"));

    // Check metadata for base classes
    if let Some(metadata) = &mammal.metadata {
        if let Some(base_classes) = metadata.get("base_classes") {
            let bases = base_classes.as_array().unwrap();
            assert_eq!(bases.len(), 1);
            assert_eq!(bases[0].as_str().unwrap(), "Animal");
        }
    }

    let dog = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Dog".to_string()))
        .expect("Should find Dog class");

    assert!(dog.signature.as_ref().unwrap().contains("Mammal"));
}

// Test multiple inheritance
#[test]
fn test_python_multiple_inheritance() {
    let source = r#"
class Flyable:
    """Can fly."""
    pass

class Swimmable:
    """Can swim."""
    pass

class Duck(Flyable, Swimmable):
    """Ducks can fly and swim."""
    pass
"#;

    let chunks = parser::extract_chunks(source, "py");

    let duck = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Duck".to_string()))
        .expect("Should find Duck class");

    assert!(duck.signature.as_ref().unwrap().contains("Flyable"));
    assert!(duck.signature.as_ref().unwrap().contains("Swimmable"));

    if let Some(metadata) = &duck.metadata {
        if let Some(base_classes) = metadata.get("base_classes") {
            let bases = base_classes.as_array().unwrap();
            assert_eq!(bases.len(), 2);
        }
    }
}

// Test decorators on functions
#[test]
fn test_python_function_decorators() {
    let source = r#"
@staticmethod
def static_method():
    """A static method."""
    return "static"

@classmethod
def class_method(cls):
    """A class method."""
    return cls

@property
def my_property(self):
    """A property."""
    return self._value
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 3);

    // Check that decorators are tracked in metadata
    for chunk in &chunks {
        assert!(chunk.metadata.is_some());
        if let Some(metadata) = &chunk.metadata {
            let has_decorators = metadata.get("has_decorators").unwrap().as_bool().unwrap();
            assert!(has_decorators);

            let decorators = metadata.get("decorators").unwrap().as_array().unwrap();
            assert!(!decorators.is_empty());
        }
    }
}

// Test multiple decorators on one function
#[test]
fn test_python_multiple_decorators() {
    let source = r#"
@decorator_one
@decorator_two(arg="value")
@decorator_three
def decorated_function(x, y):
    """A function with multiple decorators."""
    return x + y
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    assert!(func.metadata.is_some());

    if let Some(metadata) = &func.metadata {
        let decorators = metadata.get("decorators").unwrap().as_array().unwrap();
        assert_eq!(decorators.len(), 3);

        // Check decorator names
        let decorator_texts: Vec<String> = decorators
            .iter()
            .map(|d| d.as_str().unwrap().to_string())
            .collect();

        assert!(decorator_texts.iter().any(|d| d.contains("decorator_one")));
        assert!(decorator_texts.iter().any(|d| d.contains("decorator_two")));
        assert!(decorator_texts
            .iter()
            .any(|d| d.contains("decorator_three")));
    }
}

// Test decorators on classes
#[test]
fn test_python_class_decorators() {
    let source = r#"
@dataclass
class Point:
    """A point in 2D space."""
    x: int
    y: int

@dataclass(frozen=True)
class ImmutablePoint:
    """An immutable point."""
    x: int
    y: int
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert!(chunks.len() >= 2);

    for chunk in &chunks {
        if chunk.kind == "class" {
            assert!(chunk.metadata.is_some());
            if let Some(metadata) = &chunk.metadata {
                let has_decorators = metadata.get("has_decorators").unwrap().as_bool().unwrap();
                assert!(has_decorators);

                let decorators = metadata.get("decorators").unwrap().as_array().unwrap();
                assert!(!decorators.is_empty());
                assert!(decorators
                    .iter()
                    .any(|d| d.as_str().unwrap().contains("dataclass")));
            }
        }
    }
}

// Test async functions
#[test]
fn test_python_async_function() {
    let source = r#"
async def fetch_data(url: str) -> dict:
    """Fetch data from a URL asynchronously."""
    return await http_client.get(url)
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    assert_eq!(func.symbol_name, Some("fetch_data".to_string()));
    assert_eq!(func.kind, "async_func");

    // Check that signature includes async
    assert!(func.signature.as_ref().unwrap().contains("async"));

    // Check metadata
    if let Some(metadata) = &func.metadata {
        let is_async = metadata.get("is_async").unwrap().as_bool().unwrap();
        assert!(is_async);
    }
}

// Test async methods
#[test]
fn test_python_async_method() {
    let source = r#"
class AsyncClient:
    """An async client."""

    async def connect(self) -> None:
        """Connect asynchronously."""
        await self._establish_connection()

    async def disconnect(self):
        """Disconnect asynchronously."""
        await self._close_connection()
"#;

    let chunks = parser::extract_chunks(source, "py");

    let connect = chunks
        .iter()
        .find(|c| c.symbol_name == Some("connect".to_string()))
        .expect("Should find connect method");

    assert_eq!(connect.kind, "async_method");

    if let Some(metadata) = &connect.metadata {
        let is_async = metadata.get("is_async").unwrap().as_bool().unwrap();
        assert!(is_async);
    }
}

// Test global variables
#[test]
fn test_python_global_variables() {
    let source = r#"
VERSION = "1.0.0"
DEBUG = True
max_retries = 3
default_timeout = 30.0
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert!(chunks.len() >= 4);

    // Check for constants (uppercase)
    let version = chunks
        .iter()
        .find(|c| c.symbol_name == Some("VERSION".to_string()))
        .expect("Should find VERSION constant");

    assert_eq!(version.kind, "constant");
    assert!(version.signature.is_some());
    assert!(version.signature.as_ref().unwrap().contains("1.0.0"));

    let debug = chunks
        .iter()
        .find(|c| c.symbol_name == Some("DEBUG".to_string()))
        .expect("Should find DEBUG constant");

    assert_eq!(debug.kind, "constant");

    // Check for variables (lowercase)
    let max_retries = chunks
        .iter()
        .find(|c| c.symbol_name == Some("max_retries".to_string()))
        .expect("Should find max_retries variable");

    assert_eq!(max_retries.kind, "variable");
}

// Test that class variables are NOT extracted as global variables
#[test]
fn test_python_class_variables_not_global() {
    let source = r#"
GLOBAL_VAR = 42

class MyClass:
    class_var = 100

    def __init__(self):
        self.instance_var = 200
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should find GLOBAL_VAR
    let global_var = chunks
        .iter()
        .find(|c| c.symbol_name == Some("GLOBAL_VAR".to_string()))
        .expect("Should find GLOBAL_VAR");

    assert_eq!(global_var.kind, "constant");

    // Should NOT find class_var as a separate chunk (it's inside the class)
    // Note: class attributes inside class bodies are not module-level assignments
    let class_var_count = chunks
        .iter()
        .filter(|c| c.symbol_name == Some("class_var".to_string()))
        .count();

    // We don't extract class variables as separate chunks
    assert_eq!(class_var_count, 0);
}

// Test edge case: nested classes
#[test]
fn test_python_nested_classes() {
    let source = r#"
class Outer:
    """Outer class."""

    class Inner:
        """Inner class."""
        def inner_method(self):
            """Inner method."""
            pass

    def outer_method(self):
        """Outer method."""
        pass
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should extract both classes
    let outer = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Outer".to_string()))
        .expect("Should find Outer class");

    assert_eq!(outer.kind, "class");

    let inner = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Inner".to_string()))
        .expect("Should find Inner class");

    assert_eq!(inner.kind, "class");
}

// Test edge case: decorated async function
#[test]
fn test_python_decorated_async_function() {
    let source = r#"
@retry(max_attempts=3)
@log_execution
async def fetch_with_retry(url: str) -> dict:
    """Fetch data with retry logic."""
    return await fetch_data(url)
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    assert_eq!(func.kind, "async_func");

    if let Some(metadata) = &func.metadata {
        let is_async = metadata.get("is_async").unwrap().as_bool().unwrap();
        assert!(is_async);

        let has_decorators = metadata.get("has_decorators").unwrap().as_bool().unwrap();
        assert!(has_decorators);

        let decorators = metadata.get("decorators").unwrap().as_array().unwrap();
        assert_eq!(decorators.len(), 2);
    }
}

// Test real-world example: dataclass with methods
#[test]
fn test_python_dataclass_example() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class User:
    """Represents a user in the system."""
    id: int
    name: str
    email: str
    is_active: bool = True

    def activate(self) -> None:
        """Activate the user."""
        self.is_active = True

    def deactivate(self) -> None:
        """Deactivate the user."""
        self.is_active = False

    @property
    def display_name(self) -> str:
        """Get the display name."""
        return self.name.title()
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Find the class
    let user_class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("User".to_string()) && c.kind == "class")
        .expect("Should find User class");

    assert!(user_class.docstring.is_some());

    if let Some(metadata) = &user_class.metadata {
        let has_decorators = metadata.get("has_decorators").unwrap().as_bool().unwrap();
        assert!(has_decorators);
    }

    // Find methods
    let activate = chunks
        .iter()
        .find(|c| c.symbol_name == Some("activate".to_string()))
        .expect("Should find activate method");

    assert_eq!(activate.kind, "method");

    let display_name = chunks
        .iter()
        .find(|c| c.symbol_name == Some("display_name".to_string()))
        .expect("Should find display_name property");

    assert_eq!(display_name.kind, "method");

    if let Some(metadata) = &display_name.metadata {
        let has_decorators = metadata.get("has_decorators").unwrap().as_bool().unwrap();
        assert!(has_decorators);
    }
}

// Test real-world example: async context manager
#[test]
fn test_python_async_context_manager() {
    let source = r#"
class AsyncDatabase:
    """Async database connection manager."""

    async def __aenter__(self):
        """Enter the async context."""
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Exit the async context."""
        await self.disconnect()

    async def connect(self):
        """Connect to the database."""
        pass

    async def disconnect(self):
        """Disconnect from the database."""
        pass
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Find async methods
    let async_methods = chunks.iter().filter(|c| c.kind == "async_method").count();

    assert!(async_methods >= 4, "Should have at least 4 async methods");
}

// Accuracy validation test: comprehensive extraction
#[test]
fn test_python_comprehensive_extraction_accuracy() {
    let source = r#"
"""Module for user management."""

from typing import List, Optional
from dataclasses import dataclass

VERSION = "2.0.0"
DEFAULT_ROLE = "user"

@dataclass
class User:
    """User model."""
    id: int
    name: str
    email: str

    @property
    def display_name(self) -> str:
        """Get display name."""
        return self.name

    @staticmethod
    def validate_email(email: str) -> bool:
        """Validate email format."""
        return "@" in email

class Admin(User):
    """Admin user with elevated privileges."""

    def grant_permission(self, user_id: int, permission: str) -> None:
        """Grant permission to a user."""
        pass

async def fetch_users(limit: int = 10) -> List[User]:
    """Fetch users from database."""
    return []

@retry(max_attempts=3)
async def save_user(user: User) -> bool:
    """Save user to database."""
    return True

def calculate_total(items: List[int]) -> int:
    """Calculate total of items."""
    return sum(items)
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Expected symbols:
    // 2 constants: VERSION, DEFAULT_ROLE
    // 2 classes: User, Admin
    // 3 methods in User: display_name, validate_email (and potentially __init__ from dataclass)
    // 1 method in Admin: grant_permission
    // 3 functions: fetch_users, save_user, calculate_total

    // At minimum we should have:
    // - 2 constants
    // - 2 classes
    // - 3 functions
    // - methods (at least 3)

    let constants = chunks.iter().filter(|c| c.kind == "constant").count();
    assert_eq!(constants, 2, "Should extract 2 constants");

    let classes = chunks.iter().filter(|c| c.kind == "class").count();
    assert_eq!(classes, 2, "Should extract 2 classes");

    let async_funcs = chunks.iter().filter(|c| c.kind == "async_func").count();
    assert_eq!(async_funcs, 2, "Should extract 2 async functions");

    let funcs = chunks.iter().filter(|c| c.kind == "func").count();
    assert!(funcs >= 1, "Should extract at least 1 regular function");

    // Verify metadata is populated correctly
    for chunk in &chunks {
        if chunk.kind.starts_with("async") {
            if let Some(metadata) = &chunk.metadata {
                let is_async = metadata.get("is_async").unwrap().as_bool().unwrap();
                assert!(is_async, "Async symbols should have is_async=true");
            }
        }
    }
}

// Integration test with real-world Python code
#[test]
fn test_python_real_world_api_fixture() {
    let fixture_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/python/sample_api.py"
    );

    let source = fs::read_to_string(fixture_path).expect("Failed to read Python fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Verify we extracted a comprehensive set of symbols
    assert!(
        chunks.len() > 10,
        "Should extract many symbols from real code"
    );

    // Verify constants
    let constants: Vec<_> = chunks.iter().filter(|c| c.kind == "constant").collect();
    assert!(
        constants.len() >= 3,
        "Should extract module-level constants"
    );

    // Verify classes
    let classes: Vec<_> = chunks.iter().filter(|c| c.kind == "class").collect();
    assert!(classes.len() >= 5, "Should extract all classes");

    // Verify we found specific classes
    let class_names: Vec<_> = classes
        .iter()
        .filter_map(|c| c.symbol_name.as_ref())
        .collect();

    assert!(class_names.contains(&&"Request".to_string()));
    assert!(class_names.contains(&&"Response".to_string()));
    assert!(class_names.contains(&&"APIException".to_string()));
    assert!(class_names.contains(&&"BaseClient".to_string()));
    assert!(class_names.contains(&&"HTTPClient".to_string()));

    // Verify decorated classes (dataclasses)
    let decorated_classes: Vec<_> = classes
        .iter()
        .filter(|c| {
            if let Some(metadata) = &c.metadata {
                metadata
                    .get("has_decorators")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            } else {
                false
            }
        })
        .collect();

    assert!(
        decorated_classes.len() >= 2,
        "Should find decorated classes"
    );

    // Verify inheritance
    let http_client = chunks
        .iter()
        .find(|c| c.symbol_name == Some("HTTPClient".to_string()) && c.kind == "class")
        .expect("Should find HTTPClient class");

    assert!(http_client.signature.is_some());
    assert!(http_client
        .signature
        .as_ref()
        .unwrap()
        .contains("BaseClient"));

    if let Some(metadata) = &http_client.metadata {
        let base_classes = metadata.get("base_classes").unwrap().as_array().unwrap();
        assert_eq!(base_classes.len(), 1);
        assert_eq!(base_classes[0].as_str().unwrap(), "BaseClient");
    }

    // Verify async functions
    let async_funcs: Vec<_> = chunks.iter().filter(|c| c.kind == "async_func").collect();

    assert!(async_funcs.len() >= 1, "Should extract async functions");

    // Verify methods
    let methods: Vec<_> = chunks
        .iter()
        .filter(|c| c.kind == "method" || c.kind == "async_method")
        .collect();

    assert!(methods.len() >= 5, "Should extract many methods");

    // Verify static/class methods have decorators
    let from_config = chunks
        .iter()
        .find(|c| c.symbol_name == Some("from_config".to_string()))
        .expect("Should find from_config classmethod");

    if let Some(metadata) = &from_config.metadata {
        let has_decorators = metadata
            .get("has_decorators")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(has_decorators, "classmethod should have decorators");
    }

    // Verify regular functions
    let funcs: Vec<_> = chunks.iter().filter(|c| c.kind == "func").collect();

    assert!(funcs.len() >= 2, "Should extract regular functions");

    // Overall accuracy check: we should have extracted most symbols
    // Expected: ~3 constants, ~5 classes, ~1+ async funcs, ~5+ methods, ~2+ funcs
    // Total: ~16+ symbols
    assert!(
        chunks.len() >= 16,
        "Should extract comprehensive symbol set from real code"
    );
}
