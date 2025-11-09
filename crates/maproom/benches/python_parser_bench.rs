use crewchief_maproom::indexer::parser;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;
use std::time::Duration;

/// Benchmark parsing a simple Python function
fn bench_simple_function(c: &mut Criterion) {
    let source = r#"
def calculate_sum(a: int, b: int) -> int:
    """Calculate the sum of two integers.

    Args:
        a: First integer
        b: Second integer

    Returns:
        Sum of a and b
    """
    return a + b
"#;

    c.bench_function("parse_simple_function", |b| {
        b.iter(|| parser::extract_chunks(black_box(source), "py"));
    });
}

/// Benchmark parsing a simple class with methods
fn bench_simple_class(c: &mut Criterion) {
    let source = r#"
class Calculator:
    """A simple calculator class."""

    def __init__(self):
        """Initialize the calculator."""
        self.result = 0

    def add(self, a, b):
        """Add two numbers."""
        return a + b

    def subtract(self, a, b):
        """Subtract two numbers."""
        return a - b

    def multiply(self, a, b):
        """Multiply two numbers."""
        return a * b
"#;

    c.bench_function("parse_simple_class", |b| {
        b.iter(|| parser::extract_chunks(black_box(source), "py"));
    });
}

/// Benchmark parsing a complex dataclass with decorators
fn bench_complex_dataclass(c: &mut Criterion) {
    let source = r#"
from dataclasses import dataclass, field
from typing import List, Optional

@dataclass
class User:
    """User model with validation."""

    id: int
    username: str
    email: str
    is_active: bool = True
    roles: List[str] = field(default_factory=list)
    metadata: Optional[dict] = None

    def __post_init__(self):
        """Validate after initialization."""
        if not self.username:
            raise ValueError("Username required")

    @property
    def display_name(self) -> str:
        """Get display name."""
        return self.username.title()

    @classmethod
    def from_dict(cls, data: dict) -> 'User':
        """Create from dictionary."""
        return cls(**data)
"#;

    c.bench_function("parse_complex_dataclass", |b| {
        b.iter(|| parser::extract_chunks(black_box(source), "py"));
    });
}

/// Benchmark parsing file with multiple imports
fn bench_imports_heavy(c: &mut Criterion) {
    let source = r#"
import os
import sys
import json
from pathlib import Path
from typing import List, Dict, Optional, Union, Tuple
from dataclasses import dataclass
from abc import ABC, abstractmethod
import asyncio
from collections import defaultdict
from functools import wraps, lru_cache

def process_data(data: List[Dict]) -> Dict:
    """Process data."""
    return {}
"#;

    c.bench_function("parse_imports_heavy", |b| {
        b.iter(|| parser::extract_chunks(black_box(source), "py"));
    });
}

/// Benchmark parsing async functions
fn bench_async_functions(c: &mut Criterion) {
    let source = r#"
async def fetch_data(url: str) -> dict:
    """Fetch data from URL."""
    async with aiohttp.ClientSession() as session:
        async with session.get(url) as response:
            return await response.json()

async def process_batch(items: List[str]) -> List[dict]:
    """Process items in batch."""
    tasks = [fetch_data(item) for item in items]
    return await asyncio.gather(*tasks)
"#;

    c.bench_function("parse_async_functions", |b| {
        b.iter(|| parser::extract_chunks(black_box(source), "py"));
    });
}

/// Benchmark parsing nested classes
fn bench_nested_classes(c: &mut Criterion) {
    let source = r#"
class Outer:
    """Outer class."""

    class Middle:
        """Middle class."""

        class Inner:
            """Inner class."""

            def deep_method(self):
                """Deeply nested method."""
                return "deep"

        def middle_method(self):
            """Middle method."""
            return "middle"

    def outer_method(self):
        """Outer method."""
        return "outer"
"#;

    c.bench_function("parse_nested_classes", |b| {
        b.iter(|| parser::extract_chunks(black_box(source), "py"));
    });
}

/// Benchmark parsing real-world Django models file
fn bench_django_models(c: &mut Criterion) {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/models.py")
        .expect("Failed to read Django models fixture");

    let size = source.len();

    let mut group = c.benchmark_group("real_world");
    group.throughput(Throughput::Bytes(size as u64));
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("django_models", |b| {
        b.iter(|| parser::extract_chunks(black_box(&source), "py"));
    });

    group.finish();
}

/// Benchmark parsing real-world Django views file
fn bench_django_views(c: &mut Criterion) {
    let source = fs::read_to_string("tests/fixtures/python/django_samples/views.py")
        .expect("Failed to read Django views fixture");

    let size = source.len();

    let mut group = c.benchmark_group("real_world");
    group.throughput(Throughput::Bytes(size as u64));
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("django_views", |b| {
        b.iter(|| parser::extract_chunks(black_box(&source), "py"));
    });

    group.finish();
}

/// Benchmark parsing real-world Flask app file
fn bench_flask_app(c: &mut Criterion) {
    let source = fs::read_to_string("tests/fixtures/python/flask_samples/app.py")
        .expect("Failed to read Flask app fixture");

    let size = source.len();

    let mut group = c.benchmark_group("real_world");
    group.throughput(Throughput::Bytes(size as u64));
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("flask_app", |b| {
        b.iter(|| parser::extract_chunks(black_box(&source), "py"));
    });

    group.finish();
}

/// Benchmark parsing edge cases with incomplete syntax
fn bench_edge_cases_incomplete(c: &mut Criterion) {
    let source = fs::read_to_string("tests/fixtures/python/edge_cases/incomplete_syntax.py")
        .expect("Failed to read incomplete syntax fixture");

    c.bench_function("edge_cases_incomplete", |b| {
        b.iter(|| parser::extract_chunks(black_box(&source), "py"));
    });
}

/// Benchmark parsing varying file sizes
fn bench_file_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_sizes");

    // Small file (100 LOC equivalent)
    let small = r#"
def func1():
    """Function 1."""
    return 1

def func2():
    """Function 2."""
    return 2
"#
    .repeat(10);

    group.throughput(Throughput::Bytes(small.len() as u64));
    group.bench_with_input(BenchmarkId::new("small", small.len()), &small, |b, s| {
        b.iter(|| parser::extract_chunks(black_box(s), "py"));
    });

    // Medium file (500 LOC equivalent)
    let medium = small.repeat(5);
    group.throughput(Throughput::Bytes(medium.len() as u64));
    group.bench_with_input(BenchmarkId::new("medium", medium.len()), &medium, |b, s| {
        b.iter(|| parser::extract_chunks(black_box(s), "py"));
    });

    // Large file (1000 LOC equivalent)
    let large = small.repeat(10);
    group.throughput(Throughput::Bytes(large.len() as u64));
    group.bench_with_input(BenchmarkId::new("large", large.len()), &large, |b, s| {
        b.iter(|| parser::extract_chunks(black_box(s), "py"));
    });

    group.finish();
}

/// Benchmark batch parsing (simulating indexing multiple files)
fn bench_batch_parsing(c: &mut Criterion) {
    let files = vec![
        r#"
def func1():
    return 1
"#,
        r#"
class Class1:
    def method1(self):
        return 1
"#,
        r#"
async def async_func():
    return await something()
"#,
        r#"
@decorator
def decorated():
    return "decorated"
"#,
    ];

    let mut group = c.benchmark_group("batch_parsing");
    group.throughput(Throughput::Elements(files.len() as u64));

    group.bench_function("parse_4_files", |b| {
        b.iter(|| {
            for file in &files {
                parser::extract_chunks(black_box(file), "py");
            }
        });
    });

    group.finish();
}

/// Benchmark parsing with heavy decorator usage
fn bench_heavy_decorators(c: &mut Criterion) {
    let source = r#"
@decorator1
@decorator2
@decorator3
@decorator4
@decorator5
def heavily_decorated():
    """Function with many decorators."""
    pass

@dataclass
@validate_schema
@cache_result
@log_calls
class DecoratedClass:
    """Class with many decorators."""
    field1: str
    field2: int
"#;

    c.bench_function("parse_heavy_decorators", |b| {
        b.iter(|| parser::extract_chunks(black_box(source), "py"));
    });
}

/// Benchmark parsing with complex type hints
fn bench_complex_type_hints(c: &mut Criterion) {
    let source = r#"
from typing import Dict, List, Optional, Union, Callable, TypeVar, Generic

T = TypeVar('T')

def complex_types(
    param1: Dict[str, List[Optional[int]]],
    param2: Union[str, int, None],
    param3: Callable[[int, str], bool],
) -> Optional[Dict[str, Union[List[int], Tuple[str, ...]]]]:
    """Function with complex type hints."""
    pass

class GenericContainer(Generic[T]):
    """Generic class."""

    def process(self, item: T) -> Optional[T]:
        """Process item."""
        return item
"#;

    c.bench_function("parse_complex_type_hints", |b| {
        b.iter(|| parser::extract_chunks(black_box(source), "py"));
    });
}

/// Benchmark comparison: Python vs TypeScript parsing speed
fn bench_language_comparison(c: &mut Criterion) {
    let python_source = r#"
class Calculator:
    """Calculator class."""

    def add(self, a: int, b: int) -> int:
        """Add two numbers."""
        return a + b
"#;

    let typescript_source = r#"
class Calculator {
    /**
     * Add two numbers
     */
    add(a: number, b: number): number {
        return a + b;
    }
}
"#;

    let mut group = c.benchmark_group("language_comparison");

    group.bench_function("python", |b| {
        b.iter(|| parser::extract_chunks(black_box(python_source), "py"));
    });

    group.bench_function("typescript", |b| {
        b.iter(|| parser::extract_chunks(black_box(typescript_source), "ts"));
    });

    group.finish();
}

/// Benchmark parsing files/minute metric (for acceptance criteria)
fn bench_files_per_minute(c: &mut Criterion) {
    // Average Python file ~200 lines
    let average_file = fs::read_to_string("tests/fixtures/python/sample_api.py")
        .expect("Failed to read sample API fixture");

    let mut group = c.benchmark_group("throughput");
    group.throughput(Throughput::Elements(1)); // 1 file

    group.bench_function("files_per_minute_estimate", |b| {
        b.iter(|| parser::extract_chunks(black_box(&average_file), "py"));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_simple_function,
    bench_simple_class,
    bench_complex_dataclass,
    bench_imports_heavy,
    bench_async_functions,
    bench_nested_classes,
    bench_django_models,
    bench_django_views,
    bench_flask_app,
    bench_edge_cases_incomplete,
    bench_file_sizes,
    bench_batch_parsing,
    bench_heavy_decorators,
    bench_complex_type_hints,
    bench_language_comparison,
    bench_files_per_minute,
);

criterion_main!(benches);
