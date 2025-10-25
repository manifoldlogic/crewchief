use crewchief_maproom::indexer::parser;

#[test]
fn test_go_function_parsing() {
    let source = r#"
package main

// Add adds two integers and returns the result
func Add(a int, b int) int {
    return a + b
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Should extract package and function
    assert!(chunks.len() >= 2, "Expected at least 2 chunks (package + function), got {}", chunks.len());

    // Find the function chunk
    let func_chunk = chunks.iter().find(|c| c.kind == "func");
    assert!(func_chunk.is_some(), "Function chunk not found");

    let func = func_chunk.unwrap();
    assert_eq!(func.symbol_name, Some("Add".to_string()));
    assert!(func.signature.is_some());
    assert!(func.docstring.is_some());
    assert!(func.docstring.as_ref().unwrap().contains("Add adds two integers"));
}

#[test]
fn test_go_struct_parsing() {
    let source = r#"
package main

// User represents a user in the system
type User struct {
    Name string
    Age  int
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find the struct chunk
    let struct_chunk = chunks.iter().find(|c| c.kind == "struct");
    assert!(struct_chunk.is_some(), "Struct chunk not found");

    let s = struct_chunk.unwrap();
    assert_eq!(s.symbol_name, Some("User".to_string()));
    assert!(s.docstring.is_some());
    assert!(s.docstring.as_ref().unwrap().contains("User represents"));
}

#[test]
fn test_go_interface_parsing() {
    let source = r#"
package main

// Reader is the interface for reading data
type Reader interface {
    Read(p []byte) (n int, err error)
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find the interface chunk
    let interface_chunk = chunks.iter().find(|c| c.kind == "interface");
    assert!(interface_chunk.is_some(), "Interface chunk not found");

    let i = interface_chunk.unwrap();
    assert_eq!(i.symbol_name, Some("Reader".to_string()));
    assert!(i.docstring.is_some());
}

#[test]
fn test_go_method_parsing() {
    let source = r#"
package main

type User struct {
    Name string
}

// GetName returns the user's name
func (u *User) GetName() string {
    return u.Name
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find the method chunk
    let method_chunk = chunks.iter().find(|c| c.kind == "method");
    assert!(method_chunk.is_some(), "Method chunk not found");

    let m = method_chunk.unwrap();
    assert_eq!(m.symbol_name, Some("GetName".to_string()));
    assert!(m.metadata.is_some());
    // Check that receiver is in metadata
    let metadata = m.metadata.as_ref().unwrap();
    assert!(metadata.get("receiver").is_some());
}

#[test]
fn test_go_constants_parsing() {
    let source = r#"
package main

// MaxRetries is the maximum number of retry attempts
const MaxRetries = 3

const (
    // StatusOK indicates success
    StatusOK = 200
    // StatusError indicates an error
    StatusError = 500
)
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find constant chunks
    let const_chunks: Vec<_> = chunks.iter().filter(|c| c.kind == "constant").collect();
    assert!(const_chunks.len() >= 3, "Expected at least 3 constants, got {}", const_chunks.len());

    // Check that MaxRetries is present
    let max_retries = const_chunks.iter().find(|c| c.symbol_name.as_deref() == Some("MaxRetries"));
    assert!(max_retries.is_some(), "MaxRetries constant not found");
}

#[test]
fn test_go_variables_parsing() {
    let source = r#"
package main

// DefaultTimeout is the default timeout duration
var DefaultTimeout = 30

var (
    // Config holds the configuration
    Config map[string]string
    // Logger is the application logger
    Logger interface{}
)
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find variable chunks
    let var_chunks: Vec<_> = chunks.iter().filter(|c| c.kind == "variable").collect();
    assert!(var_chunks.len() >= 3, "Expected at least 3 variables, got {}", var_chunks.len());
}

#[test]
fn test_go_package_parsing() {
    let source = r#"
package main

func main() {
    println("Hello, World!")
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find package chunk
    let package_chunk = chunks.iter().find(|c| c.kind == "package");
    assert!(package_chunk.is_some(), "Package chunk not found");

    let p = package_chunk.unwrap();
    assert_eq!(p.symbol_name, Some("main".to_string()));
}

#[test]
fn test_go_goroutines_dont_crash() {
    // Test that code with goroutines is parsed and metadata is extracted
    let source = r#"
package main

func processData() {
    go func() {
        // Anonymous goroutine
        println("processing")
    }()

    ch := make(chan int)
    go worker(ch)
}

func worker(ch chan int) {
    for v := range ch {
        println(v)
    }
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Should successfully parse without crashing
    assert!(chunks.len() > 0, "Expected some chunks to be extracted");

    // Should find the functions
    let func_chunks: Vec<_> = chunks.iter().filter(|c| c.kind == "func").collect();
    assert!(func_chunks.len() >= 2, "Expected at least 2 functions");

    // Check that processData has goroutine metadata
    let process_data = func_chunks.iter().find(|c| c.symbol_name.as_deref() == Some("processData"));
    assert!(process_data.is_some(), "processData function not found");
    let process_data = process_data.unwrap();
    assert!(process_data.metadata.is_some(), "processData should have metadata");
    let metadata = process_data.metadata.as_ref().unwrap();
    assert_eq!(metadata.get("has_goroutines"), Some(&serde_json::json!(true)), "processData should have has_goroutines flag");
    assert_eq!(metadata.get("has_channels"), Some(&serde_json::json!(true)), "processData should have has_channels flag");

    // Check that worker has channel metadata
    let worker = func_chunks.iter().find(|c| c.symbol_name.as_deref() == Some("worker"));
    assert!(worker.is_some(), "worker function not found");
    let worker = worker.unwrap();
    assert!(worker.metadata.is_some(), "worker should have metadata");
    let metadata = worker.metadata.as_ref().unwrap();
    assert_eq!(metadata.get("has_channels"), Some(&serde_json::json!(true)), "worker should have has_channels flag");
}

#[test]
fn test_go_channels_dont_crash() {
    // Test that code with channels is parsed and metadata is extracted
    let source = r#"
package main

func main() {
    ch := make(chan int, 10)
    ch <- 42
    value := <-ch
    close(ch)
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Should successfully parse without crashing
    assert!(chunks.len() > 0, "Expected some chunks to be extracted");

    // Check that main has channel metadata
    let main_func = chunks.iter().find(|c| c.kind == "func" && c.symbol_name.as_deref() == Some("main"));
    assert!(main_func.is_some(), "main function not found");
    let main_func = main_func.unwrap();
    assert!(main_func.metadata.is_some(), "main should have metadata");
    let metadata = main_func.metadata.as_ref().unwrap();
    assert_eq!(metadata.get("has_channels"), Some(&serde_json::json!(true)), "main should have has_channels flag");
}

#[test]
fn test_go_empty_file() {
    let source = "";
    let chunks = parser::extract_chunks(source, "go");
    assert_eq!(chunks.len(), 0, "Empty file should produce no chunks");
}

#[test]
fn test_go_malformed_code() {
    // Test that malformed code doesn't crash the parser
    let source = r#"
package main

func broken( {
    // Missing closing parenthesis and brace
"#;

    let _chunks = parser::extract_chunks(source, "go");

    // Should not crash, may or may not extract chunks depending on tree-sitter's error recovery
    // The important thing is it doesn't panic
    assert!(true, "Parser should handle malformed code gracefully");
}

#[test]
fn test_gomod_parsing() {
    let source = r#"
module github.com/example/myproject

go 1.21

require (
    github.com/pkg/errors v0.9.1
    golang.org/x/sync v0.3.0
)
"#;

    let chunks = parser::extract_chunks(source, "gomod");

    // Should extract module name, go version, and dependencies
    assert!(chunks.len() >= 3, "Expected at least 3 chunks (module + go version + 2 requires), got {}", chunks.len());

    // Check module chunk
    let module_chunk = chunks.iter().find(|c| c.kind == "module");
    assert!(module_chunk.is_some(), "Module chunk not found");
    let module = module_chunk.unwrap();
    assert_eq!(module.symbol_name, Some("github.com/example/myproject".to_string()));

    // Check go version chunk
    let version_chunk = chunks.iter().find(|c| c.kind == "go_version");
    assert!(version_chunk.is_some(), "Go version chunk not found");

    // Check require chunks
    let require_chunks: Vec<_> = chunks.iter().filter(|c| c.kind == "require").collect();
    assert!(require_chunks.len() >= 2, "Expected at least 2 require chunks, got {}", require_chunks.len());
}

#[test]
fn test_go_import_extraction() {
    let source = r#"
package main

import "fmt"
import "os"

import (
    "context"
    "encoding/json"
    myalias "github.com/example/pkg"
    . "github.com/another/pkg"
)

func main() {
    fmt.Println("Hello")
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find all import chunks
    let import_chunks: Vec<_> = chunks.iter().filter(|c| c.kind == "import").collect();
    assert!(import_chunks.len() >= 6, "Expected at least 6 imports, got {}", import_chunks.len());

    // Check single import
    let fmt_import = import_chunks.iter().find(|c| c.signature.as_deref() == Some("fmt"));
    assert!(fmt_import.is_some(), "fmt import not found");
    let fmt_import = fmt_import.unwrap();
    assert_eq!(fmt_import.symbol_name, Some("fmt".to_string()));
    assert!(fmt_import.metadata.is_some(), "fmt import should have metadata");
    let metadata = fmt_import.metadata.as_ref().unwrap();
    assert_eq!(metadata.get("import_path"), Some(&serde_json::json!("fmt")));

    // Check aliased import
    let aliased_import = import_chunks.iter().find(|c| {
        c.metadata.as_ref()
            .and_then(|m| m.get("alias"))
            .and_then(|a| a.as_str())
            == Some("myalias")
    });
    assert!(aliased_import.is_some(), "Aliased import not found");
    let aliased_import = aliased_import.unwrap();
    assert_eq!(aliased_import.symbol_name, Some("myalias".to_string()));
    let metadata = aliased_import.metadata.as_ref().unwrap();
    assert_eq!(metadata.get("import_path"), Some(&serde_json::json!("github.com/example/pkg")));
    assert_eq!(metadata.get("alias"), Some(&serde_json::json!("myalias")));

    // Check dot import
    let dot_import = import_chunks.iter().find(|c| {
        c.metadata.as_ref()
            .and_then(|m| m.get("alias"))
            .and_then(|a| a.as_str())
            == Some(".")
    });
    assert!(dot_import.is_some(), "Dot import not found");
}
