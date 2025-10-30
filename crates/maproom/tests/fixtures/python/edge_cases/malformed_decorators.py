# Edge case: Unusual and malformed decorator patterns

# Nested decorators with arguments
@decorator1(arg1="value1")
@decorator2(arg2="value2", arg3={"key": "value"})
@decorator3
def nested_decorated_function():
    """Function with multiple complex decorators."""
    return "result"

# Decorator with lambda
@decorator(lambda x: x * 2)
def lambda_decorator_function():
    """Function decorated with lambda."""
    pass

# Decorator with function call
@get_decorator("dynamic")
def dynamic_decorator_function():
    """Function with dynamically generated decorator."""
    pass

# Decorator with attribute access
@module.submodule.decorator
def attribute_decorator_function():
    """Function with attribute-based decorator."""
    pass

# Class with multiple decorator styles
@dataclass
@custom_validator
@serializer(format="json")
class MultiDecoratedClass:
    """Class with multiple decorators."""
    field1: str
    field2: int

# Decorator with complex expression
@decorator1 if condition else decorator2
def conditional_decorator():
    """Function with conditional decorator (malformed)."""
    pass

# Chained decorator calls
@decorator_factory().with_option(True).build()
def chained_decorator_function():
    """Function with chained decorator calls."""
    pass

# Decorator with multiline arguments
@complex_decorator(
    param1="very long string that needs to be",
    param2="split across multiple lines",
    param3={
        "nested": "dictionary",
        "with": "values"
    }
)
def multiline_decorator_args():
    """Function with multiline decorator arguments."""
    pass

# Decorator on async function
@async_decorator
@retry(max_attempts=3)
async def decorated_async_function():
    """Async function with decorators."""
    return await fetch_data()

# Property with setter and deleter decorators
@property
def complex_property(self):
    """Property getter."""
    return self._value

@complex_property.setter
def complex_property(self, value):
    """Property setter."""
    self._value = value

@complex_property.deleter
def complex_property(self):
    """Property deleter."""
    del self._value

# Decorator with incomplete syntax (should not panic)
@incomplete_decorator(
def function_after_bad_decorator():
    """This might not be extracted correctly."""
    pass

# Valid function after malformed decorators
def valid_function():
    """This should be extracted correctly."""
    return "success"

# Class with unusual decorator pattern
@register_class
@validate_schema(strict=True)
@cache_results(ttl=3600)
class AdvancedDecoratedClass:
    """Class with multiple advanced decorators."""

    @staticmethod
    @memoize
    def static_decorated():
        """Static method with decorators."""
        return "static"

    @classmethod
    @validate_args
    def class_decorated(cls, arg):
        """Class method with decorators."""
        return f"class: {arg}"

    @property
    @cache_property
    def prop_decorated(self):
        """Property with decorators."""
        return self._cached_value
