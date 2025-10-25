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
    // Test that code with goroutines doesn't crash the parser
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
}

#[test]
fn test_go_channels_dont_crash() {
    // Test that code with channels doesn't crash the parser
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
