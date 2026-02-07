use crewchief_maproom::indexer::parser;

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
