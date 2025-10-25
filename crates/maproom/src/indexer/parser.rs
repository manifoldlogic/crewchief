use tree_sitter::{Language, Node, Parser};

use super::SymbolChunk;

// Use the safe language providers exposed by the crates
fn lang_typescript() -> Language { tree_sitter_typescript::language_typescript() }
fn lang_tsx() -> Language { tree_sitter_typescript::language_tsx() }
fn lang_javascript() -> Language { tree_sitter_javascript::language() }
fn lang_python() -> Language { tree_sitter_python::language() }

pub fn extract_chunks(source: &str, language: &str) -> Vec<SymbolChunk> {
    match language {
        "md" | "mdx" => extract_markdown_chunks(source),
        "json" => extract_json_chunks(source),
        "yaml" | "yml" => extract_yaml_chunks(source),
        "toml" => extract_toml_chunks(source),
        "py" => extract_python_chunks(source),
        _ => extract_code_chunks(source, language),
    }
}

fn extract_code_chunks(source: &str, language: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    let lang = match language {
        "ts" => lang_typescript(),
        "tsx" => lang_tsx(),
        "js" | "jsx" => lang_javascript(),
        // Rust fallback: we currently don't have tree-sitter-rust wired, so treat as module-only
        "rs" => return Vec::new(),
        _ => return Vec::new(),
    };
    parser.set_language(&lang).ok();
    let tree = match parser.parse(source, None) { Some(t) => t, None => return Vec::new() };
    let root = tree.root_node();
    let mut chunks = Vec::new();

    walk_add_decls(source, root, &mut chunks);
    chunks
}

fn extract_markdown_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut chunks = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    let mut in_code_block = false;
    
    while i < lines.len() {
        let line = lines[i];
        
        // Check for code block boundaries
        if line.trim().starts_with("```") {
            in_code_block = !in_code_block;
            i += 1;
            continue;
        }
        
        // Skip lines inside code blocks
        if in_code_block {
            i += 1;
            continue;
        }
        
        // Check if it's a heading (starts with # and has space)
        if let Some(heading_level) = get_heading_level(line) {
            let heading_text = line.trim_start_matches('#').trim();
            let start_line = (i + 1) as i32;
            
            // Find the end of this section (next heading of same or higher level, or end of file)
            let mut end_idx = i + 1;
            let mut section_code_block = false;
            while end_idx < lines.len() {
                // Track code blocks in the section
                if lines[end_idx].trim().starts_with("```") {
                    section_code_block = !section_code_block;
                }
                // Only check for headings outside of code blocks
                if !section_code_block {
                    if let Some(next_level) = get_heading_level(lines[end_idx]) {
                        if next_level <= heading_level {
                            break;
                        }
                    }
                }
                end_idx += 1;
            }
            
            let end_line = end_idx as i32;
            
            // Determine the kind based on heading level
            let kind = match heading_level {
                1 => "heading_1",
                2 => "heading_2",
                3 => "heading_3",
                4 => "heading_4",
                5 => "heading_5",
                6 => "heading_6",
                _ => "heading",
            };
            
            chunks.push(SymbolChunk {
                symbol_name: Some(heading_text.to_string()),
                kind: kind.to_string(),
                signature: None,
                docstring: None,
                start_line,
                end_line,
                metadata: None,
            });
        }
        
        i += 1;
    }
    
    // If no headings found, return empty to trigger module fallback
    chunks
}

fn get_heading_level(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    
    let mut level = 0;
    for ch in trimmed.chars() {
        if ch == '#' {
            level += 1;
        } else if ch == ' ' {
            // Valid heading must have space after #
            return Some(level);
        } else {
            // Not a valid heading (e.g., "#tag" without space)
            return None;
        }
    }
    None
}

fn extract_json_chunks(source: &str) -> Vec<SymbolChunk> {
    // Parse JSON and create chunks for top-level keys
    // This provides better granularity than treating the whole file as one chunk
    
    // First, try to parse as valid JSON
    let value: serde_json::Value = match serde_json::from_str(source) {
        Ok(v) => v,
        Err(_) => return Vec::new(), // Invalid JSON, fall back to module chunking
    };
    
    let mut chunks = Vec::new();
    
    // Only chunk if it's an object with reasonable number of keys
    if let serde_json::Value::Object(map) = value {
        // For package.json, always chunk scripts, dependencies, devDependencies
        let important_keys = ["scripts", "dependencies", "devDependencies", "config", "exports"];
        
        // If it has important keys or many keys, chunk it
        let has_important = important_keys.iter().any(|k| map.contains_key(*k));
        if !has_important && map.len() <= 3 {
            return Vec::new(); // Too simple, use module fallback
        }
        
        // For each top-level key, create a chunk
        let lines: Vec<&str> = source.lines().collect();
        let mut current_line = 1;
        
        for (key, _value) in map.iter() {
            // Find the line where this key appears
            let key_pattern = format!("\"{}\"", key);
            let mut start_line = current_line;
            let mut end_line = current_line;
            let mut found = false;
            let mut brace_depth = 0;
            let mut in_string = false;
            let mut escape_next = false;
            
            for (i, line) in lines.iter().enumerate().skip(current_line - 1) {
                let line_num = i + 1;
                
                // Look for the key
                if !found && line.contains(&key_pattern) {
                    start_line = line_num;
                    found = true;
                }
                
                if found {
                    // Track brace depth to find the end of this value
                    for ch in line.chars() {
                        if escape_next {
                            escape_next = false;
                            continue;
                        }
                        
                        match ch {
                            '\\' if in_string => escape_next = true,
                            '"' if !in_string => in_string = true,
                            '"' if in_string => in_string = false,
                            '{' | '[' if !in_string => brace_depth += 1,
                            '}' | ']' if !in_string => {
                                brace_depth -= 1;
                                if brace_depth == 0 {
                                    end_line = line_num;
                                    current_line = line_num + 1;
                                    break;
                                }
                            },
                            ',' if !in_string && brace_depth == 0 => {
                                // Simple value ends at comma
                                end_line = line_num;
                                current_line = line_num + 1;
                                break;
                            },
                            _ => {}
                        }
                    }
                    
                    if end_line > start_line {
                        break;
                    }
                }
            }
            
            if found && end_line >= start_line {
                chunks.push(SymbolChunk {
                    symbol_name: Some(key.clone()),
                    kind: "json_key".to_string(),
                    signature: None,
                    docstring: None,
                    start_line: start_line as i32,
                    end_line: end_line as i32,
                    metadata: None,
                });
            }
        }
    }
    
    chunks
}

fn extract_yaml_chunks(source: &str) -> Vec<SymbolChunk> {
    // YAML chunking: create chunks for top-level keys and nested sections
    let mut chunks = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i];
        
        // Skip empty lines and comments
        if line.trim().is_empty() || line.trim().starts_with('#') {
            i += 1;
            continue;
        }
        
        // Check if this is a top-level key (no leading spaces)
        if !line.starts_with(' ') && !line.starts_with('\t') && line.contains(':') {
            // Found a top-level key
            let key = line.split(':').next().unwrap_or("").trim();
            if key.is_empty() {
                i += 1;
                continue;
            }
            
            let start_line = i + 1;
            let mut end_line = start_line;
            
            // Find where this section ends (next top-level key or EOF)
            let mut j = i + 1;
            while j < lines.len() {
                let next_line = lines[j];
                // Check if we hit another top-level key
                if !next_line.starts_with(' ') && !next_line.starts_with('\t') 
                    && !next_line.trim().is_empty() 
                    && !next_line.trim().starts_with('#')
                    && next_line.contains(':') {
                    break;
                }
                end_line = j + 1;
                j += 1;
            }
            
            chunks.push(SymbolChunk {
                symbol_name: Some(key.to_string()),
                kind: "yaml_key".to_string(),
                signature: None,
                docstring: None,
                start_line: start_line as i32,
                end_line: end_line as i32,
                metadata: None,
            });
            
            i = j;
        } else {
            i += 1;
        }
    }
    
    chunks
}

fn extract_toml_chunks(source: &str) -> Vec<SymbolChunk> {
    // TOML chunking: create chunks for sections [section] and top-level keys
    let mut chunks = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        
        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }
        
        // Check for section headers [section] or [[array]]
        if trimmed.starts_with('[') {
            let section_name = trimmed.trim_start_matches('[')
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim_end_matches(']')
                .trim();
            
            if section_name.is_empty() {
                i += 1;
                continue;
            }
            
            let start_line = i + 1;
            let mut end_line = start_line;
            
            // Find where this section ends (next section or EOF)
            let mut j = i + 1;
            while j < lines.len() {
                let next_line = lines[j].trim();
                // Check if we hit another section
                if next_line.starts_with('[') {
                    break;
                }
                end_line = j + 1;
                j += 1;
            }
            
            chunks.push(SymbolChunk {
                symbol_name: Some(section_name.to_string()),
                kind: "toml_section".to_string(),
                signature: None,
                docstring: None,
                start_line: start_line as i32,
                end_line: end_line as i32,
                metadata: None,
            });
            
            i = j;
        } else {
            i += 1;
        }
    }
    
    // If no sections found but file has content, look for top-level keys
    if chunks.is_empty() && !lines.is_empty() {
        // Simple approach: chunk by top-level keys
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();
            
            if !trimmed.is_empty() && !trimmed.starts_with('#') && trimmed.contains('=') {
                let key = trimmed.split('=').next().unwrap_or("").trim();
                if !key.is_empty() {
                    chunks.push(SymbolChunk {
                        symbol_name: Some(key.to_string()),
                        kind: "toml_key".to_string(),
                        signature: None,
                        docstring: None,
                        start_line: (i + 1) as i32,
                        end_line: (i + 1) as i32,
                        metadata: None,
                    });
                }
            }
            i += 1;
        }
    }
    
    chunks
}

fn walk_add_decls(source: &str, node: Node, out: &mut Vec<SymbolChunk>) {
    let kind = node.kind();
    match kind {
        // Functions
        "function_declaration" => {
            let name = node.child_by_field_name("name").and_then(|n| Some(n.utf8_text(source.as_bytes()).ok()?.to_string()));
            push_chunk(source, node, name, "func", out);
        }
        // Classes
        "class_declaration" => {
            let name = node.child_by_field_name("name").and_then(|n| Some(n.utf8_text(source.as_bytes()).ok()?.to_string()));
            push_chunk(source, node, name, "class", out);
        }
        // Variable declarations may contain arrow functions assigned to const
        "lexical_declaration" | "variable_declaration" => {
            // Look for variable declarators with arrow_function
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "variable_declarator" {
                        let name = child.child_by_field_name("name")
                            .and_then(|n| Some(n.utf8_text(source.as_bytes()).ok()?.to_string()));
                        let value = child.child_by_field_name("value");
                        if let Some(v) = value {
                            if v.kind() == "arrow_function" || v.kind() == "function" {
                                push_chunk(source, v, name, "func", out);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_add_decls(source, child, out);
        }
    }
}

fn push_chunk(source: &str, node: Node, name: Option<String>, kind: &str, out: &mut Vec<SymbolChunk>) {
    let start = node.start_position();
    let end = node.end_position();
    let start_line = (start.row + 1) as i32;
    let end_line = (end.row + 1) as i32;
    let _preview = extract_preview(source, start_line, end_line);
    out.push(SymbolChunk {
        symbol_name: name,
        kind: kind.to_string(),
        signature: None,
        docstring: None,
        start_line,
        end_line,
        metadata: None,
    });
}

fn extract_preview(source: &str, start_line: i32, end_line: i32) -> String {
    let start = start_line.max(1) as usize - 1;
    let end = end_line.max(start_line) as usize;
    source.lines().skip(start).take(end - start).take(60).collect::<Vec<_>>().join("\n")
}

// Python-specific parsing functions
fn extract_python_chunks(source: &str) -> Vec<SymbolChunk> {
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
        chunks.insert(0, SymbolChunk {
            symbol_name: Some("__imports__".to_string()),
            kind: "imports".to_string(),
            signature: None,
            docstring: None,
            start_line: 1,
            end_line: find_last_import_line(source, root),
            metadata: Some(serde_json::json!({
                "imports": imports
            })),
        });
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
                    let value = node.child_by_field_name("right")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .map(|s| s.to_string());

                    // Determine if it's a constant (uppercase name convention)
                    let kind = if name.to_uppercase() == name && name.chars().any(|c| c.is_alphabetic()) {
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

fn extract_python_function(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>, has_decorators: bool) {
    // Extract function name
    let name = node.child_by_field_name("name")
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
    let signature = node.child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|params| {
            // Also check for return type annotation
            let return_type = node.child_by_field_name("return_type")
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
    } else {
        if is_async {
            "async_func"
        } else {
            "func"
        }
    };

    // Build metadata object
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert("is_async".to_string(), serde_json::Value::Bool(is_async));
    metadata_obj.insert("has_decorators".to_string(), serde_json::Value::Bool(has_decorators));

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

fn extract_python_class(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>, has_decorators: bool) {
    // Extract class name
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract base classes for signature
    let signature = node.child_by_field_name("superclasses")
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
    metadata_obj.insert("has_decorators".to_string(), serde_json::Value::Bool(has_decorators));
    metadata_obj.insert("base_classes".to_string(), serde_json::Value::Array(base_classes));

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
                                decorators.iter().map(|d| serde_json::Value::String(d.clone())).collect()
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
                                decorators.iter().map(|d| serde_json::Value::String(d.clone())).collect()
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
                        return Some(cleaned);
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
                    let module_name = child.child_by_field_name("name")
                        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                        .map(|s| s.to_string());

                    let alias = child.child_by_field_name("alias")
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
            let module = mod_node.utf8_text(source.as_bytes()).ok()
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
                extract_import_list(source, name_node, &mut names, &mut aliases, &mut is_wildcard);
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
                let name = child.child_by_field_name("name")
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.to_string());
                let alias = child.child_by_field_name("alias")
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

fn extract_aliased_import(source: &str, node: Node, names: &mut Vec<String>, aliases: &mut Vec<(String, String)>) {
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let alias = node.child_by_field_name("alias")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    if let Some(n) = name {
        names.push(n.clone());
        if let Some(a) = alias {
            aliases.push((n, a));
        }
    }
}

fn extract_import_list(source: &str, node: Node, names: &mut Vec<String>, aliases: &mut Vec<(String, String)>, is_wildcard: &mut bool) {
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
        if func_text == "__import__" || func_text.ends_with(".import_module") || func_text == "import_module" {
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


