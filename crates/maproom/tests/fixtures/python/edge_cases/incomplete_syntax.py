# Edge case: Incomplete and malformed Python syntax
# These should be handled gracefully without panicking

# Incomplete function definition - missing closing parenthesis
def incomplete_function(
    param1,
    param2
    # Missing closing ) and colon

# Incomplete class definition - missing colon
class IncompleteClass

# Function with incomplete decorator
@incomplete_decorator(
def decorated_incomplete():
    pass

# Nested function with syntax error
def outer_function():
    def inner_incomplete(
        # Missing params and body

# Class with incomplete method
class PartialClass:
    def incomplete_method(self
        # Missing closing paren and body

# Valid function after errors - parser should recover
def valid_function_after_errors():
    """This should still be extracted."""
    return "success"

# Incomplete try-except block
try:
    do_something()
# Missing except clause

# Incomplete if statement
if condition
    # Missing colon

# Function with incomplete return type hint
def type_hint_incomplete() ->
    pass

# Async function with missing await
async def incomplete_async(
    # Missing params and body

# Valid class after errors
class ValidClassAfterErrors:
    """This should be extracted despite earlier errors."""

    def working_method(self):
        """This method should be extracted."""
        return True
