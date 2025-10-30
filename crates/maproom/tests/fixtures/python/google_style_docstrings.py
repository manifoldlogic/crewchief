"""Sample Python module with Google-style docstrings.

This module demonstrates various Google-style docstring patterns
for testing the docstring parser.
"""


def simple_add(a, b):
    """Add two numbers together.

    Args:
        a (int): First number
        b (int): Second number

    Returns:
        int: Sum of a and b
    """
    return a + b


def divide_safely(numerator, denominator):
    """Safely divide two numbers with error handling.

    Args:
        numerator (float): The number to be divided
        denominator (float): The number to divide by

    Returns:
        float: Result of division

    Raises:
        ValueError: If denominator is zero
        TypeError: If inputs are not numeric
    """
    if not isinstance(numerator, (int, float)) or not isinstance(denominator, (int, float)):
        raise TypeError("Both arguments must be numeric")
    if denominator == 0:
        raise ValueError("Cannot divide by zero")
    return numerator / denominator


class Calculator:
    """A calculator class with basic operations.

    This class provides methods for performing basic arithmetic
    operations with optional precision control.

    Attributes:
        precision (int): Number of decimal places for results
        history (list): List of previous calculations
        debug_mode (bool): Enable debug output
    """

    def __init__(self, precision=2, debug=False):
        """Initialize the calculator.

        Args:
            precision (int): Decimal places for rounding results. Defaults to 2.
            debug (bool): Enable debug logging. Defaults to False.

        Raises:
            ValueError: If precision is negative
        """
        if precision < 0:
            raise ValueError("Precision cannot be negative")
        self.precision = precision
        self.history = []
        self.debug_mode = debug

    def calculate(self, operation, *args):
        """Perform a calculation and store in history.

        Args:
            operation (str): The operation to perform (add, subtract, multiply, divide)
            *args: Variable number of numeric arguments

        Returns:
            float: The result of the calculation

        Raises:
            ValueError: If operation is not supported
            TypeError: If any argument is not numeric

        Examples:
            >>> calc = Calculator()
            >>> calc.calculate('add', 5, 3)
            8.0
            >>> calc.calculate('multiply', 2, 3, 4)
            24.0

        Note:
            Results are rounded to the precision specified during initialization.
        """
        if operation not in ['add', 'subtract', 'multiply', 'divide']:
            raise ValueError(f"Unsupported operation: {operation}")

        for arg in args:
            if not isinstance(arg, (int, float)):
                raise TypeError("All arguments must be numeric")

        if operation == 'add':
            result = sum(args)
        elif operation == 'multiply':
            result = 1
            for arg in args:
                result *= arg
        else:
            result = 0  # Simplified

        result = round(result, self.precision)
        self.history.append((operation, args, result))
        return result


def async_fetch_data(url, timeout=30):
    """Fetch data from a URL asynchronously.

    Args:
        url (str): The URL to fetch data from
        timeout (int): Request timeout in seconds. Defaults to 30.

    Returns:
        dict: The fetched data as a dictionary

    Raises:
        TimeoutError: If request exceeds timeout
        ConnectionError: If unable to connect to URL

    Warning:
        This function requires an active internet connection.
    """
    pass


def generator_example(start, end, step=1):
    """Generate a sequence of numbers.

    Args:
        start (int): Starting value
        end (int): Ending value (exclusive)
        step (int): Step size. Defaults to 1.

    Yields:
        int: Next number in the sequence

    Examples:
        >>> list(generator_example(0, 5))
        [0, 1, 2, 3, 4]
        >>> list(generator_example(0, 10, 2))
        [0, 2, 4, 6, 8]
    """
    current = start
    while current < end:
        yield current
        current += step
