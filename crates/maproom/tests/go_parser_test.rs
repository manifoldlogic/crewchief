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
    assert!(
        chunks.len() >= 2,
        "Expected at least 2 chunks (package + function), got {}",
        chunks.len()
    );

    // Find the function chunk
    let func_chunk = chunks.iter().find(|c| c.kind == "func");
    assert!(func_chunk.is_some(), "Function chunk not found");

    let func = func_chunk.unwrap();
    assert_eq!(func.symbol_name, Some("Add".to_string()));
    assert!(func.signature.is_some());
    assert!(func.docstring.is_some());
    assert!(func
        .docstring
        .as_ref()
        .unwrap()
        .contains("Add adds two integers"));
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
    assert!(
        const_chunks.len() >= 3,
        "Expected at least 3 constants, got {}",
        const_chunks.len()
    );

    // Check that MaxRetries is present
    let max_retries = const_chunks
        .iter()
        .find(|c| c.symbol_name.as_deref() == Some("MaxRetries"));
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
    assert!(
        var_chunks.len() >= 3,
        "Expected at least 3 variables, got {}",
        var_chunks.len()
    );
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
    let process_data = func_chunks
        .iter()
        .find(|c| c.symbol_name.as_deref() == Some("processData"));
    assert!(process_data.is_some(), "processData function not found");
    let process_data = process_data.unwrap();
    assert!(
        process_data.metadata.is_some(),
        "processData should have metadata"
    );
    let metadata = process_data.metadata.as_ref().unwrap();
    assert_eq!(
        metadata.get("has_goroutines"),
        Some(&serde_json::json!(true)),
        "processData should have has_goroutines flag"
    );
    assert_eq!(
        metadata.get("has_channels"),
        Some(&serde_json::json!(true)),
        "processData should have has_channels flag"
    );

    // Check that worker has channel metadata
    let worker = func_chunks
        .iter()
        .find(|c| c.symbol_name.as_deref() == Some("worker"));
    assert!(worker.is_some(), "worker function not found");
    let worker = worker.unwrap();
    assert!(worker.metadata.is_some(), "worker should have metadata");
    let metadata = worker.metadata.as_ref().unwrap();
    assert_eq!(
        metadata.get("has_channels"),
        Some(&serde_json::json!(true)),
        "worker should have has_channels flag"
    );
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
    let main_func = chunks
        .iter()
        .find(|c| c.kind == "func" && c.symbol_name.as_deref() == Some("main"));
    assert!(main_func.is_some(), "main function not found");
    let main_func = main_func.unwrap();
    assert!(main_func.metadata.is_some(), "main should have metadata");
    let metadata = main_func.metadata.as_ref().unwrap();
    assert_eq!(
        metadata.get("has_channels"),
        Some(&serde_json::json!(true)),
        "main should have has_channels flag"
    );
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
    assert!(
        chunks.len() >= 3,
        "Expected at least 3 chunks (module + go version + 2 requires), got {}",
        chunks.len()
    );

    // Check module chunk
    let module_chunk = chunks.iter().find(|c| c.kind == "module");
    assert!(module_chunk.is_some(), "Module chunk not found");
    let module = module_chunk.unwrap();
    assert_eq!(
        module.symbol_name,
        Some("github.com/example/myproject".to_string())
    );

    // Check go version chunk
    let version_chunk = chunks.iter().find(|c| c.kind == "go_version");
    assert!(version_chunk.is_some(), "Go version chunk not found");

    // Check require chunks
    let require_chunks: Vec<_> = chunks.iter().filter(|c| c.kind == "require").collect();
    assert!(
        require_chunks.len() >= 2,
        "Expected at least 2 require chunks, got {}",
        require_chunks.len()
    );
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
    assert!(
        import_chunks.len() >= 6,
        "Expected at least 6 imports, got {}",
        import_chunks.len()
    );

    // Check single import
    let fmt_import = import_chunks
        .iter()
        .find(|c| c.signature.as_deref() == Some("fmt"));
    assert!(fmt_import.is_some(), "fmt import not found");
    let fmt_import = fmt_import.unwrap();
    assert_eq!(fmt_import.symbol_name, Some("fmt".to_string()));
    assert!(
        fmt_import.metadata.is_some(),
        "fmt import should have metadata"
    );
    let metadata = fmt_import.metadata.as_ref().unwrap();
    assert_eq!(metadata.get("import_path"), Some(&serde_json::json!("fmt")));

    // Check aliased import
    let aliased_import = import_chunks.iter().find(|c| {
        c.metadata
            .as_ref()
            .and_then(|m| m.get("alias"))
            .and_then(|a| a.as_str())
            == Some("myalias")
    });
    assert!(aliased_import.is_some(), "Aliased import not found");
    let aliased_import = aliased_import.unwrap();
    assert_eq!(aliased_import.symbol_name, Some("myalias".to_string()));
    let metadata = aliased_import.metadata.as_ref().unwrap();
    assert_eq!(
        metadata.get("import_path"),
        Some(&serde_json::json!("github.com/example/pkg"))
    );
    assert_eq!(metadata.get("alias"), Some(&serde_json::json!("myalias")));

    // Check dot import
    let dot_import = import_chunks.iter().find(|c| {
        c.metadata
            .as_ref()
            .and_then(|m| m.get("alias"))
            .and_then(|a| a.as_str())
            == Some(".")
    });
    assert!(dot_import.is_some(), "Dot import not found");
}

// Tests for Go conventions (LANG_PARSE-3003)

#[test]
fn test_go_exported_vs_unexported_functions() {
    let source = r#"
package main

// PublicFunc is exported (PascalCase)
func PublicFunc() {
}

// privateFunc is unexported (camelCase)
func privateFunc() {
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find the exported function
    let public_func = chunks
        .iter()
        .find(|c| c.kind == "func" && c.symbol_name.as_deref() == Some("PublicFunc"));
    assert!(public_func.is_some(), "PublicFunc not found");
    let public_func = public_func.unwrap();
    assert!(
        public_func.metadata.is_some(),
        "PublicFunc should have metadata"
    );
    let metadata = public_func.metadata.as_ref().unwrap();
    assert_eq!(
        metadata.get("visibility"),
        Some(&serde_json::json!("exported")),
        "PublicFunc should be exported"
    );

    // Find the unexported function
    let private_func = chunks
        .iter()
        .find(|c| c.kind == "func" && c.symbol_name.as_deref() == Some("privateFunc"));
    assert!(private_func.is_some(), "privateFunc not found");
    let private_func = private_func.unwrap();
    assert!(
        private_func.metadata.is_some(),
        "privateFunc should have metadata"
    );
    let metadata = private_func.metadata.as_ref().unwrap();
    assert_eq!(
        metadata.get("visibility"),
        Some(&serde_json::json!("unexported")),
        "privateFunc should be unexported"
    );
}

#[test]
fn test_go_exported_vs_unexported_types() {
    let source = r#"
package main

// PublicStruct is exported
type PublicStruct struct {
    Name string
}

// privateType is unexported
type privateType int
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find the exported struct
    let public_struct = chunks
        .iter()
        .find(|c| c.symbol_name.as_deref() == Some("PublicStruct"));
    assert!(public_struct.is_some(), "PublicStruct not found");
    let public_struct = public_struct.unwrap();
    assert!(
        public_struct.metadata.is_some(),
        "PublicStruct should have metadata"
    );
    let metadata = public_struct.metadata.as_ref().unwrap();
    assert_eq!(
        metadata.get("visibility"),
        Some(&serde_json::json!("exported")),
        "PublicStruct should be exported"
    );

    // Find the unexported type
    let private_type = chunks
        .iter()
        .find(|c| c.symbol_name.as_deref() == Some("privateType"));
    assert!(private_type.is_some(), "privateType not found");
    let private_type = private_type.unwrap();
    assert!(
        private_type.metadata.is_some(),
        "privateType should have metadata"
    );
    let metadata = private_type.metadata.as_ref().unwrap();
    assert_eq!(
        metadata.get("visibility"),
        Some(&serde_json::json!("unexported")),
        "privateType should be unexported"
    );
}

#[test]
fn test_go_exported_vs_unexported_constants() {
    let source = r#"
package main

// MaxRetries is exported
const MaxRetries = 3

// defaultTimeout is unexported
const defaultTimeout = 30
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find the exported constant
    let max_retries = chunks
        .iter()
        .find(|c| c.kind == "constant" && c.symbol_name.as_deref() == Some("MaxRetries"));
    assert!(max_retries.is_some(), "MaxRetries not found");
    let max_retries = max_retries.unwrap();
    assert!(
        max_retries.metadata.is_some(),
        "MaxRetries should have metadata"
    );
    let metadata = max_retries.metadata.as_ref().unwrap();
    assert_eq!(
        metadata.get("visibility"),
        Some(&serde_json::json!("exported")),
        "MaxRetries should be exported"
    );

    // Find the unexported constant
    let default_timeout = chunks
        .iter()
        .find(|c| c.kind == "constant" && c.symbol_name.as_deref() == Some("defaultTimeout"));
    assert!(default_timeout.is_some(), "defaultTimeout not found");
    let default_timeout = default_timeout.unwrap();
    assert!(
        default_timeout.metadata.is_some(),
        "defaultTimeout should have metadata"
    );
    let metadata = default_timeout.metadata.as_ref().unwrap();
    assert_eq!(
        metadata.get("visibility"),
        Some(&serde_json::json!("unexported")),
        "defaultTimeout should be unexported"
    );
}

#[test]
fn test_go_pointer_receiver() {
    let source = r#"
package main

type User struct {
    Name string
}

// SetName uses a pointer receiver
func (u *User) SetName(name string) {
    u.Name = name
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find the method
    let method = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_deref() == Some("SetName"));
    assert!(method.is_some(), "SetName method not found");
    let method = method.unwrap();
    assert!(method.metadata.is_some(), "SetName should have metadata");
    let metadata = method.metadata.as_ref().unwrap();

    assert_eq!(
        metadata.get("receiver_type"),
        Some(&serde_json::json!("pointer")),
        "SetName should have pointer receiver"
    );
    assert_eq!(
        metadata.get("receiver_type_name"),
        Some(&serde_json::json!("User")),
        "Receiver type name should be User"
    );
}

#[test]
fn test_go_value_receiver() {
    let source = r#"
package main

type User struct {
    Name string
}

// GetName uses a value receiver
func (u User) GetName() string {
    return u.Name
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find the method
    let method = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_deref() == Some("GetName"));
    assert!(method.is_some(), "GetName method not found");
    let method = method.unwrap();
    assert!(method.metadata.is_some(), "GetName should have metadata");
    let metadata = method.metadata.as_ref().unwrap();

    assert_eq!(
        metadata.get("receiver_type"),
        Some(&serde_json::json!("value")),
        "GetName should have value receiver"
    );
    assert_eq!(
        metadata.get("receiver_type_name"),
        Some(&serde_json::json!("User")),
        "Receiver type name should be User"
    );
}

#[test]
fn test_go_embedded_struct_fields() {
    let source = r#"
package main

type Base struct {
    ID int
}

type Extended struct {
    Name string
}

// Derived embeds both Base and Extended
type Derived struct {
    Base
    Extended
    Value string
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find the Derived struct
    let derived = chunks
        .iter()
        .find(|c| c.kind == "struct" && c.symbol_name.as_deref() == Some("Derived"));
    assert!(derived.is_some(), "Derived struct not found");
    let derived = derived.unwrap();
    assert!(derived.metadata.is_some(), "Derived should have metadata");
    let metadata = derived.metadata.as_ref().unwrap();

    // Check for embedded types
    let embedded_types = metadata.get("embedded_types");
    assert!(
        embedded_types.is_some(),
        "Derived should have embedded_types"
    );

    let embedded_array = embedded_types.unwrap().as_array();
    assert!(
        embedded_array.is_some(),
        "embedded_types should be an array"
    );
    let embedded_array = embedded_array.unwrap();

    assert_eq!(
        embedded_array.len(),
        2,
        "Derived should have 2 embedded types"
    );
    assert!(
        embedded_array.contains(&serde_json::json!("Base")),
        "Should contain Base"
    );
    assert!(
        embedded_array.contains(&serde_json::json!("Extended")),
        "Should contain Extended"
    );
}

#[test]
fn test_go_embedded_pointer_struct() {
    let source = r#"
package main

type Logger struct {
    Level string
}

// Service embeds *Logger (pointer)
type Service struct {
    *Logger
    Name string
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find the Service struct
    let service = chunks
        .iter()
        .find(|c| c.kind == "struct" && c.symbol_name.as_deref() == Some("Service"));
    assert!(service.is_some(), "Service struct not found");
    let service = service.unwrap();
    assert!(service.metadata.is_some(), "Service should have metadata");
    let metadata = service.metadata.as_ref().unwrap();

    // Check for embedded types
    let embedded_types = metadata.get("embedded_types");
    assert!(
        embedded_types.is_some(),
        "Service should have embedded_types"
    );

    let embedded_array = embedded_types.unwrap().as_array();
    assert!(
        embedded_array.is_some(),
        "embedded_types should be an array"
    );
    let embedded_array = embedded_array.unwrap();

    assert_eq!(
        embedded_array.len(),
        1,
        "Service should have 1 embedded type"
    );
    // Note: The pointer * should be stripped from the type name
    assert!(
        embedded_array.contains(&serde_json::json!("Logger")),
        "Should contain Logger"
    );
}

#[test]
fn test_go_interface_method_signatures() {
    let source = r#"
package main

// Reader interface with multiple methods
type Reader interface {
    Read(p []byte) (n int, err error)
    Close() error
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find the Reader interface
    let reader = chunks
        .iter()
        .find(|c| c.kind == "interface" && c.symbol_name.as_deref() == Some("Reader"));
    assert!(reader.is_some(), "Reader interface not found");
    let reader = reader.unwrap();
    assert!(reader.metadata.is_some(), "Reader should have metadata");
    let metadata = reader.metadata.as_ref().unwrap();

    // Check for interface methods
    let interface_methods = metadata.get("interface_methods");
    assert!(
        interface_methods.is_some(),
        "Reader should have interface_methods"
    );

    let methods_array = interface_methods.unwrap().as_array();
    assert!(
        methods_array.is_some(),
        "interface_methods should be an array"
    );
    let methods_array = methods_array.unwrap();

    assert_eq!(methods_array.len(), 2, "Reader should have 2 methods");
    assert!(
        methods_array
            .iter()
            .any(|m| m.as_str().unwrap().contains("Read")),
        "Should contain Read method"
    );
    assert!(
        methods_array
            .iter()
            .any(|m| m.as_str().unwrap().contains("Close")),
        "Should contain Close method"
    );
}

#[test]
fn test_go_empty_interface() {
    let source = r#"
package main

// Any is an empty interface (accepts any type)
type Any interface{}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find the Any interface
    let any = chunks
        .iter()
        .find(|c| c.kind == "interface" && c.symbol_name.as_deref() == Some("Any"));
    assert!(any.is_some(), "Any interface not found");
    let any = any.unwrap();

    // Empty interfaces may or may not have metadata depending on whether there are methods
    // In this case, there should be metadata with visibility but no interface_methods
    if let Some(metadata) = &any.metadata {
        let interface_methods = metadata.get("interface_methods");
        // If present, should be empty array
        if let Some(methods) = interface_methods {
            let methods_array = methods.as_array().unwrap();
            assert_eq!(
                methods_array.len(),
                0,
                "Empty interface should have no methods"
            );
        }
    }
}

#[test]
fn test_go_method_visibility() {
    let source = r#"
package main

type Service struct {
    data string
}

// Start is an exported method
func (s *Service) Start() {
}

// stop is an unexported method
func (s *Service) stop() {
}
"#;

    let chunks = parser::extract_chunks(source, "go");

    // Find the exported method
    let start_method = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_deref() == Some("Start"));
    assert!(start_method.is_some(), "Start method not found");
    let start_method = start_method.unwrap();
    assert!(
        start_method.metadata.is_some(),
        "Start should have metadata"
    );
    let metadata = start_method.metadata.as_ref().unwrap();
    assert_eq!(
        metadata.get("visibility"),
        Some(&serde_json::json!("exported")),
        "Start should be exported"
    );

    // Find the unexported method
    let stop_method = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_deref() == Some("stop"));
    assert!(stop_method.is_some(), "stop method not found");
    let stop_method = stop_method.unwrap();
    assert!(stop_method.metadata.is_some(), "stop should have metadata");
    let metadata = stop_method.metadata.as_ref().unwrap();
    assert_eq!(
        metadata.get("visibility"),
        Some(&serde_json::json!("unexported")),
        "stop should be unexported"
    );
}
