//! Python parser

use tree_sitter::{Node, Parser};

use super::common::lang_python;
use super::python_docstrings::parse_python_docstring;
use crate::indexer::SymbolChunk;

pub(super) fn extract_python_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser.set_language(&lang_python()).ok();

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };

    let mut chunks = Vec::new();
    let root = tree.root_node();

    // Extract function and class definitions
    walk_python_decls(source, root, &mut chunks);

    // Extract module-level assignments (global variables and constants)
    extract_python_module_assignments(source, root, &mut chunks);

    // Extract import statements and add to metadata
    let imports = extract_python_imports(source, root);

    // Always create an imports chunk if we have imports
    // This provides a consistent way to query imports
    if !imports.is_empty() {
        chunks.insert(
            0,
            SymbolChunk {
                symbol_name: Some("__imports__".to_string()),
                kind: "imports".to_string(),
                signature: None,
                docstring: None,
                start_line: 1,
                end_line: find_last_import_line(source, root),
                metadata: Some(serde_json::json!({
                    "imports": imports
                })),
            },
        );
    }

    chunks
}

fn walk_python_decls(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    match node.kind() {
        "function_definition" => {
            extract_python_function(source, node, chunks, false);
        }
        "class_definition" => {
            extract_python_class(source, node, chunks, false);
        }
        "decorated_definition" => {
            // Extract decorators and the definition they decorate
            extract_python_decorated(source, node, chunks);
            return; // Don't recurse into children; we handle them in extract_python_decorated
        }
        _ => {}
    }

    // Recursively walk child nodes
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_python_decls(source, child, chunks);
        }
    }
}

// Extract module-level assignments (global variables and constants)
fn extract_python_module_assignments(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Only extract assignments at module level (not inside classes or functions)
    if is_inside_class(node) || is_inside_function(node) {
        return;
    }

    // Look for assignment nodes
    if node.kind() == "assignment" {
        // Get the left side (variable name)
        if let Some(left) = node.child_by_field_name("left") {
            if left.kind() == "identifier" {
                if let Ok(name) = left.utf8_text(source.as_bytes()) {
                    // Get the right side (value) for context
                    let value = node
                        .child_by_field_name("right")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .map(|s| s.to_string());

                    // Determine if it's a constant (uppercase name convention)
                    let kind =
                        if name.to_uppercase() == name && name.chars().any(|c| c.is_alphabetic()) {
                            "constant"
                        } else {
                            "variable"
                        };

                    let start = node.start_position();
                    let end = node.end_position();

                    chunks.push(SymbolChunk {
                        symbol_name: Some(name.to_string()),
                        kind: kind.to_string(),
                        signature: value,
                        docstring: None,
                        start_line: (start.row + 1) as i32,
                        end_line: (end.row + 1) as i32,
                        metadata: None,
                    });
                }
            }
        }
    }

    // Recursively check children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_python_module_assignments(source, child, chunks);
        }
    }
}

fn is_inside_function(node: Node) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "function_definition" {
            return true;
        }
        current = parent.parent();
    }
    false
}

fn extract_python_function(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    has_decorators: bool,
) {
    // Extract function name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Check if this is an async function
    let is_async = {
        let mut is_async_fn = false;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "async" {
                    is_async_fn = true;
                    break;
                }
            }
        }
        is_async_fn
    };

    // Extract parameters for signature
    let signature = node
        .child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|params| {
            // Also check for return type annotation
            let return_type = node
                .child_by_field_name("return_type")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok());

            let sig = if let Some(ret) = return_type {
                format!("{} -> {}", params, ret)
            } else {
                params.to_string()
            };

            // Prepend async if needed
            if is_async {
                format!("async {}", sig)
            } else {
                sig
            }
        });

    // Extract docstring (first string in body)
    let docstring = extract_python_docstring(source, node);

    // Determine if this is a method by checking if parent is a class
    let kind = if is_inside_class(node) {
        if is_async {
            "async_method"
        } else {
            "method"
        }
    } else if is_async {
        "async_func"
    } else {
        "func"
    };

    // Build metadata object
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert("is_async".to_string(), serde_json::Value::Bool(is_async));
    metadata_obj.insert(
        "has_decorators".to_string(),
        serde_json::Value::Bool(has_decorators),
    );

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
}

fn extract_python_class(
    source: &str,
    node: Node,
    chunks: &mut Vec<SymbolChunk>,
    has_decorators: bool,
) {
    // Extract class name
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract base classes for signature
    let signature = node
        .child_by_field_name("superclasses")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract docstring
    let docstring = extract_python_docstring(source, node);

    // Extract base class names for inheritance tracking
    let base_classes = if let Some(superclasses) = node.child_by_field_name("superclasses") {
        let mut bases = Vec::new();
        for i in 0..superclasses.child_count() {
            if let Some(child) = superclasses.child(i) {
                if child.kind() == "identifier" || child.kind() == "attribute" {
                    if let Ok(base_name) = child.utf8_text(source.as_bytes()) {
                        bases.push(serde_json::Value::String(base_name.to_string()));
                    }
                }
            }
        }
        bases
    } else {
        Vec::new()
    };

    // Build metadata object
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert(
        "has_decorators".to_string(),
        serde_json::Value::Bool(has_decorators),
    );
    metadata_obj.insert(
        "base_classes".to_string(),
        serde_json::Value::Array(base_classes),
    );

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "class".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_python_decorated(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract decorators
    let mut decorators = Vec::new();
    let mut definition_node = None;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "decorator" => {
                    // Extract decorator text (including @)
                    if let Ok(decorator_text) = child.utf8_text(source.as_bytes()) {
                        decorators.push(decorator_text.to_string());
                    }
                }
                "function_definition" | "class_definition" => {
                    definition_node = Some(child);
                }
                _ => {}
            }
        }
    }

    // Extract the decorated definition with decorator information
    if let Some(def_node) = definition_node {
        match def_node.kind() {
            "function_definition" => {
                // Call extract_python_function but include decorators in metadata
                extract_python_function(source, def_node, chunks, true);

                // Update the last chunk's metadata to include decorator names
                if let Some(chunk) = chunks.last_mut() {
                    if let Some(serde_json::Value::Object(ref mut meta)) = chunk.metadata {
                        meta.insert(
                            "decorators".to_string(),
                            serde_json::Value::Array(
                                decorators
                                    .iter()
                                    .map(|d| serde_json::Value::String(d.clone()))
                                    .collect(),
                            ),
                        );
                    }
                }
            }
            "class_definition" => {
                // Call extract_python_class but include decorators in metadata
                extract_python_class(source, def_node, chunks, true);

                // Update the last chunk's metadata to include decorator names
                if let Some(chunk) = chunks.last_mut() {
                    if let Some(serde_json::Value::Object(ref mut meta)) = chunk.metadata {
                        meta.insert(
                            "decorators".to_string(),
                            serde_json::Value::Array(
                                decorators
                                    .iter()
                                    .map(|d| serde_json::Value::String(d.clone()))
                                    .collect(),
                            ),
                        );
                    }
                }

                // Recurse into the class body to extract methods
                for i in 0..def_node.child_count() {
                    if let Some(child) = def_node.child(i) {
                        walk_python_decls(source, child, chunks);
                    }
                }
            }
            _ => {}
        }
    }
}

fn extract_python_docstring(source: &str, node: Node) -> Option<String> {
    // Find the body of the function or class
    let body = node.child_by_field_name("body")?;

    // The docstring is typically the first expression_statement in the body
    for i in 0..body.child_count() {
        if let Some(child) = body.child(i) {
            if child.kind() == "expression_statement" {
                // Check if it contains a string
                if let Some(expr) = child.child(0) {
                    if expr.kind() == "string" {
                        let docstring_raw = expr.utf8_text(source.as_bytes()).ok()?;
                        // Remove quotes and clean up
                        let cleaned = docstring_raw
                            .trim_start_matches("\"\"\"")
                            .trim_start_matches("'''")
                            .trim_start_matches("\"")
                            .trim_start_matches("'")
                            .trim_end_matches("\"\"\"")
                            .trim_end_matches("'''")
                            .trim_end_matches("\"")
                            .trim_end_matches("'")
                            .trim()
                            .to_string();

                        // Parse the docstring into structured format
                        return Some(parse_python_docstring(&cleaned));
                    }
                }
            }
            // Stop after first non-comment node
            if !child.kind().contains("comment") {
                break;
            }
        }
    }
    None
}

// Python docstring parsing

fn is_inside_class(node: Node) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "class_definition" {
            return true;
        }
        current = parent.parent();
    }
    false
}

// Python import extraction

#[derive(Debug, Clone, serde::Serialize)]
struct PythonImport {
    /// Type of import: "standard", "from", "relative", "dynamic"
    import_type: String,
    /// Module name being imported (e.g., "os", "foo.bar")
    module: String,
    /// Specific names imported (for "from" imports)
    names: Vec<String>,
    /// Aliases for imports (e.g., "import numpy as np")
    aliases: Vec<(String, String)>,
    /// Relative import depth (number of dots for relative imports)
    relative_depth: Option<usize>,
    /// Line number where import appears
    line: i32,
    /// Whether this is a wildcard import (from foo import *)
    is_wildcard: bool,
}

fn extract_python_imports(source: &str, root: Node) -> Vec<PythonImport> {
    let mut imports = Vec::new();
    walk_extract_imports(source, root, &mut imports);
    imports
}

fn walk_extract_imports(source: &str, node: Node, imports: &mut Vec<PythonImport>) {
    match node.kind() {
        "import_statement" => {
            extract_standard_import(source, node, imports);
        }
        "import_from_statement" => {
            extract_from_import(source, node, imports);
        }
        "call" => {
            // Check for dynamic imports like __import__() or importlib.import_module()
            extract_dynamic_import(source, node, imports);
        }
        _ => {}
    }

    // Recursively walk children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_extract_imports(source, child, imports);
        }
    }
}

fn extract_standard_import(source: &str, node: Node, imports: &mut Vec<PythonImport>) {
    // Standard import: import foo, import foo.bar, import foo as bar
    let line = (node.start_position().row + 1) as i32;

    // Iterate through children to find dotted_name or aliased_import
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "dotted_name" | "identifier" => {
                    // Simple import: import foo
                    if let Ok(module_name) = child.utf8_text(source.as_bytes()) {
                        imports.push(PythonImport {
                            import_type: "standard".to_string(),
                            module: module_name.to_string(),
                            names: Vec::new(),
                            aliases: Vec::new(),
                            relative_depth: None,
                            line,
                            is_wildcard: false,
                        });
                    }
                }
                "aliased_import" => {
                    // Aliased import: import foo as bar
                    let module_name = child
                        .child_by_field_name("name")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .map(|s| s.to_string());

                    let alias = child
                        .child_by_field_name("alias")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .map(|s| s.to_string());

                    if let Some(module) = module_name {
                        let mut import = PythonImport {
                            import_type: "standard".to_string(),
                            module: module.clone(),
                            names: Vec::new(),
                            aliases: Vec::new(),
                            relative_depth: None,
                            line,
                            is_wildcard: false,
                        };

                        if let Some(a) = alias {
                            import.aliases.push((module, a));
                        }

                        imports.push(import);
                    }
                }
                _ => {}
            }
        }
    }
}

fn extract_from_import(source: &str, node: Node, imports: &mut Vec<PythonImport>) {
    // From import: from foo import bar, from foo import bar as baz, from .foo import bar
    let line = (node.start_position().row + 1) as i32;

    // Extract module name (can be relative_import or dotted_name)
    let module_node = node.child_by_field_name("module_name");

    let (module_name, relative_depth) = if let Some(mod_node) = module_node {
        if mod_node.kind() == "relative_import" {
            // Relative import: from .foo import bar or from .. import bar
            let relative_text = mod_node.utf8_text(source.as_bytes()).ok().unwrap_or("");
            let dots = relative_text.chars().take_while(|&c| c == '.').count();
            let module = relative_text.trim_start_matches('.').to_string();
            (module, Some(dots))
        } else {
            // Absolute import
            let module = mod_node
                .utf8_text(source.as_bytes())
                .ok()
                .map(|s| s.to_string())
                .unwrap_or_default();
            (module, None)
        }
    } else {
        (String::new(), None)
    };

    // Extract imported names
    let mut names = Vec::new();
    let mut aliases = Vec::new();
    let mut is_wildcard = false;

    // The tree-sitter Python grammar might have multiple "name" children for comma-separated imports
    // First try to get the "name" field
    if let Some(name_node) = node.child_by_field_name("name") {
        match name_node.kind() {
            "wildcard_import" => {
                // from foo import *
                is_wildcard = true;
            }
            "dotted_name" | "identifier" => {
                // from foo import bar (single import)
                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                    names.push(name.to_string());
                }
            }
            "aliased_import" => {
                // from foo import bar as baz (single aliased import)
                extract_aliased_import(source, name_node, &mut names, &mut aliases);
            }
            _ => {
                // Complex case: import_list or other structure
                // Walk all children to extract names
                extract_import_list(
                    source,
                    name_node,
                    &mut names,
                    &mut aliases,
                    &mut is_wildcard,
                );
            }
        }
    }

    // Also check all children of the import_from_statement for additional names
    // This handles the case where multiple imports are direct children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            // Skip the module name and "from"/"import" keywords
            if child.kind() == "dotted_name" || child.kind() == "identifier" {
                // Check if this is not the module name
                if let Some(mod_node) = module_node {
                    if child.id() == mod_node.id() {
                        continue; // Skip the module name itself
                    }
                }
                // This is an imported name
                if let Ok(name) = child.utf8_text(source.as_bytes()) {
                    if !names.contains(&name.to_string()) {
                        names.push(name.to_string());
                    }
                }
            } else if child.kind() == "aliased_import" {
                // Additional aliased import
                let name = child
                    .child_by_field_name("name")
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.to_string());
                let alias = child
                    .child_by_field_name("alias")
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.to_string());

                if let Some(n) = name {
                    if !names.contains(&n) {
                        names.push(n.clone());
                        if let Some(a) = alias {
                            aliases.push((n, a));
                        }
                    }
                }
            } else if child.kind() == "wildcard_import" {
                is_wildcard = true;
            }
        }
    }

    let import_type = if relative_depth.is_some() {
        "relative".to_string()
    } else {
        "from".to_string()
    };

    imports.push(PythonImport {
        import_type,
        module: module_name,
        names,
        aliases,
        relative_depth,
        line,
        is_wildcard,
    });
}

fn extract_aliased_import(
    source: &str,
    node: Node,
    names: &mut Vec<String>,
    aliases: &mut Vec<(String, String)>,
) {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let alias = node
        .child_by_field_name("alias")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    if let Some(n) = name {
        names.push(n.clone());
        if let Some(a) = alias {
            aliases.push((n, a));
        }
    }
}

fn extract_import_list(
    source: &str,
    node: Node,
    names: &mut Vec<String>,
    aliases: &mut Vec<(String, String)>,
    is_wildcard: &mut bool,
) {
    // Recursively walk the import list
    // The import_list can contain: identifier, aliased_import, wildcard_import, and comma separators
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "dotted_name" | "identifier" => {
                    if let Ok(name) = child.utf8_text(source.as_bytes()) {
                        names.push(name.to_string());
                    }
                }
                "aliased_import" => {
                    extract_aliased_import(source, child, names, aliases);
                }
                "wildcard_import" => {
                    *is_wildcard = true;
                }
                "," | "(" | ")" => {
                    // Skip punctuation
                }
                _ => {
                    // Recursively check children for nested structures
                    extract_import_list(source, child, names, aliases, is_wildcard);
                }
            }
        }
    }
}

fn extract_dynamic_import(source: &str, node: Node, imports: &mut Vec<PythonImport>) {
    // Check if this is a call to __import__ or importlib.import_module
    let function = node.child_by_field_name("function");

    if let Some(func_node) = function {
        let func_text = func_node.utf8_text(source.as_bytes()).ok().unwrap_or("");

        // Check for __import__('module') or importlib.import_module('module')
        if func_text == "__import__"
            || func_text.ends_with(".import_module")
            || func_text == "import_module"
        {
            // Extract the first argument (module name)
            if let Some(args) = node.child_by_field_name("arguments") {
                // Find the first string argument
                for i in 0..args.child_count() {
                    if let Some(child) = args.child(i) {
                        if child.kind() == "string" {
                            if let Ok(module_str) = child.utf8_text(source.as_bytes()) {
                                // Remove quotes from string
                                let module_name = module_str
                                    .trim_start_matches("\"")
                                    .trim_start_matches("'")
                                    .trim_end_matches("\"")
                                    .trim_end_matches("'")
                                    .to_string();

                                imports.push(PythonImport {
                                    import_type: "dynamic".to_string(),
                                    module: module_name,
                                    names: Vec::new(),
                                    aliases: Vec::new(),
                                    relative_depth: None,
                                    line: (node.start_position().row + 1) as i32,
                                    is_wildcard: false,
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn find_last_import_line(_source: &str, root: Node) -> i32 {
    let mut last_line = 1;
    find_last_import_line_recursive(root, &mut last_line);
    last_line
}

fn find_last_import_line_recursive(node: Node, last_line: &mut i32) {
    match node.kind() {
        "import_statement" | "import_from_statement" => {
            let line = (node.end_position().row + 1) as i32;
            if line > *last_line {
                *last_line = line;
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            find_last_import_line_recursive(child, last_line);
        }
    }
}

// Rust-specific parsing functions
