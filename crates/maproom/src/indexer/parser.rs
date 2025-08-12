use tree_sitter::{Language, Node, Parser};

use super::SymbolChunk;

// Use the safe language providers exposed by the crates
fn lang_typescript() -> Language { tree_sitter_typescript::language_typescript() }
fn lang_tsx() -> Language { tree_sitter_typescript::language_tsx() }
fn lang_javascript() -> Language { tree_sitter_javascript::language() }

pub fn extract_chunks(source: &str, language: &str) -> Vec<SymbolChunk> {
    match language {
        "md" | "mdx" => extract_markdown_chunks(source),
        "json" => extract_json_chunks(source),
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
                });
            }
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
    });
}

fn extract_preview(source: &str, start_line: i32, end_line: i32) -> String {
    let start = start_line.max(1) as usize - 1;
    let end = end_line.max(start_line) as usize;
    source.lines().skip(start).take(end - start).take(60).collect::<Vec<_>>().join("\n")
}


