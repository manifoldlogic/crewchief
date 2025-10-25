use tree_sitter::{Language, Node, Parser};

use super::SymbolChunk;

// Use the safe language providers exposed by the crates
fn lang_typescript() -> Language { tree_sitter_typescript::language_typescript() }
fn lang_tsx() -> Language { tree_sitter_typescript::language_tsx() }
fn lang_javascript() -> Language { tree_sitter_javascript::language() }
fn lang_python() -> Language { tree_sitter_python::language() }
fn lang_rust() -> Language { tree_sitter_rust::language() }
fn lang_go() -> Language { tree_sitter_go::language() }

pub fn extract_chunks(source: &str, language: &str) -> Vec<SymbolChunk> {
    match language {
        "md" | "mdx" => extract_markdown_chunks(source),
        "json" => extract_json_chunks(source),
        "yaml" | "yml" => extract_yaml_chunks(source),
        "toml" => extract_toml_chunks(source),
        "py" => extract_python_chunks(source),
        "rs" => extract_rust_chunks(source),
        "go" => extract_go_chunks(source),
        "gomod" => extract_gomod_chunks(source),
        _ => extract_code_chunks(source, language),
    }
}

fn extract_code_chunks(source: &str, language: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    let lang = match language {
        "ts" => lang_typescript(),
        "tsx" => lang_tsx(),
        "js" | "jsx" => lang_javascript(),
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

#[derive(Debug, PartialEq)]
enum DocstringFormat {
    Google,
    NumPy,
    ReStructuredText,
    Plain,
}

/// Detects the docstring format by examining content
fn detect_docstring_format(docstring: &str) -> DocstringFormat {
    let lines: Vec<&str> = docstring.lines().collect();

    // Check for reStructuredText field lists (:param:, :returns:, :type:, :raises:)
    if docstring.contains(":param ") || docstring.contains(":returns:") ||
       docstring.contains(":type ") || docstring.contains(":raises ") ||
       docstring.contains(":rtype:") {
        return DocstringFormat::ReStructuredText;
    }

    // Check for NumPy style (section headers with underlines)
    for i in 0..lines.len().saturating_sub(1) {
        let line = lines[i].trim();
        let next_line = lines[i + 1].trim();

        // NumPy uses underlines (--- or ===) under section headers
        if !line.is_empty() && (next_line.chars().all(|c| c == '-') || next_line.chars().all(|c| c == '=')) {
            if line.eq_ignore_ascii_case("parameters") ||
               line.eq_ignore_ascii_case("returns") ||
               line.eq_ignore_ascii_case("raises") ||
               line.eq_ignore_ascii_case("yields") ||
               line.eq_ignore_ascii_case("notes") ||
               line.eq_ignore_ascii_case("attributes") {
                return DocstringFormat::NumPy;
            }
        }
    }

    // Check for Google style (sections ending with colon)
    for line in &lines {
        let trimmed = line.trim();
        if trimmed == "Args:" || trimmed == "Arguments:" ||
           trimmed == "Returns:" || trimmed == "Return:" ||
           trimmed == "Raises:" || trimmed == "Yields:" ||
           trimmed == "Examples:" || trimmed == "Example:" ||
           trimmed == "Note:" || trimmed == "Notes:" ||
           trimmed == "Warning:" || trimmed == "Warnings:" ||
           trimmed == "Attributes:" {
            return DocstringFormat::Google;
        }
    }

    DocstringFormat::Plain
}

/// Parses a Python docstring and returns a normalized, structured format
fn parse_python_docstring(docstring: &str) -> String {
    if docstring.is_empty() {
        return String::new();
    }

    let format = detect_docstring_format(docstring);

    match format {
        DocstringFormat::Google => parse_google_docstring(docstring),
        DocstringFormat::NumPy => parse_numpy_docstring(docstring),
        DocstringFormat::ReStructuredText => parse_rst_docstring(docstring),
        DocstringFormat::Plain => docstring.to_string(),
    }
}

/// Parses Google-style docstrings (Args:, Returns:, Raises:, etc.)
fn parse_google_docstring(docstring: &str) -> String {
    let lines: Vec<&str> = docstring.lines().collect();
    let mut result = String::new();
    let mut current_section = String::new();
    let mut i = 0;

    // Extract brief description (everything before first section)
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed == "Args:" || trimmed == "Arguments:" ||
           trimmed == "Returns:" || trimmed == "Return:" ||
           trimmed == "Raises:" || trimmed == "Yields:" ||
           trimmed == "Examples:" || trimmed == "Example:" ||
           trimmed == "Note:" || trimmed == "Notes:" ||
           trimmed == "Warning:" || trimmed == "Warnings:" ||
           trimmed == "Attributes:" {
            break;
        }
        if !trimmed.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(trimmed);
        }
        i += 1;
    }

    // Parse sections
    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Check if this is a section header
        if trimmed.ends_with(':') && !trimmed.contains(' ') {
            // Normalize section names
            let section_name = if trimmed == "Args:" || trimmed == "Arguments:" {
                "Parameters"
            } else if trimmed == "Return:" {
                "Returns"
            } else if trimmed == "Example:" {
                "Examples"
            } else if trimmed == "Note:" {
                "Notes"
            } else if trimmed == "Warning:" {
                "Warnings"
            } else {
                trimmed.trim_end_matches(':')
            };

            current_section = section_name.to_string();
            result.push_str("\n\n");
            result.push_str(section_name);
            result.push_str(":\n");
            i += 1;

            // Extract section content
            while i < lines.len() {
                let line = lines[i];
                let trimmed = line.trim();

                // Check if we've hit the next section
                if trimmed.ends_with(':') && !trimmed.contains(' ') {
                    break;
                }

                // Handle parameter lines (usually indented)
                if !trimmed.is_empty() {
                    // For list-like sections, add "- " prefix to indented items
                    let is_list_section = current_section == "Parameters" ||
                                         current_section == "Attributes" ||
                                         current_section == "Raises" ||
                                         current_section == "Yields";

                    if is_list_section && line.starts_with("    ") {
                        result.push_str("- ");
                        result.push_str(trimmed);
                        result.push('\n');
                    } else if !trimmed.is_empty() {
                        result.push_str(line);
                        result.push('\n');
                    }
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    result.trim().to_string()
}

/// Parses NumPy-style docstrings (Parameters, Returns, etc. with underlines)
fn parse_numpy_docstring(docstring: &str) -> String {
    let lines: Vec<&str> = docstring.lines().collect();
    let mut result = String::new();
    let mut current_section = String::new();
    let mut i = 0;

    // Extract brief description (everything before first section)
    while i < lines.len() {
        if i + 1 < lines.len() {
            let line = lines[i].trim();
            let next_line = lines[i + 1].trim();

            // Check if next line is an underline
            if !line.is_empty() && (next_line.chars().all(|c| c == '-') || next_line.chars().all(|c| c == '=')) {
                if line.eq_ignore_ascii_case("parameters") ||
                   line.eq_ignore_ascii_case("returns") ||
                   line.eq_ignore_ascii_case("raises") ||
                   line.eq_ignore_ascii_case("yields") ||
                   line.eq_ignore_ascii_case("notes") ||
                   line.eq_ignore_ascii_case("examples") ||
                   line.eq_ignore_ascii_case("attributes") {
                    break;
                }
            }
        }

        let trimmed = lines[i].trim();
        if !trimmed.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(trimmed);
        }
        i += 1;
    }

    // Parse sections
    while i < lines.len() {
        if i + 1 < lines.len() {
            let line = lines[i].trim();
            let next_line = lines[i + 1].trim();

            // Check if this is a section header (has underline)
            if !line.is_empty() && (next_line.chars().all(|c| c == '-') || next_line.chars().all(|c| c == '=')) {
                current_section = line.to_string();
                result.push_str("\n\n");
                result.push_str(line);
                result.push_str(":\n");
                i += 2; // Skip header and underline

                // Extract section content
                while i < lines.len() {
                    if i + 1 < lines.len() {
                        let content_line = lines[i].trim();
                        let next = lines[i + 1].trim();

                        // Check if we've hit the next section
                        if !content_line.is_empty() && (next.chars().all(|c| c == '-') || next.chars().all(|c| c == '=')) {
                            break;
                        }
                    }

                    let content_line = lines[i];
                    let trimmed = content_line.trim();

                    // Handle parameter lines (format: "param_name : type" followed by description)
                    if !trimmed.is_empty() {
                        if current_section.eq_ignore_ascii_case("parameters") ||
                           current_section.eq_ignore_ascii_case("attributes") {
                            if !content_line.starts_with("    ") && trimmed.contains(" : ") {
                                result.push_str("- ");
                            }
                        }
                        result.push_str(trimmed);
                        result.push('\n');
                    }
                    i += 1;
                }
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    result.trim().to_string()
}

/// Parses reStructuredText-style docstrings (:param:, :returns:, etc.)
fn parse_rst_docstring(docstring: &str) -> String {
    let lines: Vec<&str> = docstring.lines().collect();
    let mut result = String::new();
    let mut params = Vec::new();
    let mut returns = Vec::new();
    let mut raises = Vec::new();
    let mut types = std::collections::HashMap::new();
    let mut return_type = None;
    let mut i = 0;

    // Extract brief description (everything before first field)
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with(':') {
            break;
        }
        if !trimmed.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(trimmed);
        }
        i += 1;
    }

    // Parse field lists
    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with(":param ") {
            // Format: :param name: description
            if let Some(rest) = line.strip_prefix(":param ") {
                if let Some(colon_pos) = rest.find(':') {
                    let param_name = rest[..colon_pos].trim();
                    let description = rest[colon_pos + 1..].trim();

                    // Collect multi-line descriptions
                    let mut full_desc = description.to_string();
                    i += 1;
                    while i < lines.len() {
                        let next_line = lines[i].trim();
                        if next_line.starts_with(':') || next_line.is_empty() {
                            break;
                        }
                        full_desc.push(' ');
                        full_desc.push_str(next_line);
                        i += 1;
                    }
                    params.push((param_name.to_string(), full_desc));
                    continue;
                }
            }
        } else if line.starts_with(":type ") {
            // Format: :type name: type
            if let Some(rest) = line.strip_prefix(":type ") {
                if let Some(colon_pos) = rest.find(':') {
                    let param_name = rest[..colon_pos].trim();
                    let type_info = rest[colon_pos + 1..].trim();
                    types.insert(param_name.to_string(), type_info.to_string());
                }
            }
        } else if line.starts_with(":returns:") || line.starts_with(":return:") {
            // Format: :returns: description
            let prefix = if line.starts_with(":returns:") { ":returns:" } else { ":return:" };
            let description = line.strip_prefix(prefix).unwrap_or("").trim();

            // Collect multi-line descriptions
            let mut full_desc = description.to_string();
            i += 1;
            while i < lines.len() {
                let next_line = lines[i].trim();
                if next_line.starts_with(':') || next_line.is_empty() {
                    break;
                }
                if !full_desc.is_empty() {
                    full_desc.push(' ');
                }
                full_desc.push_str(next_line);
                i += 1;
            }
            returns.push(full_desc);
            continue;
        } else if line.starts_with(":rtype:") {
            // Format: :rtype: type
            let type_info = line.strip_prefix(":rtype:").unwrap_or("").trim();
            return_type = Some(type_info.to_string());
        } else if line.starts_with(":raises ") {
            // Format: :raises ExceptionType: description
            if let Some(rest) = line.strip_prefix(":raises ") {
                if let Some(colon_pos) = rest.find(':') {
                    let exc_type = rest[..colon_pos].trim();
                    let description = rest[colon_pos + 1..].trim();

                    // Collect multi-line descriptions
                    let mut full_desc = description.to_string();
                    i += 1;
                    while i < lines.len() {
                        let next_line = lines[i].trim();
                        if next_line.starts_with(':') || next_line.is_empty() {
                            break;
                        }
                        full_desc.push(' ');
                        full_desc.push_str(next_line);
                        i += 1;
                    }
                    raises.push((exc_type.to_string(), full_desc));
                    continue;
                }
            }
        }
        i += 1;
    }

    // Build normalized output
    if !params.is_empty() {
        result.push_str("\n\nParameters:\n");
        for (name, desc) in params {
            result.push_str("- ");
            result.push_str(&name);
            if let Some(type_info) = types.get(&name) {
                result.push_str(" (");
                result.push_str(type_info);
                result.push(')');
            }
            result.push_str(": ");
            result.push_str(&desc);
            result.push('\n');
        }
    }

    if !returns.is_empty() {
        result.push_str("\nReturns:\n");
        for ret in returns {
            if let Some(rtype) = &return_type {
                result.push_str("- ");
                result.push_str(rtype);
                result.push_str(": ");
            }
            result.push_str(&ret);
            result.push('\n');
        }
    }

    if !raises.is_empty() {
        result.push_str("\nRaises:\n");
        for (exc_type, desc) in raises {
            result.push_str("- ");
            result.push_str(&exc_type);
            result.push_str(": ");
            result.push_str(&desc);
            result.push('\n');
        }
    }

    result.trim().to_string()
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

// Rust-specific parsing functions
fn extract_rust_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser.set_language(&lang_rust()).ok();

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };

    let mut chunks = Vec::new();
    let root = tree.root_node();

    // Extract top-level declarations
    walk_rust_decls(source, root, &mut chunks);

    chunks
}

fn walk_rust_decls(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    match node.kind() {
        "function_item" => {
            extract_rust_function(source, node, chunks);
        }
        "struct_item" => {
            extract_rust_struct(source, node, chunks);
        }
        "enum_item" => {
            extract_rust_enum(source, node, chunks);
        }
        "trait_item" => {
            extract_rust_trait(source, node, chunks);
        }
        "impl_item" => {
            extract_rust_impl(source, node, chunks);
        }
        "mod_item" => {
            extract_rust_module(source, node, chunks);
        }
        "const_item" | "static_item" => {
            extract_rust_constant(source, node, chunks);
        }
        "macro_definition" => {
            extract_rust_macro(source, node, chunks);
        }
        "use_declaration" => {
            extract_rust_use_statement(source, node, chunks);
        }
        _ => {}
    }

    // Recursively walk child nodes
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_rust_decls(source, child, chunks);
        }
    }
}

fn extract_rust_function(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract function name
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility modifier (pub, pub(crate), etc.)
    let visibility = extract_rust_visibility(source, node);

    // Extract function modifiers (async, const, unsafe)
    let modifiers = extract_rust_function_modifiers(source, node);
    let is_async = modifiers.contains(&"async");
    let is_const = modifiers.contains(&"const");
    let is_unsafe = modifiers.contains(&"unsafe");

    // Extract generic type parameters
    let type_params = node.child_by_field_name("type_parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract where clause
    let where_clause = extract_rust_where_clause(source, node);

    // Extract parameters for signature
    let params = node.child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract return type
    let return_type = node.child_by_field_name("return_type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.trim_start_matches("->").trim().to_string());

    // Build signature
    let signature = build_rust_function_signature(
        visibility.as_deref(),
        is_async,
        is_const,
        is_unsafe,
        type_params.as_deref(),
        params.as_deref(),
        return_type.as_deref(),
        where_clause.as_deref(),
    );

    // Extract doc comment (lines starting with /// or //! before the function)
    let docstring = extract_rust_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert("visibility".to_string(), serde_json::Value::String(visibility.unwrap_or_else(|| "private".to_string())));
    metadata_obj.insert("is_async".to_string(), serde_json::Value::Bool(is_async));
    metadata_obj.insert("is_const".to_string(), serde_json::Value::Bool(is_const));
    metadata_obj.insert("is_unsafe".to_string(), serde_json::Value::Bool(is_unsafe));

    // Add generic parameters to metadata if present
    if let Some(ref generics) = type_params {
        metadata_obj.insert("generics".to_string(), serde_json::Value::String(generics.clone()));
    }

    // Add where clause to metadata if present
    if let Some(ref wc) = where_clause {
        metadata_obj.insert("where_clause".to_string(), serde_json::Value::String(wc.clone()));
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "func".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_rust_struct(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract struct name
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility
    let visibility = extract_rust_visibility(source, node);

    // Extract type parameters (generics)
    let type_params = node.child_by_field_name("type_parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract where clause
    let where_clause = extract_rust_where_clause(source, node);

    // Build signature
    let signature = match (&type_params, &where_clause) {
        (Some(params), Some(wc)) => Some(format!("{} {}", params, wc)),
        (Some(params), None) => Some(params.clone()),
        (None, Some(wc)) => Some(wc.clone()),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert("visibility".to_string(), serde_json::Value::String(visibility.unwrap_or_else(|| "private".to_string())));

    // Add generic parameters to metadata if present
    if let Some(ref generics) = type_params {
        metadata_obj.insert("generics".to_string(), serde_json::Value::String(generics.clone()));
    }

    // Add where clause to metadata if present
    if let Some(ref wc) = where_clause {
        metadata_obj.insert("where_clause".to_string(), serde_json::Value::String(wc.clone()));
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "struct".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_rust_enum(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract enum name
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility
    let visibility = extract_rust_visibility(source, node);

    // Extract type parameters (generics)
    let type_params = node.child_by_field_name("type_parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract where clause
    let where_clause = extract_rust_where_clause(source, node);

    // Build signature
    let signature = match (&type_params, &where_clause) {
        (Some(params), Some(wc)) => Some(format!("{} {}", params, wc)),
        (Some(params), None) => Some(params.clone()),
        (None, Some(wc)) => Some(wc.clone()),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert("visibility".to_string(), serde_json::Value::String(visibility.unwrap_or_else(|| "private".to_string())));

    // Add generic parameters to metadata if present
    if let Some(ref generics) = type_params {
        metadata_obj.insert("generics".to_string(), serde_json::Value::String(generics.clone()));
    }

    // Add where clause to metadata if present
    if let Some(ref wc) = where_clause {
        metadata_obj.insert("where_clause".to_string(), serde_json::Value::String(wc.clone()));
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "enum".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_rust_trait(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract trait name
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility
    let visibility = extract_rust_visibility(source, node);

    // Extract type parameters (generics)
    let type_params = node.child_by_field_name("type_parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract where clause
    let where_clause = extract_rust_where_clause(source, node);

    // Build signature
    let signature = match (&type_params, &where_clause) {
        (Some(params), Some(wc)) => Some(format!("{} {}", params, wc)),
        (Some(params), None) => Some(params.clone()),
        (None, Some(wc)) => Some(wc.clone()),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert("visibility".to_string(), serde_json::Value::String(visibility.unwrap_or_else(|| "private".to_string())));

    // Add generic parameters to metadata if present
    if let Some(ref generics) = type_params {
        metadata_obj.insert("generics".to_string(), serde_json::Value::String(generics.clone()));
    }

    // Add where clause to metadata if present
    if let Some(ref wc) = where_clause {
        metadata_obj.insert("where_clause".to_string(), serde_json::Value::String(wc.clone()));
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "trait".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_rust_impl(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract the type being implemented for
    let type_node = node.child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract the trait being implemented (if any)
    let trait_node = node.child_by_field_name("trait")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Build name and signature
    let (name, signature) = if let Some(trait_name) = trait_node {
        let type_name = type_node.unwrap_or_else(|| "Unknown".to_string());
        (
            Some(format!("impl {} for {}", trait_name, type_name)),
            Some(format!("{} for {}", trait_name, type_name)),
        )
    } else if let Some(type_name) = type_node {
        (
            Some(format!("impl {}", type_name)),
            Some(type_name),
        )
    } else {
        (None, None)
    };

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "impl".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: None,
    });
}

fn extract_rust_module(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract module name
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility
    let visibility = extract_rust_visibility(source, node);

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert("visibility".to_string(), serde_json::Value::String(visibility.unwrap_or_else(|| "private".to_string())));

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "module".to_string(),
        signature: None,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_rust_constant(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract constant name
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract visibility
    let visibility = extract_rust_visibility(source, node);

    // Extract type annotation
    let type_annotation = node.child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract value for signature
    let value = node.child_by_field_name("value")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Build signature
    let signature = if let Some(ty) = type_annotation {
        Some(format!(": {}", ty))
    } else {
        None
    };

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    // Determine kind (const vs static)
    let kind = if node.kind() == "static_item" {
        "static"
    } else {
        "constant"
    };

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    metadata_obj.insert("visibility".to_string(), serde_json::Value::String(visibility.unwrap_or_else(|| "private".to_string())));
    if let Some(v) = value {
        metadata_obj.insert("value".to_string(), serde_json::Value::String(v));
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
}

fn extract_rust_macro(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract macro name
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract doc comment
    let docstring = extract_rust_doc_comment(source, node);

    let start = node.start_position();
    let end = node.end_position();

    // Mark macros as opaque blocks for now (as per ticket requirement)
    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "macro".to_string(),
        signature: Some("macro_rules!".to_string()),
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: None,
    });
}

// Helper functions for Rust parsing

fn extract_rust_visibility(source: &str, node: Node) -> Option<String> {
    // Look for visibility_modifier child
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "visibility_modifier" {
                return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
            }
        }
    }
    None
}

fn extract_rust_function_modifiers(source: &str, node: Node) -> Vec<&'static str> {
    let mut modifiers = Vec::new();

    // Look for function_modifiers child
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "function_modifiers" {
                // Extract each modifier from the function_modifiers node
                for j in 0..child.child_count() {
                    if let Some(modifier) = child.child(j) {
                        match modifier.kind() {
                            "async" => modifiers.push("async"),
                            "const" => modifiers.push("const"),
                            "unsafe" => modifiers.push("unsafe"),
                            "default" => modifiers.push("default"),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    modifiers
}

fn build_rust_function_signature(
    visibility: Option<&str>,
    is_async: bool,
    is_const: bool,
    is_unsafe: bool,
    type_params: Option<&str>,
    params: Option<&str>,
    return_type: Option<&str>,
    where_clause: Option<&str>,
) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(vis) = visibility {
        parts.push(vis.to_string());
    }

    if is_const {
        parts.push("const".to_string());
    }

    if is_async {
        parts.push("async".to_string());
    }

    if is_unsafe {
        parts.push("unsafe".to_string());
    }

    parts.push("fn".to_string());

    if let Some(tp) = type_params {
        parts.push(tp.to_string());
    }

    if let Some(p) = params {
        parts.push(p.to_string());
    }

    if let Some(ret) = return_type {
        parts.push(format!("-> {}", ret));
    }

    if let Some(wc) = where_clause {
        parts.push(wc.to_string());
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn extract_rust_where_clause(source: &str, node: Node) -> Option<String> {
    // Look for where_clause child node
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "where_clause" {
                return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
            }
        }
    }
    None
}

fn extract_rust_use_statement(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract the full use statement text
    let use_text = node.utf8_text(source.as_bytes()).ok()
        .map(|s| s.to_string());

    // Try to extract a meaningful name for the use statement
    // For simple cases like "use std::collections::HashMap;", extract "HashMap"
    // For complex cases like "use std::io::{Read, Write};", extract the whole path
    let name = if let Some(ref text) = use_text {
        // Remove "use " prefix and ";" suffix
        let trimmed = text.trim_start_matches("use").trim().trim_end_matches(";").trim();

        // Handle different use statement patterns:
        // - use foo::bar; -> "foo::bar"
        // - use foo::bar as baz; -> "foo::bar as baz"
        // - use foo::{bar, baz}; -> "foo::{bar, baz}"
        // - use super::*; -> "super::*"
        Some(trimmed.to_string())
    } else {
        None
    };

    // Extract visibility
    let visibility = extract_rust_visibility(source, node);

    // Build metadata
    let mut metadata_obj = serde_json::Map::new();
    if let Some(vis) = visibility {
        metadata_obj.insert("visibility".to_string(), serde_json::Value::String(vis));
    }

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "use".to_string(),
        signature: use_text,
        docstring: None,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: Some(serde_json::Value::Object(metadata_obj)),
    });
}

fn extract_rust_doc_comment(source: &str, node: Node) -> Option<String> {
    // Look for line_comment or block_comment nodes before the declaration
    let start_line = node.start_position().row;
    let lines: Vec<&str> = source.lines().collect();

    let mut doc_lines = Vec::new();
    let mut scan_line = start_line.saturating_sub(1);

    // Scan backwards from the node to find doc comments
    while scan_line > 0 {
        let line = lines.get(scan_line)?;
        let trimmed = line.trim();

        if trimmed.starts_with("///") {
            // Doc comment line
            let comment_text = trimmed.trim_start_matches("///").trim();
            doc_lines.insert(0, comment_text.to_string());
            scan_line = scan_line.saturating_sub(1);
        } else if trimmed.starts_with("//!") {
            // Inner doc comment
            let comment_text = trimmed.trim_start_matches("//!").trim();
            doc_lines.insert(0, comment_text.to_string());
            scan_line = scan_line.saturating_sub(1);
        } else if trimmed.is_empty() {
            // Empty line, continue scanning
            scan_line = scan_line.saturating_sub(1);
        } else if trimmed.starts_with("#[") || trimmed.starts_with("#![") {
            // Attribute, skip
            scan_line = scan_line.saturating_sub(1);
        } else {
            // Non-comment, non-empty line - stop scanning
            break;
        }
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}

// Go-specific parsing functions
fn extract_go_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut parser = Parser::new();
    parser.set_language(&lang_go()).ok();

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };

    let mut chunks = Vec::new();
    let root = tree.root_node();

    // Extract top-level declarations
    walk_go_decls(source, root, &mut chunks);

    chunks
}

fn walk_go_decls(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    match node.kind() {
        "function_declaration" => {
            extract_go_function(source, node, chunks);
        }
        "method_declaration" => {
            extract_go_method(source, node, chunks);
        }
        "type_declaration" => {
            extract_go_type_declaration(source, node, chunks);
        }
        "const_declaration" => {
            extract_go_const_declaration(source, node, chunks);
        }
        "var_declaration" => {
            extract_go_var_declaration(source, node, chunks);
        }
        "package_clause" => {
            extract_go_package(source, node, chunks);
        }
        "import_declaration" => {
            extract_go_import(source, node, chunks);
        }
        _ => {}
    }

    // Recursively walk child nodes
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk_go_decls(source, child, chunks);
        }
    }
}

fn extract_go_function(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract function name
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract parameters
    let params = node.child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract return type
    let result = node.child_by_field_name("result")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Build signature
    let signature = match (&params, &result) {
        (Some(p), Some(r)) => Some(format!("{} {}", p, r)),
        (Some(p), None) => Some(p.clone()),
        (None, Some(r)) => Some(r.clone()),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_go_doc_comment(source, node);

    // Detect goroutines and channels in the function body
    let (has_goroutines, has_channels) = detect_go_concurrency(node);

    let start = node.start_position();
    let end = node.end_position();

    // Build metadata with goroutine/channel flags and visibility
    let metadata = {
        let mut meta = serde_json::Map::new();

        // Add visibility based on function name
        if let Some(ref func_name) = name {
            meta.insert("visibility".to_string(), serde_json::json!(go_visibility(func_name)));
        }

        if has_goroutines {
            meta.insert("has_goroutines".to_string(), serde_json::json!(true));
        }
        if has_channels {
            meta.insert("has_channels".to_string(), serde_json::json!(true));
        }

        if meta.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(meta))
        }
    };

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "func".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata,
    });
}

fn extract_go_method(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract method name
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract receiver (e.g., "(r *MyType)")
    let receiver = node.child_by_field_name("receiver")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract parameters
    let params = node.child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract return type
    let result = node.child_by_field_name("result")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Build signature with receiver
    let signature = match (&receiver, &params, &result) {
        (Some(r), Some(p), Some(ret)) => Some(format!("{} {} {}", r, p, ret)),
        (Some(r), Some(p), None) => Some(format!("{} {}", r, p)),
        (Some(r), None, Some(ret)) => Some(format!("{} {}", r, ret)),
        (Some(r), None, None) => Some(r.clone()),
        (None, Some(p), Some(ret)) => Some(format!("{} {}", p, ret)),
        (None, Some(p), None) => Some(p.clone()),
        (None, None, Some(ret)) => Some(ret.clone()),
        (None, None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_go_doc_comment(source, node);

    // Detect goroutines and channels in the method body
    let (has_goroutines, has_channels) = detect_go_concurrency(node);

    // Parse receiver to get type and pointer/value information
    let (receiver_type_name, receiver_type) = if let Some(ref r) = receiver {
        parse_go_receiver(r)
    } else {
        (None, None)
    };

    let start = node.start_position();
    let end = node.end_position();

    // Build metadata with receiver, receiver_type, visibility, and goroutine/channel flags
    let metadata = {
        let mut meta = serde_json::Map::new();

        // Add visibility based on method name
        if let Some(ref method_name) = name {
            meta.insert("visibility".to_string(), serde_json::json!(go_visibility(method_name)));
        }

        if let Some(r) = receiver {
            meta.insert("receiver".to_string(), serde_json::json!(r));
        }
        if let Some(rt) = receiver_type {
            meta.insert("receiver_type".to_string(), serde_json::json!(rt));
        }
        if let Some(rtn) = receiver_type_name {
            meta.insert("receiver_type_name".to_string(), serde_json::json!(rtn));
        }
        if has_goroutines {
            meta.insert("has_goroutines".to_string(), serde_json::json!(true));
        }
        if has_channels {
            meta.insert("has_channels".to_string(), serde_json::json!(true));
        }
        if meta.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(meta))
        }
    };

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "method".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata,
    });
}

fn extract_go_type_declaration(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Go type declarations can have multiple specs (e.g., type ( ... ))
    // Look for type_spec children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "type_spec" {
                extract_go_type_spec(source, child, chunks);
            }
        }
    }
}

fn extract_go_type_spec(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract type name
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract type definition
    let type_def = node.child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Determine the kind based on the type and extract additional metadata
    let type_node_opt = node.child_by_field_name("type");
    let kind = if let Some(ref type_node) = type_node_opt {
        match type_node.kind() {
            "struct_type" => "struct",
            "interface_type" => "interface",
            _ => "type",
        }
    } else {
        "type"
    };

    // Extract doc comment
    let docstring = extract_go_doc_comment(source, node);

    let start = node.start_position();
    let end = node.end_position();

    // Build metadata with visibility and type-specific details
    let metadata = {
        let mut meta = serde_json::Map::new();

        // Add visibility based on type name
        if let Some(ref type_name) = name {
            meta.insert("visibility".to_string(), serde_json::json!(go_visibility(type_name)));
        }

        // For structs, extract embedded types
        if let Some(ref type_node) = type_node_opt {
            if type_node.kind() == "struct_type" {
                let embedded_types = extract_go_embedded_types(source, *type_node);
                if !embedded_types.is_empty() {
                    meta.insert("embedded_types".to_string(), serde_json::json!(embedded_types));
                }
            } else if type_node.kind() == "interface_type" {
                // For interfaces, extract method signatures
                let interface_methods = extract_go_interface_methods(source, *type_node);
                if !interface_methods.is_empty() {
                    meta.insert("interface_methods".to_string(), serde_json::json!(interface_methods));
                }
            }
        }

        if meta.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(meta))
        }
    };

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: kind.to_string(),
        signature: type_def,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata,
    });
}

fn extract_go_const_declaration(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Go const declarations can have multiple specs (e.g., const ( ... ))
    // Look for const_spec children or const_spec_list
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "const_spec" {
                extract_go_const_spec(source, child, chunks);
            } else if child.kind() == "const_spec_list" {
                // Handle const_spec_list which contains multiple const_spec nodes
                for j in 0..child.child_count() {
                    if let Some(spec) = child.child(j) {
                        if spec.kind() == "const_spec" {
                            extract_go_const_spec(source, spec, chunks);
                        }
                    }
                }
            }
        }
    }
}

fn extract_go_const_spec(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract constant name - look for first identifier child
    let name = {
        let mut const_name = None;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "identifier" {
                    const_name = child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
                    break;
                }
            }
        }
        const_name
    };

    // Extract type (if specified)
    let type_annotation = node.child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract value
    let value = node.child_by_field_name("value")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Build signature
    let signature = match (&type_annotation, &value) {
        (Some(t), Some(v)) => Some(format!("{} = {}", t, v)),
        (Some(t), None) => Some(t.clone()),
        (None, Some(v)) => Some(format!("= {}", v)),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_go_doc_comment(source, node);

    let start = node.start_position();
    let end = node.end_position();

    // Build metadata with visibility
    let metadata = if let Some(ref const_name) = name {
        let mut meta = serde_json::Map::new();
        meta.insert("visibility".to_string(), serde_json::json!(go_visibility(const_name)));
        Some(serde_json::Value::Object(meta))
    } else {
        None
    };

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "constant".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata,
    });
}

fn extract_go_var_declaration(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Go var declarations can have multiple specs (e.g., var ( ... ))
    // Look for var_spec children or var_spec_list
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "var_spec" {
                extract_go_var_spec(source, child, chunks);
            } else if child.kind() == "var_spec_list" {
                // Handle var_spec_list which contains multiple var_spec nodes
                for j in 0..child.child_count() {
                    if let Some(spec) = child.child(j) {
                        if spec.kind() == "var_spec" {
                            extract_go_var_spec(source, spec, chunks);
                        }
                    }
                }
            }
        }
    }
}

fn extract_go_var_spec(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract variable name - look for first identifier child
    let name = {
        let mut var_name = None;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "identifier" {
                    var_name = child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
                    break;
                }
            }
        }
        var_name
    };

    // Extract type (if specified)
    let type_annotation = node.child_by_field_name("type")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract value
    let value = node.child_by_field_name("value")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Build signature
    let signature = match (&type_annotation, &value) {
        (Some(t), Some(v)) => Some(format!("{} = {}", t, v)),
        (Some(t), None) => Some(t.clone()),
        (None, Some(v)) => Some(format!("= {}", v)),
        (None, None) => None,
    };

    // Extract doc comment
    let docstring = extract_go_doc_comment(source, node);

    let start = node.start_position();
    let end = node.end_position();

    // Build metadata with visibility
    let metadata = if let Some(ref var_name) = name {
        let mut meta = serde_json::Map::new();
        meta.insert("visibility".to_string(), serde_json::json!(go_visibility(var_name)));
        Some(serde_json::Value::Object(meta))
    } else {
        None
    };

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "variable".to_string(),
        signature,
        docstring,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata,
    });
}

fn extract_go_package(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract package name from package_identifier child
    let name = {
        let mut pkg_name = None;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "package_identifier" {
                    pkg_name = child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
                    break;
                }
            }
        }
        pkg_name
    };

    let start = node.start_position();
    let end = node.end_position();

    chunks.push(SymbolChunk {
        symbol_name: name,
        kind: "package".to_string(),
        signature: None,
        docstring: None,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: None,
    });
}

fn extract_go_import(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Go import declarations can have multiple specs (single or grouped imports)
    // Look for import_spec or import_spec_list children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "import_spec" {
                extract_go_import_spec(source, child, chunks);
            } else if child.kind() == "import_spec_list" {
                // Handle grouped imports: import ( ... )
                for j in 0..child.child_count() {
                    if let Some(spec) = child.child(j) {
                        if spec.kind() == "import_spec" {
                            extract_go_import_spec(source, spec, chunks);
                        }
                    }
                }
            }
        }
    }
}

fn extract_go_import_spec(source: &str, node: Node, chunks: &mut Vec<SymbolChunk>) {
    // Extract import path (the string literal)
    let import_path = node.child_by_field_name("path")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.trim_matches('"').to_string());

    // Extract import alias (if any)
    let alias = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // The symbol name is the alias if present, otherwise the import path
    let symbol_name = alias.clone().or_else(|| import_path.clone());

    let start = node.start_position();
    let end = node.end_position();

    // Create metadata with import details
    let mut metadata = serde_json::Map::new();
    if let Some(path) = &import_path {
        metadata.insert("import_path".to_string(), serde_json::json!(path));
    }
    if let Some(alias_name) = &alias {
        metadata.insert("alias".to_string(), serde_json::json!(alias_name));
    }

    chunks.push(SymbolChunk {
        symbol_name,
        kind: "import".to_string(),
        signature: import_path,
        docstring: None,
        start_line: (start.row + 1) as i32,
        end_line: (end.row + 1) as i32,
        metadata: if metadata.is_empty() { None } else { Some(serde_json::Value::Object(metadata)) },
    });
}

fn detect_go_concurrency(node: Node) -> (bool, bool) {
    // Recursively search for goroutine and channel usage in the AST
    let mut has_goroutines = false;
    let mut has_channels = false;

    fn walk_node(node: Node, has_goroutines: &mut bool, has_channels: &mut bool) {
        match node.kind() {
            "go_statement" => {
                // Found a goroutine spawn (go keyword)
                *has_goroutines = true;
            }
            "channel_type" => {
                // Found a channel type declaration (chan int, etc.)
                *has_channels = true;
            }
            "send_statement" | "receive_operator" => {
                // Found channel send/receive operations (<-)
                *has_channels = true;
            }
            _ => {}
        }

        // Recursively check child nodes
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                walk_node(child, has_goroutines, has_channels);
            }
        }
    }

    walk_node(node, &mut has_goroutines, &mut has_channels);
    (has_goroutines, has_channels)
}

fn extract_go_doc_comment(source: &str, node: Node) -> Option<String> {
    // Look for comment nodes before the declaration
    let start_line = node.start_position().row;
    let lines: Vec<&str> = source.lines().collect();

    let mut doc_lines = Vec::new();
    let mut scan_line = start_line.saturating_sub(1);

    // Scan backwards from the node to find doc comments
    while scan_line > 0 {
        let line = lines.get(scan_line)?;
        let trimmed = line.trim();

        if trimmed.starts_with("//") {
            // Doc comment line
            let comment_text = trimmed.trim_start_matches("//").trim();
            doc_lines.insert(0, comment_text.to_string());
            scan_line = scan_line.saturating_sub(1);
        } else if trimmed.is_empty() {
            // Empty line - check if we already have comments
            if doc_lines.is_empty() {
                scan_line = scan_line.saturating_sub(1);
            } else {
                // Empty line after comments - stop
                break;
            }
        } else {
            // Non-comment, non-empty line - stop scanning
            break;
        }
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}

// Helper function to determine if a Go identifier is exported (PascalCase) or unexported (camelCase)
fn go_is_exported(name: &str) -> bool {
    // In Go, an identifier is exported if it starts with an uppercase letter
    name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
}

// Helper function to determine Go visibility based on identifier name
fn go_visibility(name: &str) -> &'static str {
    if go_is_exported(name) {
        "exported"
    } else {
        "unexported"
    }
}

// Helper function to parse receiver type and determine if it's a pointer or value receiver
// Returns (receiver_type_name, is_pointer)
fn parse_go_receiver(receiver_text: &str) -> (Option<String>, Option<&'static str>) {
    // Receiver format: "(r *Type)" or "(r Type)"
    // Strip parentheses and split on whitespace
    let stripped = receiver_text.trim().trim_start_matches('(').trim_end_matches(')');
    let parts: Vec<&str> = stripped.split_whitespace().collect();

    if parts.len() >= 2 {
        let type_part = parts[1];
        if type_part.starts_with('*') {
            // Pointer receiver
            let type_name = type_part.trim_start_matches('*').to_string();
            (Some(type_name), Some("pointer"))
        } else {
            // Value receiver
            (Some(type_part.to_string()), Some("value"))
        }
    } else {
        (None, None)
    }
}

// Helper function to extract embedded types from a struct_type node
fn extract_go_embedded_types(source: &str, struct_node: Node) -> Vec<String> {
    let mut embedded_types = Vec::new();

    // Look for field_declaration_list child
    for i in 0..struct_node.child_count() {
        if let Some(child) = struct_node.child(i) {
            if child.kind() == "field_declaration_list" {
                // Iterate through field declarations
                for j in 0..child.child_count() {
                    if let Some(field) = child.child(j) {
                        if field.kind() == "field_declaration" {
                            // Check if this is an embedded field (no explicit field name)
                            // An embedded field has a type but no name list
                            let has_name = field.child_by_field_name("name").is_some();

                            if !has_name {
                                // This is an embedded field - extract the type
                                if let Some(type_node) = field.child_by_field_name("type") {
                                    if let Ok(type_text) = type_node.utf8_text(source.as_bytes()) {
                                        // Handle pointer types (strip the *)
                                        let type_name = type_text.trim_start_matches('*').trim();
                                        embedded_types.push(type_name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    embedded_types
}

// Helper function to extract interface method signatures from an interface_type node
fn extract_go_interface_methods(source: &str, interface_node: Node) -> Vec<String> {
    let mut methods = Vec::new();

    // Look for method_elem children (used by tree-sitter-go for interface methods)
    for i in 0..interface_node.child_count() {
        if let Some(child) = interface_node.child(i) {
            if child.kind() == "method_elem" {
                // Extract the full method signature
                if let Ok(method_sig) = child.utf8_text(source.as_bytes()) {
                    methods.push(method_sig.trim().to_string());
                }
            }
        }
    }

    methods
}

// go.mod parsing (simple text-based parsing)
fn extract_gomod_chunks(source: &str) -> Vec<SymbolChunk> {
    let mut chunks = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Extract module name
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("module ") {
            let module_name = trimmed.strip_prefix("module ").unwrap_or("").trim();

            chunks.push(SymbolChunk {
                symbol_name: Some(module_name.to_string()),
                kind: "module".to_string(),
                signature: None,
                docstring: None,
                start_line: (i + 1) as i32,
                end_line: (i + 1) as i32,
                metadata: Some(serde_json::json!({"type": "go_module"})),
            });
        } else if trimmed.starts_with("go ") {
            // Go version requirement
            let version = trimmed.strip_prefix("go ").unwrap_or("").trim();

            chunks.push(SymbolChunk {
                symbol_name: Some(format!("go {}", version)),
                kind: "go_version".to_string(),
                signature: None,
                docstring: None,
                start_line: (i + 1) as i32,
                end_line: (i + 1) as i32,
                metadata: Some(serde_json::json!({"version": version})),
            });
        } else if trimmed.starts_with("require ") {
            // Single-line require
            let req = trimmed.strip_prefix("require ").unwrap_or("").trim();
            if !req.is_empty() && !req.starts_with('(') {
                chunks.push(SymbolChunk {
                    symbol_name: Some(req.to_string()),
                    kind: "require".to_string(),
                    signature: None,
                    docstring: None,
                    start_line: (i + 1) as i32,
                    end_line: (i + 1) as i32,
                    metadata: Some(serde_json::json!({"dependency": req})),
                });
            }
        }
    }

    // Handle multi-line require blocks
    let mut in_require = false;
    let mut require_start = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("require (") || trimmed == "require (" {
            in_require = true;
            require_start = i;
        } else if in_require && trimmed == ")" {
            in_require = false;
        } else if in_require && !trimmed.is_empty() && !trimmed.starts_with("//") {
            // Extract dependency from require block
            let dep = trimmed.trim();
            if !dep.is_empty() {
                chunks.push(SymbolChunk {
                    symbol_name: Some(dep.to_string()),
                    kind: "require".to_string(),
                    signature: None,
                    docstring: None,
                    start_line: (require_start + 1) as i32,
                    end_line: (i + 1) as i32,
                    metadata: Some(serde_json::json!({"dependency": dep})),
                });
            }
        }
    }

    chunks
}


