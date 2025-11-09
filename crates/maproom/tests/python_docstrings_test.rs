use crewchief_maproom::indexer::parser;

/// Test Google-style docstring parsing with Args and Returns sections
#[test]
fn test_google_style_docstring_basic() {
    let source = r#"
def calculate_sum(a, b):
    """Calculate the sum of two numbers.

    Args:
        a (int): The first number
        b (int): The second number

    Returns:
        int: The sum of a and b
    """
    return a + b
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    assert_eq!(func.symbol_name, Some("calculate_sum".to_string()));
    assert_eq!(func.kind, "func");

    let docstring = func.docstring.as_ref().unwrap();
    assert!(docstring.contains("Calculate the sum of two numbers"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("- a (int): The first number"));
    assert!(docstring.contains("- b (int): The second number"));
    assert!(docstring.contains("Returns:"));
    assert!(docstring.contains("int: The sum of a and b"));
}

/// Test Google-style docstring with multiple sections including Raises
#[test]
fn test_google_style_docstring_with_raises() {
    let source = r#"
def divide(numerator, denominator):
    """Divide two numbers.

    Args:
        numerator (float): The number to divide
        denominator (float): The number to divide by

    Returns:
        float: The result of the division

    Raises:
        ValueError: If denominator is zero
    """
    if denominator == 0:
        raise ValueError("Cannot divide by zero")
    return numerator / denominator
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    let docstring = func.docstring.as_ref().unwrap();

    assert!(docstring.contains("Divide two numbers"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("- numerator (float)"));
    assert!(docstring.contains("- denominator (float)"));
    assert!(docstring.contains("Returns:"));
    assert!(docstring.contains("Raises:"));
    assert!(docstring.contains("ValueError"));
}

/// Test NumPy-style docstring parsing with Parameters and Returns sections
#[test]
fn test_numpy_style_docstring_basic() {
    let source = r#"
def process_data(input_array, threshold):
    """Process input data with a threshold.

    Parameters
    ----------
    input_array : ndarray
        The input data array to process
    threshold : float
        The threshold value for filtering

    Returns
    -------
    ndarray
        The processed data array
    """
    return input_array[input_array > threshold]
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    assert_eq!(func.symbol_name, Some("process_data".to_string()));

    let docstring = func.docstring.as_ref().unwrap();
    assert!(docstring.contains("Process input data with a threshold"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("input_array : ndarray"));
    assert!(docstring.contains("The input data array to process"));
    assert!(docstring.contains("threshold : float"));
    assert!(docstring.contains("Returns:"));
    assert!(docstring.contains("ndarray"));
}

/// Test NumPy-style docstring with Raises section
#[test]
fn test_numpy_style_docstring_with_raises() {
    let source = r#"
def validate_input(data):
    """Validate the input data.

    Parameters
    ----------
    data : list
        The data to validate

    Returns
    -------
    bool
        True if valid, False otherwise

    Raises
    ------
    TypeError
        If data is not a list
    """
    if not isinstance(data, list):
        raise TypeError("Data must be a list")
    return True
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    let docstring = func.docstring.as_ref().unwrap();

    assert!(docstring.contains("Validate the input data"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("data : list"));
    assert!(docstring.contains("Returns:"));
    assert!(docstring.contains("bool"));
    assert!(docstring.contains("Raises:"));
    assert!(docstring.contains("TypeError"));
}

/// Test reStructuredText-style docstring parsing
#[test]
fn test_rst_style_docstring_basic() {
    let source = r#"
def format_message(text, width):
    """Format a message to a specific width.

    :param text: The text to format
    :type text: str
    :param width: The target width in characters
    :type width: int
    :returns: The formatted text
    :rtype: str
    """
    return text[:width]
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    assert_eq!(func.symbol_name, Some("format_message".to_string()));

    let docstring = func.docstring.as_ref().unwrap();
    assert!(docstring.contains("Format a message to a specific width"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("- text (str): The text to format"));
    assert!(docstring.contains("- width (int): The target width in characters"));
    assert!(docstring.contains("Returns:"));
    assert!(docstring.contains("str: The formatted text"));
}

/// Test reStructuredText-style docstring with raises
#[test]
fn test_rst_style_docstring_with_raises() {
    let source = r#"
def open_file(filename):
    """Open and read a file.

    :param filename: Path to the file
    :type filename: str
    :returns: File contents
    :rtype: str
    :raises FileNotFoundError: If file doesn't exist
    :raises PermissionError: If file cannot be read
    """
    with open(filename, 'r') as f:
        return f.read()
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    let docstring = func.docstring.as_ref().unwrap();

    assert!(docstring.contains("Open and read a file"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("- filename (str): Path to the file"));
    assert!(docstring.contains("Returns:"));
    assert!(docstring.contains("str: File contents"));
    assert!(docstring.contains("Raises:"));
    assert!(docstring.contains("- FileNotFoundError: If file doesn't exist"));
    assert!(docstring.contains("- PermissionError: If file cannot be read"));
}

/// Test plain docstring (no special formatting)
#[test]
fn test_plain_docstring() {
    let source = r#"
def simple_function():
    """This is just a simple docstring with no special sections."""
    pass
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    let docstring = func.docstring.as_ref().unwrap();

    assert_eq!(
        docstring,
        "This is just a simple docstring with no special sections."
    );
}

/// Test class with Google-style docstring
#[test]
fn test_class_google_docstring() {
    let source = r#"
class Calculator:
    """A simple calculator class.

    Attributes:
        precision (int): Number of decimal places for results
        history (list): List of previous calculations
    """

    def __init__(self, precision=2):
        """Initialize the calculator.

        Args:
            precision (int): Number of decimal places
        """
        self.precision = precision
        self.history = []
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert!(chunks.len() >= 2, "Expected class and __init__ method");

    let class = &chunks[0];
    assert_eq!(class.kind, "class");

    let class_docstring = class.docstring.as_ref().unwrap();
    assert!(class_docstring.contains("A simple calculator class"));
    assert!(class_docstring.contains("Attributes:"));
    assert!(class_docstring.contains("- precision (int)"));
    assert!(class_docstring.contains("- history (list)"));

    let init = &chunks[1];
    assert_eq!(init.kind, "method");

    let init_docstring = init.docstring.as_ref().unwrap();
    assert!(init_docstring.contains("Initialize the calculator"));
    assert!(init_docstring.contains("Parameters:"));
    assert!(init_docstring.contains("- precision (int)"));
}

/// Test method with NumPy-style docstring
#[test]
fn test_method_numpy_docstring() {
    let source = r#"
class DataProcessor:
    """Process data efficiently."""

    def transform(self, data, factor):
        """Transform data by a factor.

        Parameters
        ----------
        data : array_like
            Input data to transform
        factor : float
            Multiplication factor

        Returns
        -------
        array_like
            Transformed data
        """
        return data * factor
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert!(chunks.len() >= 2);

    let method = chunks
        .iter()
        .find(|c| c.symbol_name == Some("transform".to_string()));
    assert!(method.is_some());

    let method = method.unwrap();
    let docstring = method.docstring.as_ref().unwrap();

    assert!(docstring.contains("Transform data by a factor"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("data : array_like"));
    assert!(docstring.contains("factor : float"));
    assert!(docstring.contains("Returns:"));
    assert!(docstring.contains("array_like"));
}

/// Test multi-line parameter descriptions in Google style
#[test]
fn test_google_multiline_params() {
    let source = r#"
def complex_function(param1, param2):
    """Perform a complex operation.

    Args:
        param1 (str): This is a parameter with a very long description
            that spans multiple lines and needs proper handling
        param2 (int): Another parameter

    Returns:
        bool: Success status
    """
    return True
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    let docstring = func.docstring.as_ref().unwrap();

    assert!(docstring.contains("Perform a complex operation"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("param1"));
    assert!(docstring.contains("param2"));
}

/// Test function with no docstring
#[test]
fn test_no_docstring() {
    let source = r#"
def undocumented_function():
    return 42
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    assert_eq!(func.symbol_name, Some("undocumented_function".to_string()));
    assert!(func.docstring.is_none());
}

/// Test empty docstring
#[test]
fn test_empty_docstring() {
    let source = r#"
def function_with_empty_docstring():
    """"""
    return 42
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    // Empty docstrings should be treated as None or empty string
    assert!(func.docstring.is_none() || func.docstring.as_ref().unwrap().is_empty());
}

/// Test Google-style with Examples section
#[test]
fn test_google_with_examples() {
    let source = r#"
def fibonacci(n):
    """Calculate the nth Fibonacci number.

    Args:
        n (int): The position in the Fibonacci sequence

    Returns:
        int: The nth Fibonacci number

    Examples:
        >>> fibonacci(5)
        5
        >>> fibonacci(10)
        55
    """
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    let docstring = func.docstring.as_ref().unwrap();

    assert!(docstring.contains("Calculate the nth Fibonacci number"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("Returns:"));
    assert!(docstring.contains("Examples:"));
}

/// Test NumPy-style with Notes section
#[test]
fn test_numpy_with_notes() {
    let source = r#"
def advanced_algorithm(data):
    """Process data using an advanced algorithm.

    Parameters
    ----------
    data : ndarray
        Input data array

    Returns
    -------
    ndarray
        Processed results

    Notes
    -----
    This algorithm uses a sophisticated approach
    that may be computationally expensive.
    """
    return data
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    let docstring = func.docstring.as_ref().unwrap();

    assert!(docstring.contains("Process data using an advanced algorithm"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("Returns:"));
    assert!(docstring.contains("Notes:"));
    assert!(docstring.contains("sophisticated approach"));
}

/// Test mixed content with different quote styles
#[test]
fn test_different_quote_styles() {
    let source = r#"
def single_quote_docstring():
    '''Single quote docstring.

    Args:
        None

    Returns:
        None
    '''
    pass

def double_quote_docstring():
    """Double quote docstring.

    Args:
        None

    Returns:
        None
    """
    pass
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 2);

    // Both should have docstrings parsed correctly
    for chunk in &chunks {
        assert!(chunk.docstring.is_some());
        let docstring = chunk.docstring.as_ref().unwrap();
        assert!(docstring.contains("docstring"));
    }
}

/// Test reST with multi-line parameter descriptions
#[test]
fn test_rst_multiline_descriptions() {
    let source = r#"
def process_request(request, timeout):
    """Process an HTTP request.

    :param request: The HTTP request object containing all the
                    necessary information about the client request
    :type request: Request
    :param timeout: Maximum time to wait for response in seconds
    :type timeout: float
    :returns: The processed response object
    :rtype: Response
    """
    return request.process()
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    let docstring = func.docstring.as_ref().unwrap();

    assert!(docstring.contains("Process an HTTP request"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("- request (Request)"));
    assert!(docstring.contains("- timeout (float)"));
    assert!(docstring.contains("Returns:"));
    assert!(docstring.contains("Response"));
}

/// Test decorated function with docstring
#[test]
fn test_decorated_function_docstring() {
    let source = r#"
@staticmethod
def utility_function(value):
    """Perform a utility operation.

    Args:
        value (any): Input value

    Returns:
        any: Processed value
    """
    return value
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    assert_eq!(func.symbol_name, Some("utility_function".to_string()));

    let docstring = func.docstring.as_ref().unwrap();
    assert!(docstring.contains("Perform a utility operation"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("Returns:"));
}

/// Test async function with docstring
#[test]
fn test_async_function_docstring() {
    let source = r#"
async def fetch_data(url):
    """Fetch data from a URL asynchronously.

    Args:
        url (str): The URL to fetch from

    Returns:
        dict: The fetched data
    """
    return {}
"#;

    let chunks = parser::extract_chunks(source, "py");
    assert_eq!(chunks.len(), 1);

    let func = &chunks[0];
    assert_eq!(func.symbol_name, Some("fetch_data".to_string()));

    let docstring = func.docstring.as_ref().unwrap();
    assert!(docstring.contains("Fetch data from a URL asynchronously"));
    assert!(docstring.contains("Parameters:"));
    assert!(docstring.contains("Returns:"));
}
