//! MLLANG-1005.3006: C Parser Performance Benchmarks
//!
//! Criterion benchmarks for C parser performance testing across different file sizes.
//! These benchmarks provide baseline metrics for parser scalability.

use crewchief_maproom::indexer::parser;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Generate realistic C source code of approximately the specified size
fn generate_c_source(size_kb: usize) -> String {
    let target_bytes = size_kb * 1024;
    let mut source = String::with_capacity(target_bytes);

    // Standard header includes
    source.push_str("#include <stdio.h>\n");
    source.push_str("#include <stdlib.h>\n");
    source.push_str("#include <string.h>\n");
    source.push_str("#include <stdint.h>\n\n");

    // Struct definitions
    source.push_str("/**\n");
    source.push_str(" * User data structure\n");
    source.push_str(" */\n");
    source.push_str("typedef struct {\n");
    source.push_str("    int id;\n");
    source.push_str("    char name[256];\n");
    source.push_str("    char email[256];\n");
    source.push_str("    int age;\n");
    source.push_str("} User;\n\n");

    // Enum definitions
    source.push_str("/**\n");
    source.push_str(" * Status codes\n");
    source.push_str(" */\n");
    source.push_str("enum Status {\n");
    source.push_str("    STATUS_OK = 0,\n");
    source.push_str("    STATUS_ERROR = 1,\n");
    source.push_str("    STATUS_PENDING = 2,\n");
    source.push_str("    STATUS_TIMEOUT = 3\n");
    source.push_str("};\n\n");

    // Generate functions until we reach target size
    let mut func_count = 0;
    while source.len() < target_bytes {
        func_count += 1;

        // Function with doc comment
        source.push_str("/**\n");
        source.push_str(&format!(" * Process data function number {}\n", func_count));
        source.push_str(" * \n");
        source.push_str(&format!(" * @param data Pointer to data_{}\n", func_count));
        source.push_str(" * @param size Size of the data\n");
        source.push_str(" * @return Status code indicating success or failure\n");
        source.push_str(" */\n");
        source.push_str(&format!(
            "int process_data_{}(void* data, size_t size) {{\n",
            func_count
        ));
        source.push_str("    if (data == NULL || size == 0) {\n");
        source.push_str("        return STATUS_ERROR;\n");
        source.push_str("    }\n");
        source.push_str("    \n");
        source.push_str("    // Validate input\n");
        source.push_str("    if (size > 1024 * 1024) {\n");
        source.push_str("        fprintf(stderr, \"Data too large\\n\");\n");
        source.push_str("        return STATUS_ERROR;\n");
        source.push_str("    }\n");
        source.push_str("    \n");
        source.push_str("    // Process data\n");
        source.push_str("    char* buffer = (char*)malloc(size);\n");
        source.push_str("    if (buffer == NULL) {\n");
        source.push_str("        return STATUS_ERROR;\n");
        source.push_str("    }\n");
        source.push_str("    memcpy(buffer, data, size);\n");
        source.push_str("    \n");
        source.push_str("    // Clean up\n");
        source.push_str("    free(buffer);\n");
        source.push_str("    return STATUS_OK;\n");
        source.push_str("}\n\n");

        // Add some static functions for variety
        if func_count % 3 == 0 {
            source.push_str(&format!(
                "static void helper_function_{}(int param) {{\n",
                func_count
            ));
            source.push_str("    printf(\"Helper %d\\n\", param);\n");
            source.push_str("}\n\n");
        }

        // Add function declarations
        if func_count % 5 == 0 {
            source.push_str(&format!(
                "extern int external_function_{}(const char* str);\n\n",
                func_count
            ));
        }

        // Add global variables
        if func_count % 7 == 0 {
            source.push_str(&format!("int global_counter_{} = 0;\n", func_count));
            source.push_str(&format!(
                "static const char* global_name_{} = \"value\";\n\n",
                func_count
            ));
        }

        // Add additional structs for variety
        if func_count % 10 == 0 {
            source.push_str(&format!("struct Data_{} {{\n", func_count));
            source.push_str("    int field1;\n");
            source.push_str("    int field2;\n");
            source.push_str("    char field3[64];\n");
            source.push_str("};\n\n");
        }
    }

    source
}

/// Benchmark parsing C files of varying sizes (1KB, 10KB, 100KB, 1MB)
fn bench_parse_c_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("c_parser_size_scaling");

    for size_kb in [1, 10, 100, 1000].iter() {
        let source = generate_c_source(*size_kb);
        let actual_size = source.len();

        group.throughput(Throughput::Bytes(actual_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size_kb)),
            &source,
            |b, s| {
                b.iter(|| parser::extract_chunks(black_box(s), "c"));
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_parse_c_by_size);

criterion_main!(benches);
