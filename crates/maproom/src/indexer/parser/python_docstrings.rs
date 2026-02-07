//! Python docstring parsing utilities

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
    if docstring.contains(":param ")
        || docstring.contains(":returns:")
        || docstring.contains(":type ")
        || docstring.contains(":raises ")
        || docstring.contains(":rtype:")
    {
        return DocstringFormat::ReStructuredText;
    }

    // Check for NumPy style (section headers with underlines)
    for i in 0..lines.len().saturating_sub(1) {
        let line = lines[i].trim();
        let next_line = lines[i + 1].trim();

        // NumPy uses underlines (--- or ===) under section headers
        if !line.is_empty()
            && (next_line.chars().all(|c| c == '-') || next_line.chars().all(|c| c == '='))
        {
            if line.eq_ignore_ascii_case("parameters")
                || line.eq_ignore_ascii_case("returns")
                || line.eq_ignore_ascii_case("raises")
                || line.eq_ignore_ascii_case("yields")
                || line.eq_ignore_ascii_case("notes")
                || line.eq_ignore_ascii_case("attributes")
            {
                return DocstringFormat::NumPy;
            }
        }
    }

    // Check for Google style (sections ending with colon)
    for line in &lines {
        let trimmed = line.trim();
        if trimmed == "Args:"
            || trimmed == "Arguments:"
            || trimmed == "Returns:"
            || trimmed == "Return:"
            || trimmed == "Raises:"
            || trimmed == "Yields:"
            || trimmed == "Examples:"
            || trimmed == "Example:"
            || trimmed == "Note:"
            || trimmed == "Notes:"
            || trimmed == "Warning:"
            || trimmed == "Warnings:"
            || trimmed == "Attributes:"
        {
            return DocstringFormat::Google;
        }
    }

    DocstringFormat::Plain
}

/// Parses a Python docstring and returns a normalized, structured format
pub(super) fn parse_python_docstring(docstring: &str) -> String {
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
    let mut i = 0;

    // Extract brief description (everything before first section)
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed == "Args:"
            || trimmed == "Arguments:"
            || trimmed == "Returns:"
            || trimmed == "Return:"
            || trimmed == "Raises:"
            || trimmed == "Yields:"
            || trimmed == "Examples:"
            || trimmed == "Example:"
            || trimmed == "Note:"
            || trimmed == "Notes:"
            || trimmed == "Warning:"
            || trimmed == "Warnings:"
            || trimmed == "Attributes:"
        {
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

            let current_section = section_name.to_string();
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
                    let is_list_section = current_section == "Parameters"
                        || current_section == "Attributes"
                        || current_section == "Raises"
                        || current_section == "Yields";

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
    let mut i = 0;

    // Extract brief description (everything before first section)
    while i < lines.len() {
        if i + 1 < lines.len() {
            let line = lines[i].trim();
            let next_line = lines[i + 1].trim();

            // Check if next line is an underline
            if !line.is_empty()
                && (next_line.chars().all(|c| c == '-') || next_line.chars().all(|c| c == '='))
            {
                if line.eq_ignore_ascii_case("parameters")
                    || line.eq_ignore_ascii_case("returns")
                    || line.eq_ignore_ascii_case("raises")
                    || line.eq_ignore_ascii_case("yields")
                    || line.eq_ignore_ascii_case("notes")
                    || line.eq_ignore_ascii_case("examples")
                    || line.eq_ignore_ascii_case("attributes")
                {
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
            if !line.is_empty()
                && (next_line.chars().all(|c| c == '-') || next_line.chars().all(|c| c == '='))
            {
                let current_section = line.to_string();
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
                        if !content_line.is_empty()
                            && (next.chars().all(|c| c == '-') || next.chars().all(|c| c == '='))
                        {
                            break;
                        }
                    }

                    let content_line = lines[i];
                    let trimmed = content_line.trim();

                    // Handle parameter lines (format: "param_name : type" followed by description)
                    if !trimmed.is_empty() {
                        if current_section.eq_ignore_ascii_case("parameters")
                            || current_section.eq_ignore_ascii_case("attributes")
                        {
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
            let prefix = if line.starts_with(":returns:") {
                ":returns:"
            } else {
                ":return:"
            };
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
