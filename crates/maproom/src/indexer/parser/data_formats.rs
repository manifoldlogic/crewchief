//! Data format parsers (JSON, YAML, TOML)

use crate::indexer::SymbolChunk;

/// Extract chunks from JSON source code
pub(super) fn extract_json_chunks(source: &str) -> Vec<SymbolChunk> {
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
        let important_keys = [
            "scripts",
            "dependencies",
            "devDependencies",
            "config",
            "exports",
        ];

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
                            }
                            ',' if !in_string && brace_depth == 0 => {
                                // Simple value ends at comma
                                end_line = line_num;
                                current_line = line_num + 1;
                                break;
                            }
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

/// Extract chunks from YAML source code
pub(super) fn extract_yaml_chunks(source: &str) -> Vec<SymbolChunk> {
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
                if !next_line.starts_with(' ')
                    && !next_line.starts_with('\t')
                    && !next_line.trim().is_empty()
                    && !next_line.trim().starts_with('#')
                    && next_line.contains(':')
                {
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

/// Extract chunks from TOML source code
pub(super) fn extract_toml_chunks(source: &str) -> Vec<SymbolChunk> {
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
            let section_name = trimmed
                .trim_start_matches('[')
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
