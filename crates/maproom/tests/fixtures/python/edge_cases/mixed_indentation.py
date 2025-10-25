# Edge case: Mixed indentation and encoding issues
# Note: This file intentionally has inconsistent formatting to test parser robustness

def function_with_spaces():
    """Function indented with spaces."""
    x = 1
    y = 2
    return x + y

def function_mixed_indent():
	"""Function with mixed tab/space indentation (tabs used here)."""
	result = 0
	for i in range(10):
	    result += i  # Mix of tab and spaces
	return result

class MixedIndentClass:
    """Class with mixed indentation patterns."""

    def method_spaces(self):
        """Method with space indentation."""
        value = 42
        return value

    def method_tabs(self):
	"""Method with tab indentation."""
	value = 84
	return value

# Unicode in function names (valid Python 3)
def функция_unicode():
    """Function with Unicode name."""
    return "unicode"

def función_española():
    """Function with Spanish characters."""
    return "español"

# Emoji in comments and strings
def emoji_function():
    """Function with emoji 🚀 in docstring."""
    # TODO: Fix this 🐛
    return "emoji 🎉"

# String with various encodings
def string_encodings():
    """Test various string patterns."""
    utf8_str = "Hello 世界"
    raw_str = r"Raw string with \n not escaped"
    byte_str = b"Byte string"
    formatted = f"Formatted {utf8_str}"
    multiline = """
    Multiline
    string
    """
    return (utf8_str, raw_str, byte_str, formatted, multiline)

# Comments with special characters
# Comment with special chars: @#$%^&*()
# Comment with Unicode: Привет мир
# Comment with emoji: 😀 👍 ✅

def function_after_unicode():
    """This should still parse correctly."""
    return True

# Line continuation with backslash
def long_function_call():
    """Function with line continuation."""
    result = some_function(
        arg1="value1", \
        arg2="value2", \
        arg3="value3"
    )
    return result

# Unusual spacing patterns
def     unusual_spacing    (  param1  ,  param2  )   :
    """Function with unusual spacing."""
    return    param1   +   param2

class UnusualSpacing  :
    """Class with unusual spacing."""

    def   method   (  self  )  :
        """Method with unusual spacing."""
        return   42

# Empty lines and whitespace variations



def function_after_empty_lines():
    """Function after multiple empty lines."""


    x = 1


    return x

# Function with backslash continuation in string
def multiline_string():
    """Test multiline strings."""
    long_string = "This is a very long string that \
continues on the next line \
and the next line too"
    return long_string

# Parenthesized continuation (preferred Python style)
def parenthesized_continuation():
    """Function using parentheses for continuation."""
    result = (
        value1 +
        value2 +
        value3 +
        value4
    )
    return result

# Valid function at end to ensure parser recovers
def final_valid_function():
    """This should be extracted successfully."""
    return "success"
