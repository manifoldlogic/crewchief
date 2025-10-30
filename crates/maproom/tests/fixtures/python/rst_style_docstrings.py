"""Sample Python module with reStructuredText-style docstrings.

This module demonstrates various reST-style docstring patterns
for testing the docstring parser.
"""


def concat_strings(first, second, separator=' '):
    """Concatenate two strings with a separator.

    :param first: The first string
    :type first: str
    :param second: The second string
    :type second: str
    :param separator: Separator to use between strings
    :type separator: str
    :returns: The concatenated string
    :rtype: str
    """
    return f"{first}{separator}{second}"


def parse_config(filename, encoding='utf-8'):
    """Parse a configuration file.

    :param filename: Path to the configuration file
    :type filename: str
    :param encoding: File encoding to use
    :type encoding: str
    :returns: Parsed configuration dictionary
    :rtype: dict
    :raises FileNotFoundError: If the file does not exist
    :raises ValueError: If the file format is invalid
    """
    try:
        with open(filename, 'r', encoding=encoding) as f:
            content = f.read()
    except FileNotFoundError:
        raise FileNotFoundError(f"Config file not found: {filename}")

    if not content.strip():
        raise ValueError("Config file is empty")

    return {}


class FileHandler:
    """Handle file operations with error checking.

    This class provides methods for reading, writing, and
    manipulating files with comprehensive error handling.

    :ivar filename: Path to the file
    :vartype filename: str
    :ivar mode: File opening mode
    :vartype mode: str
    :ivar encoding: File encoding
    :vartype encoding: str
    """

    def __init__(self, filename, mode='r', encoding='utf-8'):
        """Initialize the file handler.

        :param filename: Path to the file to handle
        :type filename: str
        :param mode: File opening mode ('r', 'w', 'a')
        :type mode: str
        :param encoding: Text encoding to use
        :type encoding: str
        :raises ValueError: If mode is invalid
        """
        if mode not in ['r', 'w', 'a', 'r+', 'w+', 'a+']:
            raise ValueError(f"Invalid mode: {mode}")

        self.filename = filename
        self.mode = mode
        self.encoding = encoding

    def read(self, size=-1):
        """Read content from the file.

        :param size: Number of bytes to read (-1 for all)
        :type size: int
        :returns: File content
        :rtype: str
        :raises IOError: If file cannot be read
        :raises PermissionError: If file permissions are insufficient
        """
        try:
            with open(self.filename, 'r', encoding=self.encoding) as f:
                return f.read(size)
        except IOError as e:
            raise IOError(f"Cannot read file: {e}")
        except PermissionError as e:
            raise PermissionError(f"Insufficient permissions: {e}")

    def write(self, content):
        """Write content to the file.

        :param content: Content to write
        :type content: str
        :returns: Number of characters written
        :rtype: int
        :raises IOError: If file cannot be written
        :raises TypeError: If content is not a string
        """
        if not isinstance(content, str):
            raise TypeError("Content must be a string")

        try:
            with open(self.filename, 'w', encoding=self.encoding) as f:
                return f.write(content)
        except IOError as e:
            raise IOError(f"Cannot write file: {e}")


def validate_email(email):
    """Validate an email address format.

    :param email: Email address to validate
    :type email: str
    :returns: True if valid, False otherwise
    :rtype: bool

    .. note::
       This is a simple validation and may not catch all invalid emails.

    .. warning::
       Does not verify that the email address actually exists.
    """
    return '@' in email and '.' in email.split('@')[1]


def fetch_url(url, timeout=30, headers=None):
    """Fetch content from a URL.

    :param url: The URL to fetch
    :type url: str
    :param timeout: Request timeout in seconds
    :type timeout: int or float
    :param headers: Optional HTTP headers
    :type headers: dict or None
    :returns: Response content
    :rtype: str
    :raises ConnectionError: If unable to connect to URL
    :raises TimeoutError: If request exceeds timeout
    :raises ValueError: If URL is malformed
    """
    if not url.startswith(('http://', 'https://')):
        raise ValueError("URL must start with http:// or https://")

    # Simplified implementation
    return ""


def calculate_statistics(data, metrics=None):
    """Calculate statistical metrics for data.

    :param data: Input data array
    :type data: list or numpy.ndarray
    :param metrics: List of metrics to calculate
                   (mean, median, std, var, min, max)
    :type metrics: list of str or None
    :returns: Dictionary mapping metric names to values
    :rtype: dict

    .. seealso::

       :func:`compute_correlation`
          Calculate correlation between variables

    .. versionadded:: 1.0

    .. versionchanged:: 1.1
       Added support for numpy arrays
    """
    if metrics is None:
        metrics = ['mean', 'median', 'std']

    results = {}
    for metric in metrics:
        if metric == 'mean':
            results[metric] = sum(data) / len(data)
        # Other metrics would be calculated here

    return results


class DatabaseConnection:
    """Manage database connections and queries.

    :param host: Database host address
    :type host: str
    :param port: Database port number
    :type port: int
    :param database: Database name
    :type database: str
    :param username: Username for authentication
    :type username: str
    :param password: Password for authentication
    :type password: str
    """

    def __init__(self, host, port, database, username, password):
        """Initialize database connection.

        :param host: Database server hostname
        :type host: str
        :param port: Database server port
        :type port: int
        :param database: Name of the database
        :type database: str
        :param username: Database username
        :type username: str
        :param password: Database password
        :type password: str
        :raises ConnectionError: If unable to connect to database
        """
        self.host = host
        self.port = port
        self.database = database
        self.username = username
        self._password = password

    def execute_query(self, query, parameters=None):
        """Execute a database query.

        :param query: SQL query string
        :type query: str
        :param parameters: Query parameters for binding
        :type parameters: tuple or dict or None
        :returns: Query results
        :rtype: list
        :raises ValueError: If query is empty or invalid
        :raises RuntimeError: If query execution fails
        """
        if not query or not query.strip():
            raise ValueError("Query cannot be empty")

        # Simplified implementation
        return []


def merge_dicts(*dicts, **kwargs):
    """Merge multiple dictionaries into one.

    :param dicts: Variable number of dictionaries to merge
    :type dicts: dict
    :param kwargs: Additional key-value pairs to include
    :returns: Merged dictionary
    :rtype: dict

    .. note::
       Later dictionaries override earlier ones for duplicate keys.
    """
    result = {}
    for d in dicts:
        result.update(d)
    result.update(kwargs)
    return result
