use maproom::indexer::parser;
use std::fs;

/// Test parsing Google-style docstrings from a complete fixture file
#[test]
fn test_google_style_fixture() {
    let source = fs::read_to_string("tests/fixtures/python/google_style_docstrings.py")
        .expect("Failed to read Google-style fixture");

    let chunks = parser::extract_chunks(&source, "py");

    // Should extract multiple functions and classes with docstrings
    assert!(
        chunks.len() > 0,
        "Should extract symbols from Google-style fixture"
    );

    // Find the divide_safely function
    let divide_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("divide_safely".to_string()));
    assert!(divide_func.is_some(), "Should find divide_safely function");

    let divide_func = divide_func.unwrap();
    let docstring = divide_func.docstring.as_ref().unwrap();

    // Verify parsed structure
    assert!(docstring.contains("Safely divide two numbers"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("- numerator (float)"));
    assert!(docstring.contains("- denominator (float)"));
    assert!(docstring.contains("Returns:"));
    assert!(docstring.contains("Raises:"));
    assert!(docstring.contains("- ValueError: If denominator is zero"));
    assert!(docstring.contains("- TypeError: If inputs are not numeric"));

    // Find the Calculator class
    let calculator_class = chunks
        .iter()
        .find(|c| c.symbol_name == Some("Calculator".to_string()));
    assert!(calculator_class.is_some(), "Should find Calculator class");

    let calculator_class = calculator_class.unwrap();
    let class_docstring = calculator_class.docstring.as_ref().unwrap();

    assert!(class_docstring.contains("A calculator class with basic operations"));
    assert!(class_docstring.contains("Attributes:"));
    assert!(class_docstring.contains("- precision (int)"));
    assert!(class_docstring.contains("- history (list)"));
}

/// Test parsing NumPy-style docstrings from a complete fixture file
#[test]
fn test_numpy_style_fixture() {
    let source = fs::read_to_string("tests/fixtures/python/numpy_style_docstrings.py")
        .expect("Failed to read NumPy-style fixture");

    let chunks = parser::extract_chunks(&source, "py");

    assert!(
        chunks.len() > 0,
        "Should extract symbols from NumPy-style fixture"
    );

    // Find the process_matrix function
    let process_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("process_matrix".to_string()));
    assert!(
        process_func.is_some(),
        "Should find process_matrix function"
    );

    let process_func = process_func.unwrap();
    let docstring = process_func.docstring.as_ref().unwrap();

    // Verify parsed structure
    assert!(docstring.contains("Process a matrix with thresholding"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("matrix : ndarray"));
    assert!(docstring.contains("threshold : float"));
    assert!(docstring.contains("normalize : bool"));
    assert!(docstring.contains("Returns:"));
    assert!(docstring.contains("Raises:"));
    assert!(docstring.contains("ValueError"));
    assert!(docstring.contains("TypeError"));

    // Find the DataProcessor class
    let data_processor = chunks
        .iter()
        .find(|c| c.symbol_name == Some("DataProcessor".to_string()));
    assert!(data_processor.is_some(), "Should find DataProcessor class");

    let data_processor = data_processor.unwrap();
    let class_docstring = data_processor.docstring.as_ref().unwrap();

    assert!(class_docstring.contains("Process and analyze numerical data"));
    assert!(class_docstring.contains("Attributes:"));
}

/// Test parsing reStructuredText-style docstrings from a complete fixture file
#[test]
fn test_rst_style_fixture() {
    let source = fs::read_to_string("tests/fixtures/python/rst_style_docstrings.py")
        .expect("Failed to read reST-style fixture");

    let chunks = parser::extract_chunks(&source, "py");

    assert!(
        chunks.len() > 0,
        "Should extract symbols from reST-style fixture"
    );

    // Find the parse_config function
    let parse_func = chunks
        .iter()
        .find(|c| c.symbol_name == Some("parse_config".to_string()));
    assert!(parse_func.is_some(), "Should find parse_config function");

    let parse_func = parse_func.unwrap();
    let docstring = parse_func.docstring.as_ref().unwrap();

    // Verify parsed structure
    assert!(docstring.contains("Parse a configuration file"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("- filename (str): Path to the configuration file"));
    assert!(docstring.contains("- encoding (str): File encoding to use"));
    assert!(docstring.contains("Returns:"));
    assert!(docstring.contains("dict: Parsed configuration dictionary"));
    assert!(docstring.contains("Raises:"));
    assert!(docstring.contains("- FileNotFoundError: If the file does not exist"));
    assert!(docstring.contains("- ValueError: If the file format is invalid"));

    // Find the FileHandler class
    let file_handler = chunks
        .iter()
        .find(|c| c.symbol_name == Some("FileHandler".to_string()));
    assert!(file_handler.is_some(), "Should find FileHandler class");

    let file_handler = file_handler.unwrap();
    let class_docstring = file_handler.docstring.as_ref().unwrap();

    assert!(class_docstring.contains("Handle file operations with error checking"));
}

/// Test that all three styles are correctly detected and parsed in mixed scenarios
#[test]
fn test_mixed_docstring_styles() {
    // Create a mixed file with all three styles
    let source = r#"
def google_function(a, b):
    """Google-style function.

    Args:
        a (int): First arg
        b (int): Second arg

    Returns:
        int: Result
    """
    return a + b

def numpy_function(x, y):
    """NumPy-style function.

    Parameters
    ----------
    x : float
        First parameter
    y : float
        Second parameter

    Returns
    -------
    float
        Result value
    """
    return x * y

def rst_function(value):
    """reST-style function.

    :param value: Input value
    :type value: int
    :returns: Processed value
    :rtype: int
    """
    return value * 2
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 3, "Should extract all three functions");

    // Verify each function has correctly parsed docstrings
    for chunk in &chunks {
        assert!(
            chunk.docstring.is_some(),
            "All functions should have docstrings"
        );
        let docstring = chunk.docstring.as_ref().unwrap();

        match chunk.symbol_name.as_deref() {
            Some("google_function") => {
                assert!(docstring.contains("Parameters:"));
                assert!(docstring.contains("- a (int): First arg"));
                assert!(docstring.contains("Returns:"));
            }
            Some("numpy_function") => {
                assert!(docstring.contains("Parameters:"));
                assert!(docstring.contains("x : float"));
                assert!(docstring.contains("Returns:"));
            }
            Some("rst_function") => {
                assert!(docstring.contains("Parameters:"));
                assert!(docstring.contains("- value (int): Input value"));
                assert!(docstring.contains("Returns:"));
                assert!(docstring.contains("int: Processed value"));
            }
            _ => panic!("Unexpected function name"),
        }
    }
}

/// Test that plain docstrings (no special formatting) are preserved as-is
#[test]
fn test_plain_docstrings_preserved() {
    let source = r#"
def simple_func():
    """This is a simple, plain docstring with no special sections."""
    pass

def multiline_plain():
    """This is a multiline plain docstring.

    It has multiple paragraphs but no special section markers.
    It should be preserved as-is without transformation.
    """
    pass
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 2);

    let simple = &chunks[0];
    assert_eq!(
        simple.docstring.as_deref(),
        Some("This is a simple, plain docstring with no special sections.")
    );

    let multiline = &chunks[1];
    let multiline_doc = multiline.docstring.as_ref().unwrap();
    assert!(multiline_doc.contains("This is a multiline plain docstring"));
    assert!(multiline_doc.contains("It has multiple paragraphs"));
    assert!(!multiline_doc.contains("Parameters:"));
}
