use maproom::indexer::parser;

#[test]
fn test_csharp_class_with_methods() {
    let source = r#"
namespace MyNamespace
{
    /// <summary>
    /// A sample class
    /// </summary>
    public class MyClass : BaseClass, IInterface
    {
        /// <summary>
        /// A method
        /// </summary>
        public int Calculate(int x, string y)
        {
            return x;
        }

        private void Helper()
        {
        }
    }
}
"#;

    let chunks = parser::extract_chunks(source, "cs");

    // Find namespace chunk
    let namespace = chunks.iter().find(|c| c.kind == "namespace").unwrap();
    assert_eq!(namespace.symbol_name.as_ref().unwrap(), "MyNamespace");

    // Find class chunk
    let class = chunks.iter().find(|c| c.kind == "class").unwrap();
    assert_eq!(class.symbol_name.as_ref().unwrap(), "MyClass");
    assert!(class.signature.as_ref().unwrap().contains("BaseClass"));
    assert!(class.signature.as_ref().unwrap().contains("IInterface"));
    assert!(class.docstring.as_ref().unwrap().contains("A sample class"));

    // Check class metadata
    let metadata = class.metadata.as_ref().unwrap();
    assert_eq!(metadata["visibility"], "public");

    // Find method chunks
    let calculate = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_ref().unwrap() == "Calculate")
        .unwrap();
    assert!(calculate
        .signature
        .as_ref()
        .unwrap()
        .contains("int x, string y"));
    assert!(calculate.signature.as_ref().unwrap().contains("int"));
    assert!(calculate.docstring.is_some());

    let helper = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_ref().unwrap() == "Helper")
        .unwrap();
    let helper_metadata = helper.metadata.as_ref().unwrap();
    assert_eq!(helper_metadata["visibility"], "private");
}

#[test]
fn test_csharp_interface_declaration() {
    let source = r#"
public interface IRepository<T> where T : class
{
    T GetById(int id);
    void Save(T entity);
    IEnumerable<T> GetAll();
}
"#;

    let chunks = parser::extract_chunks(source, "cs");

    let interface = chunks.iter().find(|c| c.kind == "interface").unwrap();
    assert_eq!(interface.symbol_name.as_ref().unwrap(), "IRepository");
    assert!(interface.signature.as_ref().unwrap().contains("<T>"));

    // Method signatures should be extracted
    let get_by_id = chunks
        .iter()
        .find(|c| c.symbol_name.as_ref().unwrap() == "GetById")
        .unwrap();
    assert_eq!(get_by_id.kind, "method");
}

#[test]
fn test_csharp_struct_declaration() {
    let source = r#"
public struct Point
{
    public int X { get; set; }
    public int Y { get; set; }

    public Point(int x, int y)
    {
        X = x;
        Y = y;
    }
}
"#;

    let chunks = parser::extract_chunks(source, "cs");

    let struct_chunk = chunks.iter().find(|c| c.kind == "struct").unwrap();
    assert_eq!(struct_chunk.symbol_name.as_ref().unwrap(), "Point");

    let constructor = chunks.iter().find(|c| c.kind == "constructor").unwrap();
    assert_eq!(constructor.symbol_name.as_ref().unwrap(), "Point");

    let properties: Vec<_> = chunks.iter().filter(|c| c.kind == "property").collect();
    assert_eq!(properties.len(), 2);
}

#[test]
fn test_csharp_enum_declaration() {
    let source = r#"
public enum Status : byte
{
    Active,
    Inactive,
    Pending
}
"#;

    let chunks = parser::extract_chunks(source, "cs");

    let enum_chunk = chunks.iter().find(|c| c.kind == "enum").unwrap();
    assert_eq!(enum_chunk.symbol_name.as_ref().unwrap(), "Status");
    assert!(enum_chunk.signature.as_ref().unwrap().contains("byte"));
}

#[test]
fn test_csharp_delegate_declaration() {
    let source = r#"
public delegate void EventHandler<T>(object sender, T args) where T : EventArgs;
"#;

    let chunks = parser::extract_chunks(source, "cs");

    let delegate = chunks.iter().find(|c| c.kind == "delegate").unwrap();
    assert_eq!(delegate.symbol_name.as_ref().unwrap(), "EventHandler");
    assert!(delegate.signature.as_ref().unwrap().contains("void"));
    assert!(delegate
        .signature
        .as_ref()
        .unwrap()
        .contains("object sender"));
}

#[test]
fn test_csharp_properties_and_events() {
    let source = r#"
public class MyClass
{
    public string Name { get; set; }
    public int Count { get; }
    public bool IsActive => true;

    public event EventHandler OnClick;
    public event EventHandler<CustomArgs> OnCustom;
}
"#;

    let chunks = parser::extract_chunks(source, "cs");

    let properties: Vec<_> = chunks.iter().filter(|c| c.kind == "property").collect();
    assert!(properties.len() >= 3);

    let name_prop = properties
        .iter()
        .find(|c| c.symbol_name.as_ref().unwrap() == "Name")
        .unwrap();
    assert!(name_prop.signature.as_ref().unwrap().contains("get"));
    assert!(name_prop.signature.as_ref().unwrap().contains("set"));

    let active_prop = properties
        .iter()
        .find(|c| c.symbol_name.as_ref().unwrap() == "IsActive")
        .unwrap();
    assert!(active_prop.signature.as_ref().unwrap().contains("=>"));

    let events: Vec<_> = chunks.iter().filter(|c| c.kind == "event").collect();
    assert_eq!(events.len(), 2);
}

#[test]
fn test_csharp_namespace_and_using() {
    let source = r#"
using System;
using System.Collections.Generic;
using static System.Math;

namespace MyNamespace
{
    public class MyClass { }
}
"#;

    let chunks = parser::extract_chunks(source, "cs");

    let namespace = chunks.iter().find(|c| c.kind == "namespace").unwrap();
    assert_eq!(namespace.symbol_name.as_ref().unwrap(), "MyNamespace");

    let imports = chunks.iter().find(|c| c.kind == "imports").unwrap();
    assert_eq!(imports.symbol_name.as_ref().unwrap(), "__imports__");

    let metadata = imports.metadata.as_ref().unwrap();
    assert!(metadata.is_array());
    assert!(metadata.as_array().unwrap().len() >= 3);
}

#[test]
fn test_csharp_access_modifiers() {
    let source = r#"
public class PublicClass { }
internal class InternalClass { }
class DefaultClass { }  // Should be internal for top-level, private for nested

public class Container
{
    public void PublicMethod() { }
    private void PrivateMethod() { }
    protected void ProtectedMethod() { }
    internal void InternalMethod() { }
    protected internal void ProtectedInternalMethod() { }
    private protected void PrivateProtectedMethod() { }
    void DefaultMethod() { }  // Should be private
}
"#;

    let chunks = parser::extract_chunks(source, "cs");

    let public_class = chunks
        .iter()
        .find(|c| c.kind == "class" && c.symbol_name.as_ref().unwrap() == "PublicClass")
        .unwrap();
    assert_eq!(
        public_class.metadata.as_ref().unwrap()["visibility"],
        "public"
    );

    let public_method = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_ref().unwrap() == "PublicMethod")
        .unwrap();
    assert_eq!(
        public_method.metadata.as_ref().unwrap()["visibility"],
        "public"
    );

    let private_method = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_ref().unwrap() == "PrivateMethod")
        .unwrap();
    assert_eq!(
        private_method.metadata.as_ref().unwrap()["visibility"],
        "private"
    );

    let protected_internal_method = chunks
        .iter()
        .find(|c| {
            c.kind == "method" && c.symbol_name.as_ref().unwrap() == "ProtectedInternalMethod"
        })
        .unwrap();
    assert_eq!(
        protected_internal_method.metadata.as_ref().unwrap()["visibility"],
        "protected internal"
    );
}

#[test]
fn test_csharp_generics() {
    let source = r#"
public class GenericClass<T, U>
{
    public T GetValue<V>(V input)
    {
        return default(T);
    }
}

public interface IGeneric<T> where T : class
{
    T Get();
}
"#;

    let chunks = parser::extract_chunks(source, "cs");

    let generic_class = chunks
        .iter()
        .find(|c| c.kind == "class" && c.symbol_name.as_ref().unwrap() == "GenericClass")
        .unwrap();
    assert!(generic_class.signature.as_ref().unwrap().contains("<T, U>"));

    let get_value = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_ref().unwrap() == "GetValue")
        .unwrap();
    assert!(get_value.signature.as_ref().unwrap().contains("<V>"));

    let generic_interface = chunks
        .iter()
        .find(|c| c.kind == "interface" && c.symbol_name.as_ref().unwrap() == "IGeneric")
        .unwrap();
    assert!(generic_interface
        .signature
        .as_ref()
        .unwrap()
        .contains("<T>"));
}

#[test]
fn test_csharp_generic_constraints() {
    let source = r#"
public class MyClass
{
    public T Process<T>(T input) where T : IComparable, ICloneable
    {
        return input;
    }

    public void MultiConstraint<T, U>(T t, U u)
        where T : class, new()
        where U : struct
    {
    }
}
"#;

    let chunks = parser::extract_chunks(source, "cs");

    let process = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_ref().unwrap() == "Process")
        .unwrap();
    let sig = process.signature.as_ref().unwrap();
    assert!(sig.contains("where T"));
    assert!(sig.contains("IComparable"));

    let multi = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_ref().unwrap() == "MultiConstraint")
        .unwrap();
    let multi_sig = multi.signature.as_ref().unwrap();
    assert!(multi_sig.contains("where T"));
    assert!(multi_sig.contains("where U"));
}

#[test]
fn test_csharp_doc_comments() {
    let source = r#"
public class MyClass
{
    /// <summary>
    /// This is a multi-line doc comment
    /// with XML tags
    /// </summary>
    /// <param name="x">The X parameter</param>
    /// <returns>The result</returns>
    public int Calculate(int x)
    {
        return x;
    }

    // Regular comment - should NOT be captured
    public void NotDocumented() { }

    ///
    /// Doc comment with blank line
    ///
    public void BlankLineDoc() { }
}
"#;

    let chunks = parser::extract_chunks(source, "cs");

    let calculate = chunks
        .iter()
        .find(|c| c.symbol_name.as_ref().unwrap() == "Calculate")
        .unwrap();
    let docstring = calculate.docstring.as_ref().unwrap();
    assert!(docstring.contains("<summary>"));
    assert!(docstring.contains("multi-line"));
    assert!(docstring.contains("<param"));
    assert!(docstring.contains("<returns>"));

    // Regular comments "//" should NOT be captured as docstrings
    // The parser's backward walk stops at non-/// comments
    let not_documented = chunks
        .iter()
        .find(|c| c.symbol_name.as_ref().unwrap() == "NotDocumented");
    // Method may or may not be extracted, but if it is, docstring should be None
    if let Some(nd) = not_documented {
        assert!(nd.docstring.is_none());
    }

    let blank_line_doc = chunks
        .iter()
        .find(|c| c.symbol_name.as_ref().unwrap() == "BlankLineDoc")
        .unwrap();
    assert!(blank_line_doc.docstring.is_some());
}

#[test]
fn test_csharp_nested_types() {
    let source = r#"
public class OuterClass
{
    public void OuterMethod() { }

    public class InnerClass
    {
        public void InnerMethod() { }
    }
}
"#;

    let chunks = parser::extract_chunks(source, "cs");

    let outer = chunks
        .iter()
        .find(|c| c.kind == "class" && c.symbol_name.as_ref().unwrap() == "OuterClass")
        .unwrap();
    assert_eq!(outer.kind, "class");

    let inner = chunks
        .iter()
        .find(|c| c.kind == "class" && c.symbol_name.as_ref().unwrap() == "InnerClass")
        .unwrap();
    assert_eq!(inner.kind, "class");

    let outer_method = chunks
        .iter()
        .find(|c| c.symbol_name.as_ref().unwrap() == "OuterMethod")
        .unwrap();
    assert_eq!(outer_method.kind, "method");

    let inner_method = chunks
        .iter()
        .find(|c| c.symbol_name.as_ref().unwrap() == "InnerMethod")
        .unwrap();
    assert_eq!(inner_method.kind, "method");
}

#[test]
fn test_csharp_file_scoped_namespace() {
    let source = r#"
namespace MyNamespace;

public class MyClass { }
public interface IMyInterface { }
"#;

    let chunks = parser::extract_chunks(source, "cs");

    let namespace = chunks.iter().find(|c| c.kind == "namespace").unwrap();
    assert_eq!(namespace.symbol_name.as_ref().unwrap(), "MyNamespace");

    let class = chunks.iter().find(|c| c.kind == "class").unwrap();
    assert_eq!(class.symbol_name.as_ref().unwrap(), "MyClass");

    let interface = chunks.iter().find(|c| c.kind == "interface").unwrap();
    assert_eq!(interface.symbol_name.as_ref().unwrap(), "IMyInterface");
}

#[test]
fn test_csharp_expression_bodied_members() {
    let source = r#"
public class MyClass
{
    public int Calculate(int x) => x * 2;

    public string Name => "DefaultName";

    public int Count
    {
        get => _count;
        set => _count = value;
    }

    private int _count;
}
"#;

    let chunks = parser::extract_chunks(source, "cs");

    let calculate = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_ref().unwrap() == "Calculate")
        .unwrap();
    assert!(calculate.signature.is_some());

    let name = chunks
        .iter()
        .find(|c| c.kind == "property" && c.symbol_name.as_ref().unwrap() == "Name")
        .unwrap();
    assert!(name.signature.as_ref().unwrap().contains("=>"));

    let count = chunks
        .iter()
        .find(|c| c.kind == "property" && c.symbol_name.as_ref().unwrap() == "Count")
        .unwrap();
    assert!(count.signature.is_some());
}

#[test]
fn test_csharp_empty_file() {
    let source = "";
    let chunks = parser::extract_chunks(source, "cs");
    assert!(chunks.is_empty());
}

#[test]
fn test_csharp_syntax_error() {
    let source = r#"
public class Broken {
    public void Method(
    // Missing closing paren and brace
"#;

    // Should not panic
    let _chunks = parser::extract_chunks(source, "cs");
    // May return partial results or empty - either is acceptable
    // Key assertion: no panic occurred
}

#[test]
fn test_csharp_unicode_identifiers() {
    // Test Unicode identifier support with Cyrillic characters
    // Note: tree-sitter-c-sharp 0.21.3 has partial Unicode support
    // - Cyrillic (Russian): Works correctly
    // - Chinese/CJK: Grammar limitation (fails to parse)
    // This validates UTF-8 handling and confirms partial Unicode support
    let source = r#"
namespace MyApp {
    /// <summary>Russian capital city class.</summary>
    public class Москва {
        public void Привет(string имя) { }
    }
}
"#;

    let chunks = parser::extract_chunks(source, "cs");

    // Verify namespace
    assert!(chunks
        .iter()
        .any(|c| c.kind == "namespace" && c.symbol_name.as_deref() == Some("MyApp")));

    // Verify Cyrillic class name
    let moscow_class = chunks
        .iter()
        .find(|c| c.kind == "class" && c.symbol_name.as_deref() == Some("Москва"));
    assert!(moscow_class.is_some(), "Cyrillic class name not found");
    let moscow_class = moscow_class.unwrap();
    assert_eq!(moscow_class.symbol_name.as_deref(), Some("Москва"));
    assert!(moscow_class.docstring.is_some());
    assert!(moscow_class
        .docstring
        .as_ref()
        .unwrap()
        .contains("Russian capital"));

    // Verify Cyrillic method name
    let hello_method = chunks
        .iter()
        .find(|c| c.kind == "method" && c.symbol_name.as_deref() == Some("Привет"));
    assert!(hello_method.is_some(), "Cyrillic method name not found");

    // Verify Cyrillic parameter name in signature
    let hello_method = hello_method.unwrap();
    assert!(
        hello_method.signature.as_ref().unwrap().contains("имя"),
        "Cyrillic parameter not in signature"
    );
}

#[test]
fn test_csharp_deep_nesting() {
    // Generate 50 levels of nested classes (within limit)
    let mut source = String::from("namespace Test {\n");
    for i in 0..50 {
        source.push_str(&format!("    public class Level{} {{\n", i));
    }
    // Close all classes
    for _ in 0..50 {
        source.push_str("    }\n");
    }
    source.push_str("}\n");

    let chunks = parser::extract_chunks(&source, "cs");

    // Verify namespace extracted
    assert!(chunks
        .iter()
        .any(|c| c.kind == "namespace" && c.symbol_name.as_deref() == Some("Test")));

    // Verify all 50 class levels extracted
    for i in 0..50 {
        let class_name = format!("Level{}", i);
        assert!(
            chunks
                .iter()
                .any(|c| c.kind == "class" && c.symbol_name.as_deref() == Some(&class_name)),
            "Class {} not found (depth limit may be too low)",
            class_name
        );
    }

    // Total: 1 namespace + 50 classes
    assert_eq!(chunks.len(), 51);
}

#[test]
fn test_csharp_extreme_nesting() {
    // Generate 105 levels of nested classes (exceeds limit of 100)
    let mut source = String::from("namespace Test {\n");
    for i in 0..105 {
        source.push_str(&format!("    public class Level{} {{\n", i));
    }
    for _ in 0..105 {
        source.push_str("    }\n");
    }
    source.push_str("}\n");

    let chunks = parser::extract_chunks(&source, "cs");

    // Verify namespace extracted
    assert!(chunks
        .iter()
        .any(|c| c.kind == "namespace" && c.symbol_name.as_deref() == Some("Test")));

    // Verify first 99 levels extracted (depth 1-99, namespace is depth 0)
    let extracted_classes: Vec<_> = chunks.iter().filter(|c| c.kind == "class").collect();

    // Should extract up to depth limit, then stop
    // Exact count depends on when limit is hit (may be ~100 classes)
    assert!(
        extracted_classes.len() < 105,
        "Expected truncation at depth limit, but got {} classes",
        extracted_classes.len()
    );
    assert!(
        extracted_classes.len() >= 90,
        "Expected at least 90 classes before limit, got {}",
        extracted_classes.len()
    );
}
