# Edge case: Unusual class patterns and complex hierarchies

# Metaclass usage
class CustomMeta(type):
    """A custom metaclass."""
    def __new__(mcs, name, bases, namespace):
        return super().__new__(mcs, name, bases, namespace)

class MetaclassUser(metaclass=CustomMeta):
    """Class using a custom metaclass."""
    pass

# Multiple inheritance with complex MRO
class A:
    """Base class A."""
    pass

class B(A):
    """Class B inherits from A."""
    pass

class C(A):
    """Class C inherits from A."""
    pass

class D(B, C):
    """Diamond inheritance pattern."""
    pass

# Deeply nested classes
class Outer:
    """Outer class."""

    class Middle:
        """Middle nested class."""

        class Inner:
            """Deeply nested class."""

            class DeepestInner:
                """Deepest nested class."""

                def method(self):
                    """Method in deepest class."""
                    return "deep"

# Class with __slots__
class SlottedClass:
    """Class using __slots__."""
    __slots__ = ['x', 'y', 'z']

    def __init__(self, x, y, z):
        self.x = x
        self.y = y
        self.z = z

# Abstract base class
from abc import ABC, abstractmethod

class AbstractBase(ABC):
    """Abstract base class."""

    @abstractmethod
    def must_implement(self):
        """Subclasses must implement this."""
        pass

    @abstractmethod
    async def async_must_implement(self):
        """Async method that must be implemented."""
        pass

# Class with multiple abstract methods
class ComplexAbstract(ABC):
    """Complex abstract class."""

    @abstractmethod
    def method1(self):
        pass

    @abstractmethod
    def method2(self):
        pass

    @property
    @abstractmethod
    def abstract_property(self):
        pass

# Generic class with type parameters
from typing import Generic, TypeVar

T = TypeVar('T')
U = TypeVar('U')

class GenericClass(Generic[T]):
    """Generic class with type parameter."""

    def process(self, item: T) -> T:
        """Process item of generic type."""
        return item

class MultiGeneric(Generic[T, U]):
    """Class with multiple type parameters."""

    def combine(self, first: T, second: U) -> tuple:
        """Combine items of different types."""
        return (first, second)

# Protocol class
from typing import Protocol

class Drawable(Protocol):
    """Protocol for drawable objects."""

    def draw(self) -> None:
        """Draw the object."""
        ...

# Dataclass with complex features
from dataclasses import dataclass, field

@dataclass(frozen=True, order=True)
class ComplexDataclass:
    """Dataclass with advanced features."""
    id: int
    name: str
    metadata: dict = field(default_factory=dict)
    _private: str = field(default="private", init=False)

# Class with __init_subclass__
class PluginBase:
    """Base class for plugins."""

    registry = {}

    def __init_subclass__(cls, plugin_name=None, **kwargs):
        """Register subclasses automatically."""
        super().__init_subclass__(**kwargs)
        if plugin_name:
            cls.registry[plugin_name] = cls

class MyPlugin(PluginBase, plugin_name="my_plugin"):
    """Plugin implementation."""
    pass

# Class with complex property patterns
class PropertyComplex:
    """Class with complex property usage."""

    def __init__(self):
        self._x = 0
        self._y = 0

    @property
    def x(self):
        """X coordinate."""
        return self._x

    @x.setter
    def x(self, value):
        self._x = max(0, value)

    @x.deleter
    def x(self):
        self._x = 0

    @property
    def magnitude(self):
        """Computed read-only property."""
        return (self._x ** 2 + self._y ** 2) ** 0.5

# Exception hierarchy
class BaseError(Exception):
    """Base exception."""
    pass

class SpecificError(BaseError):
    """Specific error type."""
    pass

class VerySpecificError(SpecificError):
    """Very specific error type."""
    pass

# Context manager class
class ContextManager:
    """Context manager with enter/exit."""

    def __enter__(self):
        """Enter context."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Exit context."""
        return False

# Async context manager
class AsyncContextManager:
    """Async context manager."""

    async def __aenter__(self):
        """Async enter."""
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async exit."""
        return False

# Iterator class
class CustomIterator:
    """Custom iterator implementation."""

    def __init__(self, data):
        self.data = data
        self.index = 0

    def __iter__(self):
        return self

    def __next__(self):
        if self.index >= len(self.data):
            raise StopIteration
        value = self.data[self.index]
        self.index += 1
        return value

# Descriptor class
class Descriptor:
    """Descriptor protocol implementation."""

    def __get__(self, obj, objtype=None):
        return self.value

    def __set__(self, obj, value):
        self.value = value

    def __delete__(self, obj):
        del self.value
