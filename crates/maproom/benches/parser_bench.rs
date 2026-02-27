/// MD_ENHANCE-4002: Parser Benchmarks
///
/// Criterion benchmarks for markdown parser performance testing.
/// These benchmarks provide detailed performance metrics and regression detection.
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use maproom::indexer::parser;
use std::fs;

fn bench_small_document(c: &mut Criterion) {
    let content = r#"# Small Document

## Section 1

Some content here.

```rust
fn test() {
    println!("Hello");
}
```

## Section 2

More content.

- Item 1
- Item 2
- Item 3
"#;

    let mut group = c.benchmark_group("small_document");
    group.throughput(Throughput::Bytes(content.len() as u64));

    group.bench_function("parse", |b| {
        b.iter(|| parser::extract_chunks(black_box(content), black_box("md")));
    });

    group.finish();
}

fn bench_medium_document(c: &mut Criterion) {
    let mut content = String::from("# Medium Document\n\n");

    for i in 1..=50 {
        content.push_str(&format!("## Section {}\n\n", i));
        content.push_str("This section has some content.\n");
        content.push_str("Multiple paragraphs to test parsing.\n\n");

        if i % 5 == 0 {
            content.push_str(&format!(
                "```typescript\nfunction example{}() {{\n    return {};\n}}\n```\n\n",
                i, i
            ));
        }

        if i % 3 == 0 {
            content.push_str("- Feature A\n- Feature B\n- Feature C\n\n");
        }
    }

    let mut group = c.benchmark_group("medium_document");
    group.throughput(Throughput::Bytes(content.len() as u64));

    group.bench_function("parse", |b| {
        b.iter(|| parser::extract_chunks(black_box(&content), black_box("md")));
    });

    group.finish();
}

fn bench_large_document(c: &mut Criterion) {
    let mut content = String::from("# Large Document\n\n");

    for i in 1..=500 {
        content.push_str(&format!("## Section {}\n\n", i));
        content.push_str("Content for this section.\n");
        content.push_str("Additional content line.\n\n");

        if i % 10 == 0 {
            content.push_str(&format!(
                "```rust\nfn function_{}() {{\n    println!(\"test\");\n}}\n```\n\n",
                i
            ));
        }
    }

    let mut group = c.benchmark_group("large_document");
    group.throughput(Throughput::Bytes(content.len() as u64));

    group.bench_function("parse", |b| {
        b.iter(|| parser::extract_chunks(black_box(&content), black_box("md")));
    });

    group.finish();
}

fn bench_code_block_heavy(c: &mut Criterion) {
    let mut content = String::from("# Code Examples\n\n");

    for i in 1..=100 {
        content.push_str(&format!("## Example {}\n\n", i));
        content.push_str(&format!("```typescript\nconst example{} = () => {{\n", i));
        content.push_str("    const x = 1;\n");
        content.push_str("    const y = 2;\n");
        content.push_str("    return x + y;\n");
        content.push_str("};\n```\n\n");
    }

    let mut group = c.benchmark_group("code_block_heavy");
    group.throughput(Throughput::Bytes(content.len() as u64));

    group.bench_function("parse", |b| {
        b.iter(|| parser::extract_chunks(black_box(&content), black_box("md")));
    });

    group.finish();
}

fn bench_heading_heavy(c: &mut Criterion) {
    let mut content = String::from("# Root\n\n");

    for i in 1..=100 {
        content.push_str(&format!("## Level 2 - {}\n\n", i));
        content.push_str(&format!("### Level 3 - {}.1\n\n", i));
        content.push_str(&format!("### Level 3 - {}.2\n\n", i));
        content.push_str(&format!("#### Level 4 - {}.2.1\n\n", i));
    }

    let mut group = c.benchmark_group("heading_heavy");
    group.throughput(Throughput::Bytes(content.len() as u64));

    group.bench_function("parse", |b| {
        b.iter(|| parser::extract_chunks(black_box(&content), black_box("md")));
    });

    group.finish();
}

fn bench_real_readme(c: &mut Criterion) {
    if let Ok(content) = fs::read_to_string("/workspace/README.md") {
        let mut group = c.benchmark_group("real_readme");
        group.throughput(Throughput::Bytes(content.len() as u64));

        group.bench_function("parse", |b| {
            b.iter(|| parser::extract_chunks(black_box(&content), black_box("md")));
        });

        group.finish();
    }
}

fn bench_real_claude_md(c: &mut Criterion) {
    if let Ok(content) = fs::read_to_string("/workspace/CLAUDE.md") {
        let mut group = c.benchmark_group("real_claude_md");
        group.throughput(Throughput::Bytes(content.len() as u64));

        group.bench_function("parse", |b| {
            b.iter(|| parser::extract_chunks(black_box(&content), black_box("md")));
        });

        group.finish();
    }
}

fn bench_document_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("document_sizes");

    for size in [10, 50, 100, 500, 1000].iter() {
        let mut content = String::from("# Document\n\n");

        for i in 1..=*size {
            content.push_str(&format!("## Section {}\n\nContent.\n\n", i));
        }

        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| parser::extract_chunks(black_box(&content), black_box("md")));
        });
    }

    group.finish();
}

fn bench_element_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("element_types");

    // Heading-only document
    let mut headings_only = String::from("# Document\n\n");
    for i in 1..=100 {
        headings_only.push_str(&format!("## Section {}\n\n", i));
    }

    group.throughput(Throughput::Bytes(headings_only.len() as u64));
    group.bench_function("headings_only", |b| {
        b.iter(|| parser::extract_chunks(black_box(&headings_only), black_box("md")));
    });

    // Code-only document
    let mut code_only = String::from("# Code\n\n");
    for i in 1..=50 {
        code_only.push_str(&format!("```rust\nfn test_{}() {{}}\n```\n\n", i));
    }

    group.throughput(Throughput::Bytes(code_only.len() as u64));
    group.bench_function("code_only", |b| {
        b.iter(|| parser::extract_chunks(black_box(&code_only), black_box("md")));
    });

    // List-only document
    let mut list_only = String::from("# Lists\n\n");
    for _ in 1..=20 {
        list_only.push_str("- Item 1\n- Item 2\n- Item 3\n- Item 4\n- Item 5\n\n");
    }

    group.throughput(Throughput::Bytes(list_only.len() as u64));
    group.bench_function("list_only", |b| {
        b.iter(|| parser::extract_chunks(black_box(&list_only), black_box("md")));
    });

    // Table-only document
    let mut table_only = String::from("# Tables\n\n");
    for i in 1..=20 {
        table_only.push_str(&format!(
            "| Col1 | Col2 | Col3 |\n|------|------|------|\n| {} | {} | {} |\n\n",
            i,
            i * 2,
            i * 3
        ));
    }

    group.throughput(Throughput::Bytes(table_only.len() as u64));
    group.bench_function("table_only", |b| {
        b.iter(|| parser::extract_chunks(black_box(&table_only), black_box("md")));
    });

    group.finish();
}

fn bench_nested_hierarchy(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_hierarchy");

    // Shallow nesting (h1 > h2)
    let mut shallow = String::from("# Root\n\n");
    for i in 1..=100 {
        shallow.push_str(&format!("## Section {}\n\nContent.\n\n", i));
    }

    group.throughput(Throughput::Bytes(shallow.len() as u64));
    group.bench_function("shallow", |b| {
        b.iter(|| parser::extract_chunks(black_box(&shallow), black_box("md")));
    });

    // Deep nesting (h1 > h2 > h3 > h4)
    let mut deep = String::from("# Root\n\n");
    for i in 1..=25 {
        deep.push_str(&format!(
            "## L2-{}\n\n### L3-{}\n\n#### L4-{}\n\nContent.\n\n",
            i, i, i
        ));
    }

    group.throughput(Throughput::Bytes(deep.len() as u64));
    group.bench_function("deep", |b| {
        b.iter(|| parser::extract_chunks(black_box(&deep), black_box("md")));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_small_document,
    bench_medium_document,
    bench_large_document,
    bench_code_block_heavy,
    bench_heading_heavy,
    bench_real_readme,
    bench_real_claude_md,
    bench_document_sizes,
    bench_element_types,
    bench_nested_hierarchy
);

criterion_main!(benches);
