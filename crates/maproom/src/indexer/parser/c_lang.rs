//! C language parser

use tree_sitter::{Node, Parser};

use super::common::lang_c;
use crate::indexer::SymbolChunk;

/// Main entry point for C chunk extraction.
///
/// Parses C source code using tree-sitter and extracts chunks for functions,
/// structs, enums, typedefs, global variables, and preprocessor includes.
/// Returns an empty vector if parsing fails.
pub(super) fn extract_c_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang_c())
        .expect("Failed to set C language");

    let tree = match parser.parse(source, None) {
        Some(tree) => tree,
        None => {
            tracing::warn!(
                "Failed to parse C source - malformed syntax or grammar incompatibility"
            );
            return Vec::new();
        }
    };

    let mut chunks = Vec::new();
    let mut includes = Vec::new();
    let root = tree.root_node();

    walk_c_decls(source, root, &mut chunks, &mut includes);

    // Create __imports__ chunk if we collected any includes
    if !includes.is_empty() {
        chunks.push(SymbolChunk {
            symbol_name: Some("__imports__".to_string()),
            kind: "imports".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: 1,
            metadata: Some(serde_json::json!(includes)),
        });
    }

    // Log successful parse with chunk summary
    let func_count = chunks.iter().filter(|c| c.kind == "func").count();
    let struct_count = chunks.iter().filter(|c| c.kind == "struct").count();
    let enum_count = chunks.iter().filter(|c| c.kind == "enum").count();
    let typedef_count = chunks.iter().filter(|c| c.kind == "typedef").count();
    let other_count = chunks.len() - func_count - struct_count - enum_count - typedef_count;

    tracing::debug!(
        "Parsed C source: {} chunks extracted ({} functions, {} structs, {} enums, {} typedefs, {} other)",
        chunks.len(),
        func_count,
        struct_count,
        enum_count,
        typedef_count,
        other_count
    );

    chunks
}

/// Walks the AST and dispatches to appropriate extractors based on node kind.
///
/// Recursively traverses all child nodes to find function definitions, declarations,
/// typedefs, preprocessor includes, and standalone struct/enum definitions.
fn walk_c_decls(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    includes: &mut Vec<serde_json::Value>,
) {
    match node.kind() {
        "function_definition" => {
            extract_c_function(source, node, chunks);
        }
        "declaration" => {
            extract_c_declaration(source, node, chunks);
        }
        "type_definition" => {
            extract_c_typedef(source, node, chunks);
        }
        "preproc_include" => {
            collect_c_include(source, node, includes);
        }
        "struct_specifier" => {
            // Standalone struct definition at top level (e.g., `struct User { ... };`)
            // tree-sitter-c parses these as struct_specifier, not declaration
            if let Some(body) = node.child_by_field_name("body") {
                extract_c_struct(source, node, body, node, chunks);
            }
        }
        "enum_specifier" => {
            // Standalone enum definition at top level (e.g., `enum Color { ... };`)
            if let Some(body) = node.child_by_field_name("body") {
                extract_c_enum(source, node, body, node, chunks);
            }
        }
        _ => {}
    }

    // Recursively walk child nodes
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_c_decls(source, child, chunks, includes);
    }
}

/// Extracts a function definition (function_definition node).
///
/// Delegates to extract_c_function_common for shared logic between
/// function definitions and function declarations.
fn extract_c_function(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    extract_c_function_common(source, node, chunks);
}

/// Common function extraction logic for both function_definition and declaration nodes.
///
/// Extracts return type, function name, parameters, storage class, and documentation.
/// Handles C's nested declarator pattern where the function name is buried inside
/// pointer_declarator and function_declarator nodes.
fn extract_c_function_common(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract return type
    let return_type = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract declarator (contains function name and parameters)
    let declarator = match node.child_by_field_name("declarator") {
        Some(decl) => decl,
        None => return,
    };

    // Navigate the declarator tree to find the function name and parameters
    let (name, params) = extract_function_name_and_params(source, declarator);

    let name = match name {
        Some(n) => n,
        None => return,
    };

    // Build signature with return type and parameters
    let signature = match (&return_type, &params) {
        (Some(ret), Some(par)) => Some(format!("{} {}", ret, par)),
        (Some(ret), None) => Some(ret.clone()),
        (None, Some(par)) => Some(par.clone()),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_c_doc_comment(source, node);

    // Check for storage class specifier (static, extern, etc.)
    let storage_class = extract_storage_class(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    if let Some(ref storage) = storage_class {
        metadata_obj.insert(
            "storage_class".to_string(),
            serde_json::Value::String(storage.clone()),
        );
    }
    if let Some(ref ret) = return_type {
        metadata_obj.insert(
            "return_type".to_string(),
            serde_json::Value::String(ret.clone()),
        );
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: Some(name),
        kind: "func".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: if metadata_obj.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(metadata_obj))
        },
    });
}

/// Extracts declarations (multi-purpose handler).
///
/// Handles three cases:
/// 1. Struct/enum definitions with bodies (struct Foo { ... };)
/// 2. Function declarations (int foo(void);)
/// 3. Global variable declarations (int x, y, z;)
///
/// Dispatches to appropriate extractor based on type specifier and declarator kind.
fn extract_c_declaration(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Get the type specifier
    let type_node = match node.child_by_field_name("type") {
        Some(t) => t,
        None => return,
    };

    // Check if this is a struct or enum definition with a body
    match type_node.kind() {
        "struct_specifier" => {
            // Only extract if it has a body (definition, not just declaration)
            if let Some(body) = type_node.child_by_field_name("body") {
                extract_c_struct(source, type_node, body, node, chunks);
                return; // Don't also extract as a variable
            }
        }
        "enum_specifier" => {
            // Only extract if it has a body
            if let Some(body) = type_node.child_by_field_name("body") {
                extract_c_enum(source, type_node, body, node, chunks);
                return; // Don't also extract as a variable
            }
        }
        _ => {}
    }

    // Check if this is a function declaration (has function_declarator but no body)
    if let Some(declarator) = node.child_by_field_name("declarator") {
        if is_function_declarator(&declarator) {
            extract_c_function_declaration(source, node, chunks);
            return;
        }
    }

    // If we get here, it's a variable declaration (or forward declaration)
    extract_c_global_variable(source, node, chunks);
}

/// Checks if a declarator node represents a function declarator.
///
/// Recursively checks for function_declarator nodes, traversing through
/// pointer_declarator wrappers (e.g., int (*foo)(void)).
fn is_function_declarator(node: &Node) -> bool {
    match node.kind() {
        "function_declarator" => true,
        "pointer_declarator" => {
            // Check if it's a pointer to a function
            if let Some(declarator) = node.child_by_field_name("declarator") {
                is_function_declarator(&declarator)
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Extracts a function declaration (forward declaration with no body).
///
/// Delegates to extract_c_function_common for shared logic with function definitions.
fn extract_c_function_declaration(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    extract_c_function_common(source, node, chunks);
}

/// Extracts a struct definition with a body.
///
/// Creates a chunk with the struct name, field count, and documentation.
/// The type_node contains the struct name, body contains field_declaration nodes,
/// and declaration_node is used for position and documentation extraction.
fn extract_c_struct(
    source: &str,
    type_node: Node,
    body: Node,
    declaration_node: Node,
    chunks: &mut Vec<SymbolChunk>,
) {
    // Extract struct name
    let name = type_node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Count fields in the body
    let field_count = count_struct_fields(body);

    // Extract doc comment from the declaration node
    let docstring = extract_c_doc_comment(source, declaration_node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "field_count".to_string(),
        serde_json::Value::Number(serde_json::Number::from(field_count)),
    );

    let start = declaration_node.start_position();
    let end = declaration_node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "struct".to_string(),
        signature: None,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

/// Extracts an enum definition with a body.
///
/// Creates a chunk with the enum name, enumerator count, and documentation.
/// Similar structure to extract_c_struct.
fn extract_c_enum(
    source: &str,
    type_node: Node,
    body: Node,
    declaration_node: Node,
    chunks: &mut Vec<SymbolChunk>,
) {
    // Extract enum name
    let name = type_node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Count enumerators in the body
    let enumerator_count = count_enumerators(body);

    // Extract doc comment from the declaration node
    let docstring = extract_c_doc_comment(source, declaration_node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "enumerator_count".to_string(),
        serde_json::Value::Number(serde_json::Number::from(enumerator_count)),
    );

    let start = declaration_node.start_position();
    let end = declaration_node.end_position();

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

/// Extracts global variable declarations.
///
/// Handles multi-variable declarations (e.g., int x, y, z;) by iterating over
/// init_declarator and pointer_declarator children. Falls back to extracting
/// from the main declarator if no children are found. Avoids duplicate chunks
/// by checking if the variable name was already added.
fn extract_c_global_variable(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract type
    let type_text = node
        .child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract declarator
    let declarator = match node.child_by_field_name("declarator") {
        Some(decl) => decl,
        None => return,
    };

    // Handle multiple declarators (e.g., int a, b, c;)
    // C allows declaring multiple variables of the same type: int x = 1, y = 2, z;
    // tree-sitter parses these as multiple init_declarator siblings
    extract_multi_variable_declarators(source, node, &type_text, chunks);

    // Fallback: try to extract name from the declarator directly
    // This handles single variable declarations that don't use init_declarator
    extract_single_variable_declarator(source, node, declarator, &type_text, chunks);
}

/// Extracts variables from multi-variable declarations (int x, y, z;).
///
/// Iterates over child nodes looking for init_declarator and pointer_declarator nodes,
/// which represent individual variables in a comma-separated declaration.
fn extract_multi_variable_declarators(
    source: &str,
    node: Node,
    type_text: &Option<String>,
    chunks: &mut Vec<SymbolChunk>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "init_declarator" || child.kind() == "pointer_declarator" {
            if let Some(name) = extract_declarator_name(source, child) {
                add_variable_chunk(source, node, &name, type_text, chunks);
            }
        }
    }
}

/// Extracts a single variable from a declarator node (fallback case).
///
/// Used when the declaration doesn't have init_declarator children.
/// Checks for duplicates to avoid adding the same variable twice.
fn extract_single_variable_declarator(
    source: &str,
    node: Node,
    declarator: Node,
    type_text: &Option<String>,
    chunks: &mut Vec<SymbolChunk>,
) {
    if let Some(name) = extract_declarator_name(source, declarator) {
        // Check if we already added this (avoid duplicates)
        if !chunks
            .iter()
            .any(|c| c.symbol_name.as_deref() == Some(&name))
        {
            add_variable_chunk(source, node, &name, type_text, chunks);
        }
    }
}

/// Creates and adds a variable chunk to the chunks vector.
///
/// Extracts documentation, builds metadata with type information,
/// and creates a SymbolChunk for the variable.
fn add_variable_chunk(
    source: &str,
    node: Node,
    name: &str,
    type_text: &Option<String>,
    chunks: &mut Vec<SymbolChunk>,
) {
    let docstring = extract_c_doc_comment(source, node);

    let mut metadata_obj = serde_json::Map::new();
    if let Some(ref typ) = type_text {
        metadata_obj.insert("type".to_string(), serde_json::Value::String(typ.clone()));
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: Some(name.to_string()),
        kind: "variable".to_string(),
        signature: type_text.clone(),
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: if metadata_obj.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(metadata_obj))
        },
    });
}

/// Extracts typedef declarations.
///
/// Handles named typedefs (typedef int MyInt;) and anonymous struct/enum typedefs
/// (typedef struct { ... } MyStruct;). Extracts the typedef name and underlying type.
fn extract_c_typedef(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract the type being aliased
    let type_node = node.child_by_field_name("type");
    let type_text = type_node
        .as_ref()
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract the typedef name from declarator
    let declarator = match node.child_by_field_name("declarator") {
        Some(decl) => decl,
        None => {
            // Handle anonymous struct typedef: typedef struct { ... } Name;
            if let Some(type_node) = type_node {
                if type_node.kind() == "struct_specifier" {
                    // Try to find the identifier after the struct body
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "type_identifier" {
                            if let Ok(name) = child.utf8_text(source.as_bytes()) {
                                let docstring = extract_c_doc_comment(source, node);

                                let mut metadata_obj = serde_json::Map::new();
                                metadata_obj.insert(
                                    "underlying_type".to_string(),
                                    serde_json::Value::String("struct".to_string()),
                                );

                                let start = node.start_position();
                                let end = node.end_position();

                                chunks.push(SymbolChunk {
                                    symbol_name: Some(name.to_string()),
                                    kind: "typedef".to_string(),
                                    signature: Some("struct".to_string()),
                                    docstring,
                                    start_line: (start.row + 1) as i32,
                                    end_line: (end.row + 1) as i32,
                                    metadata: Some(serde_json::Value::Object(metadata_obj)),
                                });
                                return;
                            }
                        }
                    }
                }
            }
            return;
        }
    };

    let name = extract_declarator_name(source, declarator);

    if let Some(name) = name {
        let docstring = extract_c_doc_comment(source, node);

        let mut metadata_obj = serde_json::Map::new();
        if let Some(ref typ) = type_text {
            metadata_obj.insert(
                "underlying_type".to_string(),
                serde_json::Value::String(typ.clone()),
            );
        }

        let start = node.start_position();
        let end = node.end_position();

        chunks.push(SymbolChunk {
            symbol_name: Some(name),
            kind: "typedef".to_string(),
            signature: type_text,
            docstring,
            start_line: (start.row + 1) as i32,
            end_line: (end.row + 1) as i32,
            metadata: if metadata_obj.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(metadata_obj))
            },
        });
    }
}

/// Collects preprocessor include directives.
///
/// Extracts the include path and classifies it as system (<...>) or local ("...").
/// Aggregates includes into the __imports__ chunk metadata.
fn collect_c_include(source: &str, node: Node, includes: &mut Vec<serde_json::Value>) {
    // Extract the path
    let path = node
        .child_by_field_name("path")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    if let Some(path) = path {
        // Distinguish between system includes (<stdio.h>) and local includes ("myheader.h")
        // System includes use angle brackets, local includes use quotes
        let is_system = path.starts_with('<');

        includes.push(serde_json::json!({
            "type": if is_system { "system" } else { "local" },
            "path": path
        }));
    }
}

/// Extracts documentation comments immediately preceding a node.
///
/// Uses a backward-walking algorithm to collect comments from the line before the node,
/// continuing upward through consecutive comment lines until hitting non-comment content.
/// This approach mirrors how developers naturally write documentation comments - placing
/// them directly above the code they describe.
///
/// Handles three comment styles:
/// - Single-line comments: // comment
/// - Block comments: /* comment */
/// - Multi-line block comments: /* ... */ spanning multiple lines
///
/// The backward walk terminates when:
/// - A non-comment, non-blank line is encountered (start of previous declaration)
/// - The start of a block comment is found
/// - We reach the beginning of the file
fn extract_c_doc_comment(source: &str, node: Node) -> Option<String> {
    let start_line = node.start_position().row;
    let lines: Vec<&str> = source.lines().collect();
    let mut doc_lines = Vec::new();

    // Walk backward from the line immediately before the node starts
    // We walk backward (not forward) because comments precede their declarations in C
    for i in (0..start_line).rev() {
        let line = lines.get(i)?.trim();

        // Handle single-line comments (// ...)
        if line.starts_with("//") {
            let comment = line.strip_prefix("//").unwrap_or("").trim();
            doc_lines.insert(0, comment);
        }
        // Handle block comments ending on this line (... */)
        else if line.ends_with("*/") {
            // Multi-line block comment - walk backward to find start
            if let Some(block_comment) = extract_block_comment(&lines, i) {
                // Insert block comment lines at the beginning
                for comment_line in block_comment {
                    doc_lines.insert(0, comment_line);
                }
            }
            break; // Block comment terminates the search
        } else if line.starts_with("/*") {
            // Single-line block comment (/* ... */ on one line)
            let comment = line
                .strip_prefix("/*")
                .unwrap_or("")
                .trim_end_matches("*/")
                .trim();
            doc_lines.insert(0, comment);
            break; // Block comment terminates the search
        } else if !line.is_empty() {
            // Non-comment, non-blank line - stop searching
            // This is likely the end of the previous declaration
            break;
        }
        // Blank lines are skipped (continue searching upward)
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}

/// Extracts a multi-line block comment by walking backward from the closing line.
///
/// Collects all lines from `*/` back to `/*` or `/**`, cleaning comment markers
/// and extracting the text content. Returns None if the opening marker is not found.
fn extract_block_comment<'a>(lines: &'a [&str], end_index: usize) -> Option<Vec<&'a str>> {
    let mut block_lines = Vec::new();
    let mut j = end_index;
    let mut found_start = false;

    // Walk backward from */ to find /* or /**
    while j < lines.len() {
        let block_line = lines[j].trim();
        block_lines.push(block_line);

        if block_line.starts_with("/*") || block_line.starts_with("/**") {
            found_start = true;
            break;
        }

        if j == 0 {
            break;
        }
        j -= 1;
    }

    if !found_start {
        return None;
    }

    // Clean up block comment lines and collect content
    let mut cleaned_lines = Vec::new();
    for block_line in block_lines.iter().rev() {
        let cleaned = block_line
            .trim_start_matches("/*")
            .trim_start_matches("/**")
            .trim_end_matches("*/")
            .trim_start_matches('*')
            .trim();
        if !cleaned.is_empty() {
            cleaned_lines.push(cleaned);
        }
    }

    Some(cleaned_lines)
}

// Helper functions

/// Extracts the function name and parameters from a declarator node.
///
/// Navigates C's nested declarator tree to find the function name buried inside
/// pointer_declarator and function_declarator wrappers. For example:
/// - `int foo(void)` -> function_declarator -> identifier "foo"
/// - `int *foo(void)` -> pointer_declarator -> function_declarator -> identifier "foo"
///
/// Returns (name, parameters) where parameters includes parentheses.
fn extract_function_name_and_params(source: &str, node: Node) -> (Option<String>, Option<String>) {
    match node.kind() {
        "function_declarator" => {
            // Get the nested declarator (contains the actual name)
            let name_node = node.child_by_field_name("declarator");
            let name = name_node.and_then(|n| extract_identifier(source, n));

            // Get parameters
            let params = node
                .child_by_field_name("parameters")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .map(|s| s.to_string());

            (name, params)
        }
        "pointer_declarator" => {
            // Pointer to function - recurse into the declarator
            if let Some(declarator) = node.child_by_field_name("declarator") {
                extract_function_name_and_params(source, declarator)
            } else {
                (None, None)
            }
        }
        "identifier" => {
            let name = node
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());
            (name, None)
        }
        _ => (None, None),
    }
}

/// Recursively extracts an identifier from a declarator node.
///
/// Traverses through pointer_declarator and function_declarator wrappers to find
/// the underlying identifier node. Used when we know we're looking for a simple name,
/// not parameters.
fn extract_identifier(source: &str, node: Node) -> Option<String> {
    match node.kind() {
        "identifier" => node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|s| s.to_string()),
        "pointer_declarator" => {
            let declarator = node.child_by_field_name("declarator")?;
            extract_identifier(source, declarator)
        }
        "function_declarator" => {
            let declarator = node.child_by_field_name("declarator")?;
            extract_identifier(source, declarator)
        }
        _ => None,
    }
}

/// Extracts the name from a declarator node.
///
/// Handles various declarator types: identifier, init_declarator, pointer_declarator,
/// array_declarator, and type_identifier. Recursively traverses the declarator chain
/// to find the actual name. C's declarator syntax allows deeply nested patterns:
/// - `int x` -> identifier "x"
/// - `int *x` -> pointer_declarator -> identifier "x"
/// - `int x = 5` -> init_declarator -> identifier "x"
/// - `int *x[10]` -> array_declarator -> pointer_declarator -> identifier "x"
fn extract_declarator_name(source: &str, node: Node) -> Option<String> {
    match node.kind() {
        "identifier" => node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|s| s.to_string()),
        "init_declarator" => {
            let declarator = node.child_by_field_name("declarator")?;
            extract_declarator_name(source, declarator)
        }
        "pointer_declarator" => {
            let declarator = node.child_by_field_name("declarator")?;
            extract_declarator_name(source, declarator)
        }
        "array_declarator" => {
            let declarator = node.child_by_field_name("declarator")?;
            extract_declarator_name(source, declarator)
        }
        "type_identifier" => node
            .utf8_text(source.as_bytes())
            .ok()
            .map(|s| s.to_string()),
        _ => None,
    }
}

/// Extracts the storage class specifier from a declaration node.
///
/// Searches child nodes for storage_class_specifier (static, extern, register, auto).
/// Returns the storage class keyword if found.
fn extract_storage_class(source: &str, node: Node) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "storage_class_specifier" {
            return child
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string());
        }
    }
    None
}

/// Counts the number of fields in a struct body.
///
/// Iterates through child nodes and counts field_declaration nodes.
fn count_struct_fields(body: Node) -> usize {
    let mut count = 0;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "field_declaration" {
            count += 1;
        }
    }
    count
}

/// Counts the number of enumerators in an enum body.
///
/// Iterates through child nodes and counts enumerator nodes.
fn count_enumerators(body: Node) -> usize {
    let mut count = 0;
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "enumerator" {
            count += 1;
        }
    }
    count
}
