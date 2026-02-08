use crewchief_maproom::indexer::parser;

#[test]
fn test_c_function_with_params() {
    let source = r#"
/**
 * Calculates the sum of two integers.
 * @param a First integer
 * @param b Second integer
 * @return Sum of a and b
 */
int add(int a, int b) {
    return a + b;
}

// Multiply two numbers
int multiply(int x, int y) {
    return x * y;
}
"#;

    let chunks = parser::extract_chunks(source, "c");

    // Find add function
    let add_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("add".to_string()))
        .expect("add function not found");

    assert_eq!(add_func.kind, "func");
    assert!(add_func.signature.is_some());
    let sig = add_func.signature.as_ref().unwrap();
    assert!(sig.contains("int"));
    assert!(sig.contains("(int a, int b)"));
    assert!(add_func.docstring.is_some());
    let doc = add_func.docstring.as_ref().unwrap();
    assert!(doc.contains("Calculates the sum"));
    assert!(doc.contains("@param"));

    // Verify metadata contains return type
    let metadata = add_func.metadata.as_ref().unwrap();
    assert_eq!(metadata["return_type"], "int");

    // Find multiply function
    let multiply_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("multiply".to_string()))
        .expect("multiply function not found");

    assert_eq!(multiply_func.kind, "func");
    assert!(multiply_func.docstring.is_some());
    let doc = multiply_func.docstring.as_ref().unwrap();
    assert!(doc.contains("Multiply two numbers"));
}

#[test]
fn test_c_struct_definition() {
    let source = r#"
// User data structure
struct User {
    int id;
    char name[50];
    int age;
};

/* Point in 2D space */
struct Point {
    double x;
    double y;
};
"#;

    let chunks = parser::extract_chunks(source, "c");

    // Find User struct
    let user_struct = chunks
        .iter()
        .find(|c| c.symbol_name == Some("User".to_string()))
        .expect("User struct not found");

    assert_eq!(user_struct.kind, "struct");
    assert!(user_struct.docstring.is_some());
    let doc = user_struct.docstring.as_ref().unwrap();
    assert!(doc.contains("User data structure"));

    // Verify metadata contains field count
    let metadata = user_struct.metadata.as_ref().unwrap();
    assert_eq!(metadata["field_count"], 3);

    // Find Point struct
    let point_struct = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Point".to_string()))
        .expect("Point struct not found");

    assert_eq!(point_struct.kind, "struct");
    let metadata = point_struct.metadata.as_ref().unwrap();
    assert_eq!(metadata["field_count"], 2);
    assert!(point_struct.docstring.is_some());
    let doc = point_struct.docstring.as_ref().unwrap();
    assert!(doc.contains("Point in 2D space"));
}

#[test]
fn test_c_enum_definition() {
    let source = r#"
// Color enumeration
enum Color {
    RED,
    GREEN,
    BLUE
};

/* Status codes */
enum Status {
    SUCCESS = 0,
    ERROR = 1,
    PENDING = 2
};
"#;

    let chunks = parser::extract_chunks(source, "c");

    // Find Color enum
    let color_enum = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Color".to_string()))
        .expect("Color enum not found");

    assert_eq!(color_enum.kind, "enum");
    assert!(color_enum.docstring.is_some());
    let doc = color_enum.docstring.as_ref().unwrap();
    assert!(doc.contains("Color enumeration"));

    // Verify metadata contains enumerator count
    let metadata = color_enum.metadata.as_ref().unwrap();
    assert_eq!(metadata["enumerator_count"], 3);

    // Find Status enum
    let status_enum = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Status".to_string()))
        .expect("Status enum not found");

    assert_eq!(status_enum.kind, "enum");
    let metadata = status_enum.metadata.as_ref().unwrap();
    assert_eq!(metadata["enumerator_count"], 3);
}

#[test]
fn test_c_typedef() {
    let source = r#"
// Unsigned integer alias
typedef unsigned int uint;

// Pointer to character
typedef char* string;

// Anonymous struct typedef
typedef struct {
    int x;
    int y;
} Point;

// Named struct typedef
typedef struct Rectangle {
    int width;
    int height;
} Rectangle;
"#;

    let chunks = parser::extract_chunks(source, "c");

    // Find uint typedef
    let uint_typedef = chunks
        .iter()
        .find(|c| c.symbol_name == Some("uint".to_string()))
        .expect("uint typedef not found");

    assert_eq!(uint_typedef.kind, "typedef");
    assert!(uint_typedef.signature.is_some());
    let sig = uint_typedef.signature.as_ref().unwrap();
    assert!(sig.contains("unsigned int"));
    assert!(uint_typedef.docstring.is_some());

    // Verify metadata contains underlying type
    let metadata = uint_typedef.metadata.as_ref().unwrap();
    assert!(metadata["underlying_type"]
        .as_str()
        .unwrap()
        .contains("unsigned int"));

    // Find Point typedef (anonymous struct)
    let point_typedef = chunks
        .iter()
        .find(|c| c.kind == "typedef" && c.symbol_name == Some("Point".to_string()))
        .expect("Point typedef not found");

    assert_eq!(point_typedef.kind, "typedef");

    // Find Rectangle typedef
    let rect_typedef = chunks
        .iter()
        .find(|c| c.kind == "typedef" && c.symbol_name == Some("Rectangle".to_string()))
        .expect("Rectangle typedef not found");

    assert_eq!(rect_typedef.kind, "typedef");
}

#[test]
fn test_c_global_variables() {
    let source = r#"
// Global counter
int global_count = 0;

// Maximum buffer size
const int MAX_SIZE = 1024;

// Multiple declarations
int a, b, c;
"#;

    let chunks = parser::extract_chunks(source, "c");

    // Find global_count variable
    let global_var = chunks
        .iter()
        .find(|c| c.symbol_name == Some("global_count".to_string()))
        .expect("global_count not found");

    assert_eq!(global_var.kind, "variable");
    assert!(global_var.signature.is_some());
    assert!(global_var.docstring.is_some());
    let doc = global_var.docstring.as_ref().unwrap();
    assert!(doc.contains("Global counter"));

    // Find MAX_SIZE constant
    let max_size = chunks
        .iter()
        .find(|c| c.symbol_name == Some("MAX_SIZE".to_string()))
        .expect("MAX_SIZE not found");

    assert_eq!(max_size.kind, "variable");

    // Find multiple declaration variables
    let var_a = chunks
        .iter()
        .find(|c| c.symbol_name == Some("a".to_string()));
    let var_b = chunks
        .iter()
        .find(|c| c.symbol_name == Some("b".to_string()));
    let var_c = chunks
        .iter()
        .find(|c| c.symbol_name == Some("c".to_string()));

    // At least one should be found (handling of multiple declarators may vary)
    assert!(var_a.is_some() || var_b.is_some() || var_c.is_some());
}

#[test]
fn test_c_includes() {
    let source = r#"
#include <stdio.h>
#include <stdlib.h>
#include "local.h"
#include "utils/helper.h"

int main() {
    return 0;
}
"#;

    let chunks = parser::extract_chunks(source, "c");

    // Find __imports__ chunk
    let imports_chunk = chunks
        .iter()
        .find(|c| c.kind == "imports")
        .expect("__imports__ chunk not found");

    assert_eq!(imports_chunk.symbol_name, Some("__imports__".to_string()));

    // Verify metadata contains all import types
    let metadata = imports_chunk.metadata.as_ref().unwrap();
    let imports_array = metadata.as_array().expect("metadata should be array");

    assert_eq!(imports_array.len(), 4);

    // Check for system includes
    let has_stdio = imports_array
        .iter()
        .any(|imp| imp["type"] == "system" && imp["path"].as_str().unwrap().contains("stdio.h"));
    assert!(has_stdio, "Should have stdio.h system include");

    let has_stdlib = imports_array
        .iter()
        .any(|imp| imp["type"] == "system" && imp["path"].as_str().unwrap().contains("stdlib.h"));
    assert!(has_stdlib, "Should have stdlib.h system include");

    // Check for local includes
    let has_local = imports_array
        .iter()
        .any(|imp| imp["type"] == "local" && imp["path"].as_str().unwrap().contains("local.h"));
    assert!(has_local, "Should have local.h local include");

    let has_helper = imports_array.iter().any(|imp| {
        imp["type"] == "local" && imp["path"].as_str().unwrap().contains("utils/helper.h")
    });
    assert!(has_helper, "Should have utils/helper.h local include");
}

#[test]
fn test_c_static_function() {
    let source = r#"
// Internal helper function
static int internal_add(int a, int b) {
    return a + b;
}

// External function
extern void external_func(void);

// Regular function
int regular_func(void) {
    return 0;
}
"#;

    let chunks = parser::extract_chunks(source, "c");

    // Find static function
    let static_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("internal_add".to_string()))
        .expect("internal_add function not found");

    assert_eq!(static_func.kind, "func");
    let metadata = static_func.metadata.as_ref().unwrap();
    assert_eq!(metadata["storage_class"], "static");

    // Find extern function
    let extern_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("external_func".to_string()))
        .expect("external_func function not found");

    assert_eq!(extern_func.kind, "func");
    let metadata = extern_func.metadata.as_ref().unwrap();
    assert_eq!(metadata["storage_class"], "extern");

    // Find regular function (no storage class)
    let regular_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("regular_func".to_string()))
        .expect("regular_func function not found");

    assert_eq!(regular_func.kind, "func");
    // Should not have storage_class in metadata, or it should be empty
    if let Some(metadata) = &regular_func.metadata {
        assert!(
            !metadata.as_object().unwrap().contains_key("storage_class")
                || metadata["storage_class"].is_null()
        );
    }
}

#[test]
fn test_c_empty_file() {
    let source = "";

    let chunks = parser::extract_chunks(source, "c");

    // Empty file should return empty chunks
    assert_eq!(chunks.len(), 0, "Empty file should produce no chunks");
}

#[test]
fn test_c_syntax_error() {
    let source = r#"
int broken_function(
    // Missing closing paren and body
struct {{{{ invalid
"#;

    // Parser should not panic on malformed code
    let chunks = parser::extract_chunks(source, "c");

    // May return empty or partial results, but should not crash
    // The key is that we reach this line without panicking
    let _ = chunks.len();
}

#[test]
fn test_c_whitespace_only_file() {
    let source = "   \n\n\t\t\n    \n\t  \n\n";

    let chunks = parser::extract_chunks(source, "c");

    // File with only whitespace should produce no chunks
    assert_eq!(
        chunks.len(),
        0,
        "Whitespace-only file should produce no chunks"
    );
}

#[test]
fn test_c_comment_only_file() {
    let source = r#"
// This is a line comment
// Another line comment

/*
 * This is a block comment
 * with multiple lines
 */

// More line comments
/* Single line block comment */
"#;

    let chunks = parser::extract_chunks(source, "c");

    // File with only comments should produce no chunks
    assert_eq!(
        chunks.len(),
        0,
        "Comment-only file should produce no chunks"
    );
}

#[test]
fn test_c_mixed_whitespace_comments() {
    let source = r#"

    // Comment with leading whitespace

        /* Block comment with spaces */

// Another comment



"#;

    let chunks = parser::extract_chunks(source, "c");

    // File with mixed whitespace and comments should produce no chunks
    assert_eq!(
        chunks.len(),
        0,
        "Mixed whitespace and comments should produce no chunks"
    );
}

#[test]
fn test_c_preprocessor_only_file() {
    let source = r#"
#ifndef MY_HEADER_H
#define MY_HEADER_H

#endif // MY_HEADER_H
"#;

    // Parser should not panic on preprocessor-only files
    let chunks = parser::extract_chunks(source, "c");

    // Header guard only - may be empty or may have imports chunk
    // The key is that we don't panic and return a valid Vec
    let _ = chunks.len();
    // Note: This is valid C preprocessor code, not a syntax error
}
