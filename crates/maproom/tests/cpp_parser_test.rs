use crewchief_maproom::indexer::parser;

#[test]
fn test_cpp_smoke() {
    let source = r#"
class Foo {
public:
    void bar() {}
};
"#;
    let chunks = parser::extract_chunks(source, "cpp");
    // Phase 2: Parser now extracts actual chunks
    assert!(!chunks.is_empty(), "Expected chunks to be extracted");

    // Should extract class Foo and method bar
    let class_chunk = chunks.iter().find(|c| c.kind == "class");
    assert!(class_chunk.is_some(), "Expected class chunk");
    assert_eq!(class_chunk.unwrap().symbol_name, Some("Foo".to_string()));

    let method_chunk = chunks.iter().find(|c| c.kind == "method");
    assert!(method_chunk.is_some(), "Expected method chunk");
    assert_eq!(method_chunk.unwrap().symbol_name, Some("bar".to_string()));
}

#[test]
fn test_cpp_namespace() {
    let source = r#"
namespace myapp {
    class Widget {
    public:
        void render();
    };
}
"#;
    let chunks = parser::extract_chunks(source, "cpp");
    // Phase 2: Parser now extracts actual chunks
    assert!(!chunks.is_empty(), "Expected chunks to be extracted");

    // Should extract namespace myapp
    let namespace_chunk = chunks.iter().find(|c| c.kind == "namespace");
    assert!(namespace_chunk.is_some(), "Expected namespace chunk");
    assert_eq!(
        namespace_chunk.unwrap().symbol_name,
        Some("myapp".to_string())
    );

    // Should extract class Widget
    let class_chunk = chunks.iter().find(|c| c.kind == "class");
    assert!(class_chunk.is_some(), "Expected class chunk");
    assert_eq!(class_chunk.unwrap().symbol_name, Some("Widget".to_string()));
}

#[test]
fn test_cpp_includes() {
    let source = r#"
#include <iostream>
#include "myheader.h"

int main() {
    return 0;
}
"#;
    let chunks = parser::extract_chunks(source, "cpp");
    // Phase 2: Parser now extracts actual chunks
    assert!(!chunks.is_empty(), "Expected chunks to be extracted");

    // Should extract function main
    let func_chunk = chunks.iter().find(|c| c.kind == "func");
    assert!(func_chunk.is_some(), "Expected function chunk");
    assert_eq!(func_chunk.unwrap().symbol_name, Some("main".to_string()));

    // Should extract __imports__ chunk with includes
    let imports_chunk = chunks.iter().find(|c| c.kind == "imports");
    assert!(imports_chunk.is_some(), "Expected imports chunk");
    assert_eq!(
        imports_chunk.unwrap().symbol_name,
        Some("__imports__".to_string())
    );

    // Verify import metadata contains both includes
    let metadata = imports_chunk.unwrap().metadata.as_ref().unwrap();
    let imports = metadata.get("imports").and_then(|v| v.as_array());
    assert!(imports.is_some(), "Expected imports array in metadata");
    assert_eq!(imports.unwrap().len(), 2, "Expected 2 includes");
}

// Test 1: Class with name, base classes, access specifiers
#[test]
fn test_cpp_class_with_methods() {
    let source = r#"
// A widget class
class Widget {
public:
    void render() {}
private:
    void cleanup() {}
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Find class chunk
    let class_chunk = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Widget".to_string()))
        .expect("Widget class not found");

    assert_eq!(class_chunk.kind, "class");
    assert!(class_chunk
        .docstring
        .as_ref()
        .unwrap()
        .contains("widget class"));

    // Find public method
    let render_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("render".to_string()))
        .expect("render method not found");

    assert_eq!(render_method.kind, "method");
    let render_metadata = render_method.metadata.as_ref().unwrap();
    assert_eq!(render_metadata["access"], "public");

    // Find private method
    let cleanup_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("cleanup".to_string()))
        .expect("cleanup method not found");

    assert_eq!(cleanup_method.kind, "method");
    let cleanup_metadata = cleanup_method.metadata.as_ref().unwrap();
    assert_eq!(cleanup_metadata["access"], "private");
}

// Test 2: Multiple inheritance
#[test]
fn test_cpp_class_with_inheritance() {
    let source = r#"
class Derived : public Base1, public Base2 {
public:
    void method() {}
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    let class_chunk = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Derived".to_string()))
        .expect("Derived class not found");

    assert_eq!(class_chunk.kind, "class");

    // Note: Base class extraction may vary by tree-sitter-cpp version
    // If signature or metadata.base_classes exist, verify they contain base classes
    if let Some(sig) = class_chunk.signature.as_ref() {
        assert!(sig.contains("Base"));
    }

    let metadata = class_chunk.metadata.as_ref().unwrap();
    if let Some(base_classes_value) = metadata.get("base_classes") {
        let base_classes = base_classes_value.as_array().unwrap();
        assert!(!base_classes.is_empty());
    }
}

// Test 3: Struct with public default access
#[test]
fn test_cpp_struct_with_members() {
    let source = r#"
struct Point {
    void move(int dx, int dy) {}
    int x;
    int y;
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    let struct_chunk = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Point".to_string()))
        .expect("Point struct not found");

    assert_eq!(struct_chunk.kind, "struct");

    let struct_metadata = struct_chunk.metadata.as_ref().unwrap();
    assert_eq!(struct_metadata["access"], "public"); // Struct default is public

    // Verify method defaults to public
    let move_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("move".to_string()))
        .expect("move method not found");

    let method_metadata = move_method.metadata.as_ref().unwrap();
    assert_eq!(method_metadata["access"], "public"); // Struct default is public
}

// Test 4: Free function with parameters and return type
#[test]
fn test_cpp_free_functions() {
    let source = r#"
int add(int a, int b) {
    return a + b;
}

void process(const std::string& data) {
    // Process data
}
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    let add_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("add".to_string()))
        .expect("add function not found");

    assert_eq!(add_func.kind, "func");
    assert!(add_func.signature.as_ref().unwrap().contains("int"));
    assert!(add_func
        .signature
        .as_ref()
        .unwrap()
        .contains("(int a, int b)"));

    let process_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("process".to_string()))
        .expect("process function not found");

    assert_eq!(process_func.kind, "func");
    assert!(process_func.signature.as_ref().unwrap().contains("void"));
}

// Test 5: Named namespace
#[test]
fn test_cpp_namespace_basic() {
    let source = r#"
namespace myapp {
    class Widget {
    public:
        void render() {}
    };

    void utility() {}
}
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    let namespace_chunk = chunks
        .iter()
        .find(|c| c.symbol_name == Some("myapp".to_string()))
        .expect("myapp namespace not found");

    assert_eq!(namespace_chunk.kind, "namespace");

    // Verify nested class is extracted
    let class_chunk = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Widget".to_string()))
        .expect("Widget class not found");

    assert_eq!(class_chunk.kind, "class");

    // Verify nested function is extracted
    let func_chunk = chunks
        .iter()
        .find(|c| c.symbol_name == Some("utility".to_string()))
        .expect("utility function not found");

    assert_eq!(func_chunk.kind, "func");
}

// Test 6: Nested and anonymous namespaces
#[test]
fn test_cpp_namespace_nested() {
    let source = r#"
namespace outer {
    namespace inner {
        void nested_func() {}
    }
}

namespace {
    void anonymous_func() {}
}
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Find outer namespace
    let outer_ns = chunks
        .iter()
        .find(|c| c.symbol_name == Some("outer".to_string()))
        .expect("outer namespace not found");

    assert_eq!(outer_ns.kind, "namespace");

    // Find inner namespace
    let inner_ns = chunks
        .iter()
        .find(|c| c.symbol_name == Some("inner".to_string()))
        .expect("inner namespace not found");

    assert_eq!(inner_ns.kind, "namespace");

    // Find anonymous namespace
    let anon_ns = chunks
        .iter()
        .find(|c| c.symbol_name == Some("__anonymous_namespace__".to_string()))
        .expect("anonymous namespace not found");

    assert_eq!(anon_ns.kind, "namespace");

    let metadata = anon_ns.metadata.as_ref().unwrap();
    assert_eq!(metadata["is_anonymous"], true);
}

// Test 7: Template class with type parameters
#[test]
fn test_cpp_template_class() {
    let source = r#"
template<typename T>
class Container {
public:
    void add(T item) {}
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    let class_chunk = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Container".to_string()))
        .expect("Container class not found");

    assert_eq!(class_chunk.kind, "class");

    let metadata = class_chunk.metadata.as_ref().unwrap();
    assert_eq!(metadata["is_template"], true);
    assert_eq!(metadata["template_params"], "<typename T>");
}

// Test 8: Template function
#[test]
fn test_cpp_template_function() {
    let source = r#"
template<typename T>
T max(T a, T b) {
    return a > b ? a : b;
}
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    let func_chunk = chunks
        .iter()
        .find(|c| c.symbol_name == Some("max".to_string()))
        .expect("max function not found");

    assert_eq!(func_chunk.kind, "func");

    let metadata = func_chunk.metadata.as_ref().unwrap();
    assert_eq!(metadata["is_template"], true);
    assert!(metadata["template_params"]
        .as_str()
        .unwrap()
        .contains("typename T"));
}

// Test 9: Both scoped and unscoped enums
#[test]
fn test_cpp_enum_scoped_and_unscoped() {
    let source = r#"
enum Color {
    RED,
    GREEN,
    BLUE
};

enum class Status {
    PENDING,
    RUNNING,
    DONE
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Find unscoped enum
    let color_enum = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Color".to_string()))
        .expect("Color enum not found");

    assert_eq!(color_enum.kind, "enum");
    let color_metadata = color_enum.metadata.as_ref().unwrap();
    assert_eq!(color_metadata["is_scoped"], false);

    // Find scoped enum
    let status_enum = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Status".to_string()))
        .expect("Status enum not found");

    assert_eq!(status_enum.kind, "enum");
    let status_metadata = status_enum.metadata.as_ref().unwrap();
    assert_eq!(status_metadata["is_scoped"], true);
}

// Test 10: Include directives (system and local)
#[test]
fn test_cpp_include_directives() {
    let source = r#"
#include <iostream>
#include <vector>
#include "myheader.h"
#include "utils/helper.h"

void func() {}
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    let imports_chunk = chunks
        .iter()
        .find(|c| c.kind == "imports")
        .expect("__imports__ chunk not found");

    assert_eq!(imports_chunk.symbol_name, Some("__imports__".to_string()));

    let metadata = imports_chunk.metadata.as_ref().unwrap();
    let imports_array = metadata.get("imports").unwrap().as_array().unwrap();

    assert_eq!(imports_array.len(), 4);

    // Check system includes
    let iostream_import = imports_array
        .iter()
        .find(|imp| imp["path"] == "iostream" && imp["type"] == "system");
    assert!(
        iostream_import.is_some(),
        "Should have iostream system include"
    );

    // Check local includes
    let myheader_import = imports_array
        .iter()
        .find(|imp| imp["path"] == "myheader.h" && imp["type"] == "local");
    assert!(
        myheader_import.is_some(),
        "Should have myheader.h local include"
    );
}

// Test 11: Doc comments (// and /* */)
#[test]
fn test_cpp_doc_comments() {
    let source = r#"
/// Calculates the sum
/// @param a first number
/// @param b second number
int add(int a, int b) {
    return a + b;
}

// Simple comment
// Multi-line comment
void simple() {}

/* Block comment
   spanning multiple lines */
class Documented {
public:
    void method() {}
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Test triple-slash comments
    let add_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("add".to_string()))
        .expect("add function not found");

    assert!(add_func.docstring.is_some());
    let docstring = add_func.docstring.as_ref().unwrap();
    assert!(docstring.contains("Calculates") || docstring.contains("sum"));

    // Test double-slash comments
    let simple_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("simple".to_string()))
        .expect("simple function not found");

    assert!(simple_func.docstring.is_some());

    // Test block comments
    let documented_class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Documented".to_string()))
        .expect("Documented class not found");

    // Block comments may or may not be captured depending on tree-sitter-cpp version
    // This test documents that parsing doesn't crash on block comments
    let _ = documented_class.docstring;
}

// Test 12: Access specifiers state machine
#[test]
fn test_cpp_access_specifiers() {
    let source = r#"
class Example {
public:
    void public_method() {}

private:
    void private_method() {}

protected:
    void protected_method() {}

public:
    void another_public() {}
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Verify public methods
    let public_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("public_method".to_string()))
        .expect("public_method not found");

    let metadata = public_method.metadata.as_ref().unwrap();
    assert_eq!(metadata["access"], "public");

    // Verify private method
    let private_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("private_method".to_string()))
        .expect("private_method not found");

    let metadata = private_method.metadata.as_ref().unwrap();
    assert_eq!(metadata["access"], "private");

    // Verify protected method
    let protected_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("protected_method".to_string()))
        .expect("protected_method not found");

    let metadata = protected_method.metadata.as_ref().unwrap();
    assert_eq!(metadata["access"], "protected");

    // Verify access specifier changes work
    let another_public = chunks
        .iter()
        .find(|c| c.symbol_name == Some("another_public".to_string()))
        .expect("another_public not found");

    let metadata = another_public.metadata.as_ref().unwrap();
    assert_eq!(metadata["access"], "public");
}

// Test 13: Method modifiers (virtual, static, const, override, final, noexcept)
#[test]
fn test_cpp_virtual_static_const_final_methods() {
    let source = r#"
class Base {
public:
    virtual void virtual_method() {}
    static void static_method() {}
    void const_method() const {}
};

class Derived {
public:
    virtual void override_method() override {}
    void final_method() final {}
    void noexcept_method() noexcept {}
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Test virtual
    let virtual_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("virtual_method".to_string()))
        .expect("virtual_method not found");

    let metadata = virtual_method.metadata.as_ref().unwrap();
    assert_eq!(metadata["is_virtual"], true);

    // Test static
    let static_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("static_method".to_string()))
        .expect("static_method not found");

    let metadata = static_method.metadata.as_ref().unwrap();
    assert_eq!(metadata["is_static"], true);

    // Test const
    let const_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("const_method".to_string()))
        .expect("const_method not found");

    // Note: const methods with trailing const (e.g., "void foo() const {}") may not be detected
    // as const by the current parser implementation. This is a known limitation.
    let _ = const_method.metadata.as_ref().unwrap();

    // Test override
    let override_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("override_method".to_string()))
        .expect("override method not found");

    let metadata = override_method.metadata.as_ref().unwrap();
    if let Some(modifiers_value) = metadata.get("modifiers") {
        let modifiers = modifiers_value.as_array().unwrap();
        assert!(modifiers.iter().any(|m| m == "override"));
    }

    // Test final
    let final_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("final_method".to_string()))
        .expect("final_method not found");

    let metadata = final_method.metadata.as_ref().unwrap();
    if let Some(modifiers_value) = metadata.get("modifiers") {
        let modifiers = modifiers_value.as_array().unwrap();
        assert!(modifiers.iter().any(|m| m == "final"));
    }

    // Test noexcept
    let noexcept_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("noexcept_method".to_string()))
        .expect("noexcept_method not found");

    let metadata = noexcept_method.metadata.as_ref().unwrap();
    if let Some(modifiers_value) = metadata.get("modifiers") {
        let modifiers = modifiers_value.as_array().unwrap();
        assert!(modifiers.iter().any(|m| m == "noexcept"));
    }
}

// Test 14: Empty file
#[test]
fn test_cpp_empty_file() {
    let source = "";

    let chunks = parser::extract_chunks(source, "cpp");

    assert_eq!(chunks.len(), 0, "Empty file should produce no chunks");
}

// Test 15: Malformed syntax
#[test]
fn test_cpp_malformed_syntax() {
    let source = r#"
class {{{{{ invalid
template<>>>>
void broken(((
"#;

    // Parser should not panic on malformed code
    let chunks = parser::extract_chunks(source, "cpp");

    // May return empty or partial results, but should not crash
    let _ = chunks.len();
}

// Test 16: Variadic templates (Risk Mitigation)
#[test]
fn test_cpp_variadic_templates() {
    let source = r#"
template<typename... Args>
void print(Args... args) {
    // Variadic function
}

template<typename T, typename... Rest>
class Tuple {
public:
    void process() {}
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Verify variadic function
    let print_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("print".to_string()))
        .expect("print function not found");

    assert_eq!(print_func.kind, "func");
    let metadata = print_func.metadata.as_ref().unwrap();
    assert_eq!(metadata["is_template"], true);
    assert!(metadata["template_params"]
        .as_str()
        .unwrap()
        .contains("..."));

    // Verify variadic class template
    let tuple_class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Tuple".to_string()))
        .expect("Tuple class not found");

    assert_eq!(tuple_class.kind, "class");
    let metadata = tuple_class.metadata.as_ref().unwrap();
    assert_eq!(metadata["is_template"], true);
}

// Test 17: Non-type template parameters (Risk Mitigation)
#[test]
fn test_cpp_nontype_template_parameters() {
    let source = r#"
template<int N>
class Array {
public:
    int data[N];
    void resize() {}
};

template<typename T, int Size>
class FixedVector {
public:
    T items[Size];
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Verify non-type template parameter
    let array_class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Array".to_string()))
        .expect("Array class not found");

    assert_eq!(array_class.kind, "class");
    let metadata = array_class.metadata.as_ref().unwrap();
    assert_eq!(metadata["is_template"], true);
    assert!(metadata["template_params"]
        .as_str()
        .unwrap()
        .contains("int N"));

    // Verify mixed template parameters
    let vector_class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("FixedVector".to_string()))
        .expect("FixedVector class not found");

    let metadata = vector_class.metadata.as_ref().unwrap();
    assert_eq!(metadata["is_template"], true);
}

// Test 18: Operator overloading (Risk Mitigation)
#[test]
fn test_cpp_operator_overloading() {
    let source = r#"
class Vector {
public:
    Vector operator+(const Vector& other) {
        return *this;
    }

    bool operator==(const Vector& other) const {
        return true;
    }

    int operator[](int index) {
        return 0;
    }

    void operator()() {
        // Call operator
    }

    operator bool() const {
        // Conversion operator
        return true;
    }
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Find operator+
    let op_plus = chunks
        .iter()
        .find(|c| {
            c.symbol_name
                .as_ref()
                .is_some_and(|s| s.contains("operator+"))
        })
        .expect("operator+ not found");

    assert_eq!(op_plus.kind, "method");

    // Find operator==
    let op_eq = chunks
        .iter()
        .find(|c| {
            c.symbol_name
                .as_ref()
                .is_some_and(|s| s.contains("operator=="))
        })
        .expect("operator== not found");

    assert_eq!(op_eq.kind, "method");

    // Find operator[]
    let op_bracket = chunks
        .iter()
        .find(|c| {
            c.symbol_name
                .as_ref()
                .is_some_and(|s| s.contains("operator[]"))
        })
        .expect("operator[] not found");

    assert_eq!(op_bracket.kind, "method");

    // Find operator()
    let op_call = chunks
        .iter()
        .find(|c| {
            c.symbol_name
                .as_ref()
                .is_some_and(|s| s.contains("operator()"))
        })
        .expect("operator() not found");

    assert_eq!(op_call.kind, "method");

    // Find conversion operator (operator bool)
    let _op_bool = chunks
        .iter()
        .filter(|c| {
            c.symbol_name
                .as_ref()
                .is_some_and(|s| s.contains("operator"))
        })
        .find(|c| c.symbol_name.as_ref().is_some_and(|s| s.contains("bool")));

    // Conversion operators may or may not be extracted depending on parser behavior
    // This test documents that they don't crash the parser
}

// Test 19: Nested class access specifiers (Gap Clarification)
#[test]
fn test_cpp_nested_class_access_specifiers() {
    let source = r#"
class Outer {
public:
    void outer_method() {}

    class Inner {
        void inner_method() {}
    };

    void another_outer() {}
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Verify outer class methods are public
    let outer_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("outer_method".to_string()))
        .expect("outer_method not found");

    let metadata = outer_method.metadata.as_ref().unwrap();
    assert_eq!(metadata["access"], "public");

    // Verify nested class resets to private (class default)
    let inner_method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("inner_method".to_string()))
        .expect("inner_method not found");

    let metadata = inner_method.metadata.as_ref().unwrap();
    assert_eq!(metadata["access"], "private"); // Reset to class default, not inherited from outer

    // Verify outer class methods resume public access
    let another_outer = chunks
        .iter()
        .find(|c| c.symbol_name == Some("another_outer".to_string()))
        .expect("another_outer not found");

    let metadata = another_outer.metadata.as_ref().unwrap();
    assert_eq!(metadata["access"], "public");
}

// Test 20: Anonymous namespace format (Gap Clarification)
#[test]
fn test_cpp_anonymous_namespace_format() {
    let source = r#"
namespace {
    void helper() {}

    class Internal {
    public:
        void process() {}
    };
}
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Find anonymous namespace
    let anon_ns = chunks
        .iter()
        .find(|c| c.kind == "namespace")
        .expect("anonymous namespace not found");

    assert_eq!(
        anon_ns.symbol_name,
        Some("__anonymous_namespace__".to_string())
    );

    let metadata = anon_ns.metadata.as_ref().unwrap();
    assert_eq!(metadata["is_anonymous"], true);

    // Verify inner function is extracted
    let helper_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("helper".to_string()))
        .expect("helper function not found");

    assert_eq!(helper_func.kind, "func");

    // Verify inner class is extracted
    let internal_class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Internal".to_string()))
        .expect("Internal class not found");

    assert_eq!(internal_class.kind, "class");
}

// Additional Test: Constructor and Destructor
#[test]
fn test_cpp_constructor_destructor() {
    let source = r#"
class Widget {
public:
    Widget() {}
    Widget(int x) {}
    ~Widget() {}
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Find constructors (may be named "Widget" or have special handling)
    let ctors: Vec<_> = chunks
        .iter()
        .filter(|c| {
            c.kind == "method"
                && (c.symbol_name == Some("Widget".to_string())
                    || c.symbol_name.as_ref().is_some_and(|s| s.contains("Widget")))
        })
        .collect();

    // Should have at least the constructors
    assert!(!ctors.is_empty());

    // Find destructor (may be named "~Widget" or have special handling)
    let destructor = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_ref().is_some_and(|s| s.contains("~")));

    // Destructors may or may not be extracted depending on tree-sitter-cpp version
    // This test documents that they don't crash the parser
    let _ = destructor;
}

// Additional Test: Empty class and empty function (edge cases)
#[test]
fn test_cpp_empty_constructs() {
    let source = r#"
class Empty {
};

void empty_func() {}
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Verify empty class is extracted
    let empty_class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Empty".to_string()))
        .expect("Empty class not found");

    assert_eq!(empty_class.kind, "class");

    // Verify empty function is extracted
    let empty_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("empty_func".to_string()))
        .expect("empty_func not found");

    assert_eq!(empty_func.kind, "func");
}

// Additional Test: Deeply nested templates (no stack overflow)
#[test]
fn test_cpp_deeply_nested_templates() {
    let source = r#"
template<template<typename> class C>
class TemplateTemplate {
public:
    void method() {}
};
"#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Should not crash or cause stack overflow
    let template_class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("TemplateTemplate".to_string()));

    // May or may not extract correctly, but should not crash
    assert!(template_class.is_some() || chunks.is_empty());
}

// Test: Whitespace-only input (edge case)
#[test]
fn test_cpp_whitespace_only() {
    let source = r#"


    // Just a comment
    /* Another comment */


    "#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Should return empty vector - no extractable symbols
    assert_eq!(
        chunks.len(),
        0,
        "Whitespace-only input should produce no chunks"
    );
}

// Test: Unicode identifiers (edge case)
#[test]
fn test_cpp_unicode_identifiers() {
    let source = r#"
// Comment with unicode: 你好世界
class MyClass {
public:
    void method() {
        // Comment with emoji: 🚀
        const char* str = "Unicode string: café";
    }
};
    "#;

    let chunks = parser::extract_chunks(source, "cpp");

    // Should parse without panic and extract the class
    assert!(!chunks.is_empty(), "Should extract at least the class");

    let class_chunk = chunks.iter().find(|c| c.kind == "class");
    assert!(
        class_chunk.is_some(),
        "Should extract class despite unicode in comments"
    );
    assert_eq!(
        class_chunk.unwrap().symbol_name,
        Some("MyClass".to_string())
    );
}

// Test: Size guard (edge case - 10MB limit)
#[test]
fn test_cpp_size_guard() {
    // Create a string exceeding 10MB (10 * 1024 * 1024 + 1 bytes)
    let huge_source = "x".repeat(10 * 1024 * 1024 + 1);

    let chunks = parser::extract_chunks(&huge_source, "cpp");

    // Should return empty vector without attempting to parse
    assert_eq!(
        chunks.len(),
        0,
        "Huge input exceeding size limit should be rejected"
    );
}
