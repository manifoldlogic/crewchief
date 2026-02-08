//! C++ language parser for the maproom indexer.
//!
//! This module extracts semantic symbols from C++ source code using tree-sitter-cpp.
//!
//! # Extracted Constructs
//!
//! - **Classes and Structs**: Including inheritance, template parameters, access specifiers
//! - **Functions and Methods**: Free functions, member functions, operator overloads
//! - **Namespaces**: Named and anonymous namespaces
//! - **Enums**: Both scoped (`enum class`) and unscoped enums
//! - **Templates**: Template classes, functions, and parameter extraction
//! - **Include Directives**: Collected into a single `__imports__` chunk
//! - **Doc Comments**: `//` and `/* */` style comments
//!
//! # Design Approach
//!
//! The parser uses a recursive AST walker (`walk_cpp_decls`) that dispatches to specialized
//! extraction functions based on node types. Access specifiers for classes are tracked using
//! a state machine that processes `public:`, `private:`, and `protected:` sections.
//!
//! # Known Limitations
//!
//! - **Preprocessor macros**: Only `#include` directives are captured; `#define` and conditionals are ignored
//! - **C++20+ features**: Concepts, modules, and coroutines are not extracted
//! - **Template metaprogramming**: SFINAE and advanced template patterns are not analyzed
//! - **constexpr modifiers**: Detection deferred to future enhancement
//!
//! # Access Specifier Handling
//!
//! Classes default to `private` access; structs default to `public`. The parser maintains
//! a current access state while walking class/struct bodies, updating when encountering
//! `public:`, `private:`, or `protected:` labels. Nested classes reset to their own defaults.

use std::time::Instant;

use tracing::{debug, warn};
use tree_sitter::{Node, Parser};

use super::common::lang_cpp;
use crate::indexer::SymbolChunk;

// Maximum source size to prevent resource exhaustion (10MB)
const MAX_SOURCE_SIZE: usize = 10 * 1024 * 1024;

// Access specifier constants
const ACCESS_PUBLIC: &str = "public";
const ACCESS_PRIVATE: &str = "private";
const ACCESS_PROTECTED: &str = "protected";

pub(super) fn extract_cpp_chunks(source: &str) -> Vec<SymbolChunk> {
    // Guard against extremely large files
    if source.len() > MAX_SOURCE_SIZE {
        warn!(
            language = "cpp",
            source_length = source.len(),
            max_size = MAX_SOURCE_SIZE,
            "C++ source exceeds maximum size limit - skipping parse"
        );
        return vec![];
    }

    let mut parser = Parser::new();
    parser
        .set_language(&lang_cpp())
        .expect("Failed to set C++ language");

    let start = Instant::now();
    let tree = match parser.parse(source, None) {
        Some(tree) => tree,
        None => {
            warn!(
                language = "cpp",
                source_length = source.len(),
                "Failed to parse C++ source - tree-sitter returned None"
            );
            return vec![];
        }
    };
    let parse_duration = start.elapsed();

    if parse_duration.as_secs() >= 1 {
        debug!(
            language = "cpp",
            source_length = source.len(),
            duration_ms = parse_duration.as_millis() as u64,
            "Slow C++ parse detected"
        );
    }

    let mut chunks = Vec::new();
    let mut includes = Vec::new();

    walk_cpp_decls(source, tree.root_node(), &mut chunks, &mut includes);

    // Create __imports__ chunk from includes if we collected any
    if !includes.is_empty() {
        chunks.push(SymbolChunk {
            symbol_name: Some("__imports__".to_string()),
            kind: "imports".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 1,
            metadata: Some(serde_json::json!({"imports": includes})),
        });
    }

    chunks
}

fn walk_cpp_decls(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    includes: &mut Vec<serde_json::Value>,
) {
    match node.kind() {
        "function_definition" => extract_cpp_function(source, node, chunks, None),
        "class_specifier" => extract_cpp_class(source, node, chunks, None),
        "struct_specifier" => extract_cpp_struct(source, node, chunks, None),
        "enum_specifier" => extract_cpp_enum(source, node, chunks),
        "namespace_definition" => extract_cpp_namespace(source, node, chunks, includes),
        "template_declaration" => extract_cpp_template(source, node, chunks, includes),
        "preproc_include" => collect_cpp_include(source, node, includes),
        _ => {}
    }

    // Recursively walk child nodes
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_cpp_decls(source, child, chunks, includes);
    }
}

/// Extracts a class declaration.
///
/// Classes default to `private` access for members.
fn extract_cpp_class(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    template_params: Option<String>,
) {
    extract_cpp_class_or_struct(
        source,
        node,
        chunks,
        template_params,
        ACCESS_PRIVATE,
        "class",
    );
}

/// Extracts a struct declaration.
///
/// Structs default to `public` access for members.
fn extract_cpp_struct(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    template_params: Option<String>,
) {
    extract_cpp_class_or_struct(
        source,
        node,
        chunks,
        template_params,
        ACCESS_PUBLIC,
        "struct",
    );
}

/// Shared helper for extracting class and struct declarations.
///
/// The only difference between classes and structs in C++ is the default access specifier:
/// classes default to `private`, structs default to `public`.
///
/// # Parameters
///
/// - `source`: The full source text
/// - `node`: The `class_specifier` or `struct_specifier` AST node
/// - `chunks`: Accumulator for extracted symbol chunks
/// - `template_params`: Template parameters if preceded by a template declaration
/// - `default_access`: Default access level for members (`ACCESS_PRIVATE` or `ACCESS_PUBLIC`)
/// - `kind`: The chunk kind (`"class"` or `"struct"`)
fn extract_cpp_class_or_struct(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    template_params: Option<String>,
    default_access: &str,
    kind: &str,
) {
    // Extract name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    if name.is_none() {
        debug!(
            node_kind = node.kind(),
            line = node.start_position().row + 1,
            kind = kind,
            "Failed to extract class/struct name - identifier not found"
        );
    }

    // Extract base classes (inheritance)
    let mut base_classes = Vec::new();
    if let Some(base_clause) = node.child_by_field_name("base_clause") {
        let mut cursor = base_clause.walk();
        for child in base_clause.children(&mut cursor) {
            if child.kind() == "type_identifier" || child.kind() == "scoped_type_identifier" {
                if let Ok(base_name) = child.utf8_text(source.as_bytes()) {
                    base_classes.push(base_name.to_string());
                }
            }
        }
    }

    // Build signature with base classes
    let signature = if !base_classes.is_empty() {
        Some(format!(": {}", base_classes.join(", ")))
    } else {
        None
    };

    // Extract doc comment
    let docstring = extract_cpp_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "access".to_string(),
        serde_json::Value::String(default_access.to_string()),
    );

    if !base_classes.is_empty() {
        metadata_obj.insert(
            "base_classes".to_string(),
            serde_json::Value::Array(
                base_classes
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            ),
        );
    }

    if let Some(ref tp) = template_params {
        metadata_obj.insert("is_template".to_string(), serde_json::Value::Bool(true));
        metadata_obj.insert(
            "template_params".to_string(),
            serde_json::Value::String(tp.clone()),
        );
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: kind.to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });

    // Walk class/struct body with access specifier tracking
    if let Some(body) = node.child_by_field_name("body") {
        walk_cpp_class_body(source, body, chunks, default_access);
    }
}

/// Walks the body of a class or struct, extracting members and tracking access specifiers.
///
/// Maintains a state machine for access control labels (`public:`, `private:`, `protected:`),
/// updating the current access level as each section is encountered. All members extracted
/// within a section inherit that section's access level until the next label.
///
/// # Parameters
///
/// - `body`: The `field_declaration_list` node from the class/struct declaration
/// - `source`: The full source text
/// - `chunks`: Accumulator for extracted symbol chunks
/// - `default_access`: Starting access level (`ACCESS_PRIVATE` for class, `ACCESS_PUBLIC` for struct)
fn walk_cpp_class_body(
    source: &str,
    body: Node,
    chunks: &mut Vec<SymbolChunk>,
    default_access: &str,
) {
    let mut current_access = default_access;
    let mut cursor = body.walk();

    for child in body.children(&mut cursor) {
        match child.kind() {
            "access_specifier" => {
                // Extract "public:", "private:", or "protected:"
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    let access = text.trim_end_matches(':').trim();
                    current_access = match access {
                        ACCESS_PUBLIC => ACCESS_PUBLIC,
                        ACCESS_PRIVATE => ACCESS_PRIVATE,
                        ACCESS_PROTECTED => ACCESS_PROTECTED,
                        _ => current_access, // Keep current if unknown
                    };
                }
            }
            "function_definition" => {
                extract_cpp_method(source, child, chunks, None, current_access);
            }
            "class_specifier" => {
                // Nested class - reset access to private (class default)
                extract_cpp_class(source, child, chunks, None);
            }
            "struct_specifier" => {
                // Nested struct - reset access to public (struct default)
                extract_cpp_struct(source, child, chunks, None);
            }
            "template_declaration" => {
                // Template member function or nested template class
                extract_cpp_template_member(source, child, chunks, current_access);
            }
            _ => {}
        }
    }
}

fn extract_cpp_function(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    template_params: Option<String>,
) {
    // Check if inside a class (method vs free function)
    let is_method = is_inside_cpp_class(node);

    if is_method {
        // Will be handled by walk_cpp_class_body
        return;
    }

    // Free function
    extract_cpp_function_impl(source, node, chunks, template_params, ACCESS_PUBLIC, false);
}

fn extract_cpp_method(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    template_params: Option<String>,
    access: &str,
) {
    extract_cpp_function_impl(source, node, chunks, template_params, access, true);
}

fn extract_cpp_function_impl(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    template_params: Option<String>,
    access: &str,
    is_method: bool,
) {
    // Extract function name
    let declarator = node.child_by_field_name("declarator");
    let name = extract_function_name(source, declarator);

    if name.is_none() {
        debug!(
            node_kind = node.kind(),
            line = node.start_position().row + 1,
            "Failed to extract function name - identifier not found"
        );
    }

    // Extract parameters
    let params = extract_function_parameters(source, declarator);

    // Extract return type
    let return_type = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract function modifiers
    let modifiers = extract_cpp_function_modifiers(source, node);
    let is_virtual = modifiers.contains(&"virtual");
    let is_static = modifiers.contains(&"static");
    let is_const = modifiers.contains(&"const");
    let is_noexcept = modifiers.contains(&"noexcept");
    let is_override = modifiers.contains(&"override");
    let is_final = modifiers.contains(&"final");

    // Build signature
    let signature =
        build_cpp_function_signature(return_type.as_deref(), params.as_deref(), &modifiers);

    // Extract doc comment
    let docstring = extract_cpp_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "access".to_string(),
        serde_json::Value::String(access.to_string()),
    );
    metadata_obj.insert(
        "is_virtual".to_string(),
        serde_json::Value::Bool(is_virtual),
    );
    metadata_obj.insert("is_static".to_string(), serde_json::Value::Bool(is_static));
    metadata_obj.insert("is_const".to_string(), serde_json::Value::Bool(is_const));

    if let Some(ref tp) = template_params {
        metadata_obj.insert("is_template".to_string(), serde_json::Value::Bool(true));
        metadata_obj.insert(
            "template_params".to_string(),
            serde_json::Value::String(tp.clone()),
        );
    }

    // Collect additional modifiers
    let mut mod_list = Vec::new();
    if is_override {
        mod_list.push("override");
    }
    if is_final {
        mod_list.push("final");
    }
    if is_noexcept {
        mod_list.push("noexcept");
    }
    if !mod_list.is_empty() {
        metadata_obj.insert(
            "modifiers".to_string(),
            serde_json::Value::Array(
                mod_list
                    .iter()
                    .map(|s| serde_json::Value::String(s.to_string()))
                    .collect(),
            ),
        );
    }

    let start = node.start_position();
    let end = node.end_position();

    let kind = if is_method { "method" } else { "func" };

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: kind.to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_cpp_enum(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract enum name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    if name.is_none() {
        debug!(
            node_kind = node.kind(),
            line = node.start_position().row + 1,
            "Failed to extract enum name - identifier not found"
        );
    }

    // Check if scoped enum (enum class)
    let is_scoped = node
        .utf8_text(source.as_bytes())
        .ok()
        .map(|text| text.contains("enum class") || text.contains("enum struct"))
        .unwrap_or(false);

    // Extract doc comment
    let docstring = extract_cpp_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert("is_scoped".to_string(), serde_json::Value::Bool(is_scoped));

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "enum".to_string(),
        signature: None,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_cpp_namespace(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    includes: &mut Vec<serde_json::Value>,
) {
    // Extract namespace name (may be None for anonymous namespaces)
    let has_name_node = node.child_by_field_name("name").is_some();
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Log only if a name node exists but text extraction failed (not anonymous)
    if name.is_none() && has_name_node {
        debug!(
            node_kind = node.kind(),
            line = node.start_position().row + 1,
            "Failed to extract namespace name - identifier not found"
        );
    }

    let is_anonymous = name.is_none();
    let symbol_name = if is_anonymous {
        Some("__anonymous_namespace__".to_string())
    } else {
        name
    };

    // Extract doc comment
    let docstring = extract_cpp_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    if is_anonymous {
        metadata_obj.insert("is_anonymous".to_string(), serde_json::Value::Bool(true));
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name,
        kind: "namespace".to_string(),
        signature: None,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });

    // Recursively walk namespace body
    if let Some(body) = node.child_by_field_name("body") {
        walk_cpp_decls(source, body, chunks, includes);
    }
}

/// Extracts template parameters from a template declaration node.
///
/// Returns the raw template parameter list text (e.g., `"<typename T, int N>"`).
/// Supports type parameters (`typename`, `class`), non-type parameters, and
/// variadic parameters (`typename... Args`).
///
/// # Approach
///
/// Extracts the text between `template` and the following declaration, capturing
/// the full parameter list including defaults. Does not parse individual parameters
/// into structured data - the raw text is preserved in metadata. The inner declaration
/// (class, struct, or function) is then delegated to the appropriate extraction function.
fn extract_cpp_template(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    includes: &mut Vec<serde_json::Value>,
) {
    // Extract template parameters
    let template_params = node
        .child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Find the inner declaration and delegate
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "class_specifier" => {
                extract_cpp_class(source, child, chunks, template_params.clone());
                return;
            }
            "struct_specifier" => {
                extract_cpp_struct(source, child, chunks, template_params.clone());
                return;
            }
            "function_definition" => {
                extract_cpp_function(source, child, chunks, template_params.clone());
                return;
            }
            "declaration" => {
                // Template function declaration
                walk_cpp_decls(source, child, chunks, includes);
                return;
            }
            _ => {}
        }
    }
}

fn extract_cpp_template_member(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    access: &str,
) {
    // Extract template parameters
    let template_params = node
        .child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Find the inner declaration and delegate
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                extract_cpp_method(source, child, chunks, template_params, access);
                return;
            }
            "class_specifier" => {
                extract_cpp_class(source, child, chunks, template_params);
                return;
            }
            "struct_specifier" => {
                extract_cpp_struct(source, child, chunks, template_params);
                return;
            }
            _ => {}
        }
    }
}

fn collect_cpp_include(source: &str, node: Node, includes: &mut Vec<serde_json::Value>) {
    // Extract the path node
    let path_node = node.child_by_field_name("path");
    if path_node.is_none() {
        return;
    }

    let path_node = path_node.unwrap();
    let path_text = match path_node.utf8_text(source.as_bytes()) {
        Ok(text) => text,
        Err(_) => return,
    };

    // Determine if system or local include based on node kind
    let include_type = match path_node.kind() {
        "system_lib_string" => "system", // <vector>
        "string_literal" => "local",     // "utils/foo.h"
        _ => "system",                   // Default to system
    };

    // Strip delimiters (< > or " ")
    let path = path_text
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim_start_matches('"')
        .trim_end_matches('"');

    includes.push(serde_json::json!({
        "path": path,
        "type": include_type
    }));
}

fn extract_cpp_doc_comment(source: &str, node: Node) -> Option<String> {
    let start_line = node.start_position().row;
    let lines: Vec<&str> = source.lines().collect();
    let mut doc_lines = Vec::new();

    // Walk backward from the line before the node
    for i in (0..start_line).rev() {
        let line = lines.get(i)?;
        let trimmed = line.trim();

        if trimmed.starts_with("///") {
            // Doxygen-style doc comment
            let comment = trimmed.trim_start_matches("///").trim();
            doc_lines.insert(0, comment);
        } else if trimmed.starts_with("//") {
            // Regular comment
            let comment = trimmed.trim_start_matches("//").trim();
            doc_lines.insert(0, comment);
        } else if trimmed.starts_with("/*") || trimmed.starts_with('*') {
            // Block comment (simplified - doesn't handle multi-line properly)
            let comment = trimmed
                .trim_start_matches("/*")
                .trim_end_matches("*/")
                .trim_start_matches('*')
                .trim();
            doc_lines.insert(0, comment);
        } else if !trimmed.is_empty() {
            // Non-comment, non-blank line - stop
            break;
        }
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}

// Helper functions

fn is_inside_cpp_class(node: Node) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "class_specifier" | "struct_specifier" => return true,
            _ => current = parent.parent(),
        }
    }
    false
}

/// Extracts the function name from a function declarator node.
///
/// Handles operator overloads (e.g., `operator+`, `operator<<`) and special
/// functions like constructors and destructors. Returns the raw function name
/// without parameters or qualifiers.
///
/// # Behavior
///
/// - For normal functions: Returns identifier text (e.g., `"calculate"`)
/// - For operator overloads: Returns full operator signature (e.g., `"operator+"`)
/// - For conversion operators: Returns `"operator <type>"`
/// - For constructors/destructors: Returns class name or `"~ClassName"`
fn extract_function_name(source: &str, declarator: Option<Node>) -> Option<String> {
    let declarator = declarator?;

    // Handle different declarator types
    match declarator.kind() {
        "function_declarator" => {
            // Try field name "declarator" first, then scan children for name nodes
            if let Some(name_node) = declarator.child_by_field_name("declarator") {
                extract_function_name(source, Some(name_node))
            } else {
                // Scan children for identifier/field_identifier/operator_name
                let mut cursor = declarator.walk();
                for child in declarator.children(&mut cursor) {
                    match child.kind() {
                        "identifier"
                        | "field_identifier"
                        | "operator_name"
                        | "destructor_name"
                        | "qualified_identifier"
                        | "scoped_identifier" => {
                            return child
                                .utf8_text(source.as_bytes())
                                .ok()
                                .map(|s| s.to_string());
                        }
                        _ => {}
                    }
                }
                None
            }
        }
        "identifier" | "field_identifier" => {
            // Found the name
            declarator
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string())
        }
        "operator_name" => {
            // Operator overload (e.g., operator+, operator==)
            declarator
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string())
        }
        "qualified_identifier" | "scoped_identifier" => {
            // Qualified name (e.g., ClassName::method)
            declarator
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string())
        }
        "pointer_declarator" | "reference_declarator" => {
            // Pointer or reference return type - recurse
            if let Some(decl) = declarator.child_by_field_name("declarator") {
                extract_function_name(source, Some(decl))
            } else {
                None
            }
        }
        "destructor_name" => {
            // Destructor (e.g., ~ClassName)
            declarator
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string())
        }
        _ => None,
    }
}

fn extract_function_parameters(source: &str, declarator: Option<Node>) -> Option<String> {
    let declarator = declarator?;

    // Find parameter_list node
    if declarator.kind() == "function_declarator" {
        if let Some(params) = declarator.child_by_field_name("parameters") {
            return params
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());
        }
    }

    // Recurse into nested declarators
    for i in 0..declarator.child_count() {
        if let Some(child) = declarator.child(i) {
            if child.kind() == "function_declarator" {
                if let Some(params) = child.child_by_field_name("parameters") {
                    return params
                        .utf8_text(source.as_bytes())
                        .ok()
                        .map(|s| s.to_string());
                }
            }
            // Try recursing
            if let Some(result) = extract_function_parameters(source, Some(child)) {
                return Some(result);
            }
        }
    }

    None
}

fn extract_cpp_function_modifiers(source: &str, node: Node) -> Vec<&'static str> {
    let mut modifiers = Vec::new();

    // Scan all children for storage class specifiers and type qualifiers
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "storage_class_specifier" => {
                if let Ok("static") = child.utf8_text(source.as_bytes()) {
                    modifiers.push("static");
                }
            }
            "virtual_specifier" | "virtual_function_specifier" | "virtual" => {
                modifiers.push("virtual");
            }
            "type_qualifier" => {
                if let Ok("const") = child.utf8_text(source.as_bytes()) {
                    modifiers.push("const");
                }
            }
            _ => {}
        }
    }

    // Check for override, final, noexcept in the full text (these can appear after params)
    if let Ok(full_text) = node.utf8_text(source.as_bytes()) {
        if full_text.contains("override") {
            modifiers.push("override");
        }
        if full_text.contains("final") {
            modifiers.push("final");
        }
        if full_text.contains("noexcept") {
            modifiers.push("noexcept");
        }
        // Check for virtual in function text if not already found
        if full_text.starts_with("virtual") && !modifiers.contains(&"virtual") {
            modifiers.push("virtual");
        }
    }

    modifiers
}

fn build_cpp_function_signature(
    return_type: Option<&str>,
    params: Option<&str>,
    modifiers: &[&str],
) -> Option<String> {
    let mut parts = Vec::new();

    // Add modifiers in typical C++ order
    if modifiers.contains(&"static") {
        parts.push("static");
    }
    if modifiers.contains(&"virtual") {
        parts.push("virtual");
    }

    // Add return type
    if let Some(ret) = return_type {
        parts.push(ret);
    }

    // Add parameters
    if let Some(p) = params {
        parts.push(p);
    }

    // Add trailing modifiers
    if modifiers.contains(&"const") {
        parts.push("const");
    }
    if modifiers.contains(&"noexcept") {
        parts.push("noexcept");
    }
    if modifiers.contains(&"override") {
        parts.push("override");
    }
    if modifiers.contains(&"final") {
        parts.push("final");
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}
