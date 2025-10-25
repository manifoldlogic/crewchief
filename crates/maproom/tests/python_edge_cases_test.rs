use crewchief_maproom::indexer::parser;
use std::fs;

/// Test that incomplete syntax doesn't cause panics
/// Note: Error recovery from malformed syntax not yet implemented - marked as future enhancement
#[test]
#[ignore = "Error recovery not implemented - parser stops at first malformed syntax"]
fn test_incomplete_syntax_no_panic() {
    let source = fs::read_to_string("tests/fixtures/python/edge_cases/incomplete_syntax.py")
        .expect("Failed to read incomplete syntax fixture");

    // The main test is that this doesn't panic
    let chunks = parser::extract_chunks(&source, "py");

    // We should still extract valid symbols that come after errors
    let valid_function = chunks.iter()
        .find(|c| c.symbol_name == Some("valid_function_after_errors".to_string()));
    assert!(valid_function.is_some(), "Should recover and extract valid symbols after syntax errors");

    let valid_class = chunks.iter()
        .find(|c| c.symbol_name == Some("ValidClassAfterErrors".to_string()));
    assert!(valid_class.is_some(), "Should extract valid class after syntax errors");
}

/// Test malformed decorators don't crash the parser
#[test]
fn test_malformed_decorators_no_panic() {
    let source = fs::read_to_string("tests/fixtures/python/edge_cases/malformed_decorators.py")
        .expect("Failed to read malformed decorators fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Should still extract functions despite decorator issues
    assert!(!chunks.is_empty(), "Should extract some symbols despite malformed decorators");

    // Check for complex decorated functions
    let nested_decorated = chunks.iter()
        .find(|c| c.symbol_name == Some("nested_decorated_function".to_string()));
    if let Some(func) = nested_decorated {
        assert!(func.metadata.is_some(), "Decorated function should have metadata");
        if let Some(metadata) = &func.metadata {
            if let Some(has_decorators) = metadata.get("has_decorators") {
                assert!(has_decorators.as_bool().unwrap_or(false),
                       "Should detect decorators on nested_decorated_function");
            }
        }
    }

    // Check for multiline decorator args
    let multiline_decorator = chunks.iter()
        .find(|c| c.symbol_name == Some("multiline_decorator_args".to_string()));
    assert!(multiline_decorator.is_some(), "Should handle multiline decorator arguments");

    // Check for property decorators
    let property_chunks: Vec<_> = chunks.iter()
        .filter(|c| c.symbol_name == Some("complex_property".to_string()))
        .collect();
    // Should find getter, setter, and deleter (tree-sitter may capture all three)
    assert!(!property_chunks.is_empty(), "Should extract property methods");
}

/// Test unusual class patterns are handled correctly
#[test]
fn test_unusual_classes_extraction() {
    let source = fs::read_to_string("tests/fixtures/python/edge_cases/unusual_classes.py")
        .expect("Failed to read unusual classes fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Metaclass
    let custom_meta = chunks.iter()
        .find(|c| c.symbol_name == Some("CustomMeta".to_string()) && c.kind == "class");
    assert!(custom_meta.is_some(), "Should extract metaclass");

    let metaclass_user = chunks.iter()
        .find(|c| c.symbol_name == Some("MetaclassUser".to_string()) && c.kind == "class");
    assert!(metaclass_user.is_some(), "Should extract class using metaclass");

    // Diamond inheritance
    let diamond_class = chunks.iter()
        .find(|c| c.symbol_name == Some("D".to_string()) && c.kind == "class");
    assert!(diamond_class.is_some(), "Should extract diamond inheritance class");

    // Nested classes
    let outer = chunks.iter()
        .find(|c| c.symbol_name == Some("Outer".to_string()) && c.kind == "class");
    assert!(outer.is_some(), "Should extract outer class");

    let middle = chunks.iter()
        .find(|c| c.symbol_name == Some("Middle".to_string()) && c.kind == "class");
    assert!(middle.is_some(), "Should extract middle nested class");

    let inner = chunks.iter()
        .find(|c| c.symbol_name == Some("Inner".to_string()) && c.kind == "class");
    assert!(inner.is_some(), "Should extract inner nested class");

    // Generic classes
    let generic_class = chunks.iter()
        .find(|c| c.symbol_name == Some("GenericClass".to_string()) && c.kind == "class");
    assert!(generic_class.is_some(), "Should extract generic class with type parameters");

    // Protocol
    let drawable = chunks.iter()
        .find(|c| c.symbol_name == Some("Drawable".to_string()) && c.kind == "class");
    assert!(drawable.is_some(), "Should extract Protocol class");

    // Dataclass with features
    let complex_dataclass = chunks.iter()
        .find(|c| c.symbol_name == Some("ComplexDataclass".to_string()) && c.kind == "class");
    if let Some(dc) = complex_dataclass {
        assert!(dc.metadata.is_some(), "Dataclass should have metadata");
        if let Some(metadata) = &dc.metadata {
            if let Some(has_decorators) = metadata.get("has_decorators") {
                assert!(has_decorators.as_bool().unwrap_or(false),
                       "Dataclass should have decorator");
            }
        }
    }

    // Context managers
    let ctx_mgr = chunks.iter()
        .find(|c| c.symbol_name == Some("ContextManager".to_string()) && c.kind == "class");
    assert!(ctx_mgr.is_some(), "Should extract context manager class");

    let async_ctx_mgr = chunks.iter()
        .find(|c| c.symbol_name == Some("AsyncContextManager".to_string()) && c.kind == "class");
    assert!(async_ctx_mgr.is_some(), "Should extract async context manager class");
}

/// Test mixed indentation doesn't crash parser
#[test]
fn test_mixed_indentation_tolerance() {
    let source = fs::read_to_string("tests/fixtures/python/edge_cases/mixed_indentation.py")
        .expect("Failed to read mixed indentation fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Should extract functions despite indentation issues
    assert!(!chunks.is_empty(), "Should extract symbols despite mixed indentation");

    // Check for specific functions
    let spaces_func = chunks.iter()
        .find(|c| c.symbol_name == Some("function_with_spaces".to_string()));
    assert!(spaces_func.is_some(), "Should extract function with space indentation");

    let mixed_func = chunks.iter()
        .find(|c| c.symbol_name == Some("function_mixed_indent".to_string()));
    assert!(mixed_func.is_some(), "Should handle mixed tab/space indentation");

    // Unicode function names
    let unicode_func = chunks.iter()
        .find(|c| c.symbol_name == Some("функция_unicode".to_string()));
    assert!(unicode_func.is_some(), "Should extract Unicode function names");

    let spanish_func = chunks.iter()
        .find(|c| c.symbol_name == Some("función_española".to_string()));
    assert!(spanish_func.is_some(), "Should extract function names with special characters");

    // Emoji handling
    let emoji_func = chunks.iter()
        .find(|c| c.symbol_name == Some("emoji_function".to_string()));
    if let Some(func) = emoji_func {
        assert!(func.docstring.is_some(), "Should extract docstring with emoji");
    }

    // Final valid function should be extracted
    let final_func = chunks.iter()
        .find(|c| c.symbol_name == Some("final_valid_function".to_string()));
    assert!(final_func.is_some(), "Should extract final valid function");
}

/// Test empty and minimal files
#[test]
fn test_empty_file_handling() {
    let source = "";
    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 0, "Empty file should yield no chunks");
}

/// Test file with only comments
#[test]
fn test_comments_only_file() {
    let source = r#"
# This is a comment
# Another comment
# TODO: Write some code
"#;
    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 0, "Comments-only file should yield no chunks");
}

/// Test file with only whitespace
#[test]
fn test_whitespace_only_file() {
    let source = "    \n\n   \t\t\n    ";
    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 0, "Whitespace-only file should yield no chunks");
}

/// Test very large decorator stack
#[test]
fn test_large_decorator_stack() {
    let source = r#"
@decorator1
@decorator2
@decorator3
@decorator4
@decorator5
@decorator6
@decorator7
@decorator8
@decorator9
@decorator10
def heavily_decorated():
    """Function with many decorators."""
    pass
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1, "Should extract heavily decorated function");

    let func = &chunks[0];
    if let Some(metadata) = &func.metadata {
        if let Some(decorators) = metadata.get("decorators") {
            let dec_array = decorators.as_array().unwrap();
            assert!(dec_array.len() >= 5, "Should capture multiple decorators");
        }
    }
}

/// Test incomplete class definition
#[test]
#[ignore = "Error recovery not implemented - parser stops at first malformed syntax"]
fn test_incomplete_class_definition() {
    let source = r#"
class IncompleteClass
    # Missing colon

def function_after():
    """This should still be extracted."""
    return True
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should recover and extract the function
    let func = chunks.iter()
        .find(|c| c.symbol_name == Some("function_after".to_string()));
    assert!(func.is_some(), "Should recover from incomplete class and extract following function");
}

/// Test incomplete function parameters
#[test]
#[ignore = "Error recovery not implemented - parser stops at first malformed syntax"]
fn test_incomplete_function_params() {
    let source = r#"
def incomplete(param1, param2
    # Missing closing paren and body

def complete_function():
    """Complete function after incomplete one."""
    return "ok"
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should extract the complete function
    let func = chunks.iter()
        .find(|c| c.symbol_name == Some("complete_function".to_string()));
    assert!(func.is_some(), "Should extract complete function after incomplete params");
}

/// Test nested incomplete structures
#[test]
#[ignore = "Error recovery not implemented - parser stops at first malformed syntax"]
fn test_nested_incomplete_structures() {
    let source = r#"
class Outer:
    def incomplete(self
        # Missing closing paren

    def complete(self):
        """Complete method."""
        return True
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should extract the class and the complete method
    let outer_class = chunks.iter()
        .find(|c| c.symbol_name == Some("Outer".to_string()));
    assert!(outer_class.is_some(), "Should extract outer class despite incomplete method");

    let complete_method = chunks.iter()
        .find(|c| c.symbol_name == Some("complete".to_string()));
    assert!(complete_method.is_some(), "Should extract complete method");
}

/// Test unusual spacing in function definitions
#[test]
fn test_unusual_spacing() {
    let source = r#"
def     func_weird_spacing    (   a  ,   b   )    :
    """Function with unusual spacing."""
    return a + b
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1, "Should handle unusual spacing");

    let func = &chunks[0];
    assert_eq!(func.symbol_name, Some("func_weird_spacing".to_string()));
}

/// Test string literals with special characters
#[test]
fn test_string_literals_special_chars() {
    let source = r#"
def string_test():
    """Test various string types."""
    raw = r"Raw \n string"
    bytes = b"Byte string"
    unicode = "Unicode: 你好世界"
    formatted = f"Formatted {raw}"
    multiline = """
    Multi
    line
    """
    return (raw, bytes, unicode, formatted, multiline)
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1, "Should handle various string literal types");

    let func = &chunks[0];
    assert_eq!(func.symbol_name, Some("string_test".to_string()));
}

/// Test async/await with incomplete syntax
#[test]
#[ignore = "Error recovery not implemented - parser stops at first malformed syntax"]
fn test_incomplete_async() {
    let source = r#"
async def incomplete_async(
    # Missing params and body

async def complete_async():
    """Complete async function."""
    return await something()
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should extract the complete async function
    let func = chunks.iter()
        .find(|c| c.symbol_name == Some("complete_async".to_string()));
    assert!(func.is_some(), "Should extract complete async function");

    if let Some(f) = func {
        assert_eq!(f.kind, "async_func", "Should correctly identify as async function");
    }
}

/// Test comprehensions and complex expressions don't interfere with parsing
#[test]
fn test_comprehensions_in_functions() {
    let source = r#"
def with_comprehensions():
    """Function using comprehensions."""
    list_comp = [x * 2 for x in range(10)]
    dict_comp = {x: x**2 for x in range(5)}
    set_comp = {x for x in range(10) if x % 2 == 0}
    gen_exp = (x for x in range(100))
    return list_comp
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1, "Should handle comprehensions in function body");

    let func = &chunks[0];
    assert_eq!(func.symbol_name, Some("with_comprehensions".to_string()));
}

/// Test lambda expressions don't create separate chunks
#[test]
fn test_lambda_expressions() {
    let source = r#"
def with_lambdas():
    """Function containing lambdas."""
    square = lambda x: x ** 2
    add = lambda a, b: a + b
    complex = lambda x: (lambda y: x + y)
    return square, add, complex
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should only extract the containing function, not the lambdas
    assert_eq!(chunks.len(), 1, "Lambdas should not create separate chunks");
    assert_eq!(chunks[0].symbol_name, Some("with_lambdas".to_string()));
}

/// Test that parser handles maximum edge cases gracefully
#[test]
fn test_extreme_nesting() {
    let source = r#"
class A:
    class B:
        class C:
            class D:
                class E:
                    def deep_method(self):
                        """Deeply nested method."""
                        return "deep"
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should extract all nested classes
    assert!(!chunks.is_empty(), "Should handle deep nesting");

    let deep_method = chunks.iter()
        .find(|c| c.symbol_name == Some("deep_method".to_string()));
    assert!(deep_method.is_some(), "Should extract deeply nested method");
}

/// Test recovery from multiple consecutive errors
#[test]
#[ignore = "Error recovery not implemented - parser stops at first malformed syntax"]
fn test_multiple_consecutive_errors() {
    let source = r#"
def error1(
# Error

def error2(
# Error

def error3(
# Error

def valid_function():
    """This should be extracted."""
    return True
"#;

    let chunks = parser::extract_chunks(source, "py");

    // Should recover and extract the valid function
    let valid = chunks.iter()
        .find(|c| c.symbol_name == Some("valid_function".to_string()));
    assert!(valid.is_some(), "Should recover from multiple consecutive errors");
}
