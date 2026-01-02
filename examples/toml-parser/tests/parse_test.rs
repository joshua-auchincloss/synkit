//! Parsing tests for the TOML parser

use toml_parser::*;

fn parse_doc(input: &str) -> Document {
    let mut stream = TokenStream::lex(input).unwrap();
    stream.parse::<Document>().unwrap().value
}

#[test]
fn test_simple_key_value() {
    let doc = parse_doc(r#"key = "value""#);
    assert_eq!(doc.items.len(), 1);

    match &doc.items[0] {
        DocumentItem::KeyValue(kv) => {
            assert!(matches!(&kv.value.key.value, Key::Bare(tok) if tok.0 == "key"));
            assert!(matches!(&kv.value.value.value, Value::String(tok) if tok.0 == "value"));
        }
        _ => panic!("expected key-value"),
    }
}

#[test]
fn test_integer_value() {
    let doc = parse_doc("count = 42");
    match &doc.items[0] {
        DocumentItem::KeyValue(kv) => {
            assert!(matches!(&kv.value.value.value, Value::Integer(tok) if tok.0 == 42));
        }
        _ => panic!("expected key-value"),
    }
}

#[test]
fn test_negative_integer() {
    let doc = parse_doc("offset = -10");
    match &doc.items[0] {
        DocumentItem::KeyValue(kv) => {
            assert!(matches!(&kv.value.value.value, Value::Integer(tok) if tok.0 == -10));
        }
        _ => panic!("expected key-value"),
    }
}

#[test]
fn test_boolean_values() {
    let doc = parse_doc(
        r#"
enabled = true
disabled = false
"#,
    );

    let kvs: Vec<_> = doc
        .items
        .iter()
        .filter_map(|item| match item {
            DocumentItem::KeyValue(kv) => Some(&kv.value),
            _ => None,
        })
        .collect();

    assert!(matches!(&kvs[0].value.value, Value::True(_)));
    assert!(matches!(&kvs[1].value.value, Value::False(_)));
}

#[test]
fn test_quoted_key() {
    let doc = parse_doc(r#""foo.bar" = "value""#);
    match &doc.items[0] {
        DocumentItem::KeyValue(kv) => {
            assert!(matches!(&kv.value.key.value, Key::Quoted(tok) if tok.0 == "foo.bar"));
        }
        _ => panic!("expected key-value"),
    }
}

#[test]
fn test_dotted_key() {
    let doc = parse_doc(r#"server.host = "localhost""#);
    match &doc.items[0] {
        DocumentItem::KeyValue(kv) => {
            assert!(matches!(&kv.value.key.value, Key::Dotted(_)));
        }
        _ => panic!("expected key-value"),
    }
}

#[test]
fn test_simple_array() {
    let doc = parse_doc("numbers = [1, 2, 3]");
    match &doc.items[0] {
        DocumentItem::KeyValue(kv) => {
            if let Value::Array(arr) = &kv.value.value.value {
                assert_eq!(arr.items.len(), 3);
            } else {
                panic!("expected array");
            }
        }
        _ => panic!("expected key-value"),
    }
}

#[test]
fn test_mixed_array() {
    let doc = parse_doc(r#"mixed = ["a", "b", "c"]"#);
    match &doc.items[0] {
        DocumentItem::KeyValue(kv) => {
            if let Value::Array(arr) = &kv.value.value.value {
                assert_eq!(arr.items.len(), 3);
            } else {
                panic!("expected array");
            }
        }
        _ => panic!("expected key-value"),
    }
}

#[test]
fn test_inline_table() {
    let doc = parse_doc(r#"point = { x = 1, y = 2 }"#);
    match &doc.items[0] {
        DocumentItem::KeyValue(kv) => {
            if let Value::InlineTable(tbl) = &kv.value.value.value {
                assert_eq!(tbl.items.len(), 2);
            } else {
                panic!("expected inline table");
            }
        }
        _ => panic!("expected key-value"),
    }
}

#[test]
fn test_table_section() {
    let doc = parse_doc(
        r#"
[package]
name = "test"
version = "1.0"
"#,
    );

    let tables: Vec<_> = doc
        .items
        .iter()
        .filter_map(|item| match item {
            DocumentItem::Table(t) => Some(&t.value),
            _ => None,
        })
        .collect();

    assert_eq!(tables.len(), 1);
    assert!(matches!(&tables[0].name.value, Key::Bare(tok) if tok.0 == "package"));
}

#[test]
fn test_nested_table() {
    let doc = parse_doc(
        r#"
[server.http]
port = 8080
"#,
    );

    let tables: Vec<_> = doc
        .items
        .iter()
        .filter_map(|item| match item {
            DocumentItem::Table(t) => Some(&t.value),
            _ => None,
        })
        .collect();

    assert_eq!(tables.len(), 1);
    assert!(matches!(&tables[0].name.value, Key::Dotted(_)));
}

#[test]
fn test_multiple_tables() {
    let doc = parse_doc(
        r#"
[first]
a = 1

[second]
b = 2
"#,
    );

    let tables: Vec<_> = doc
        .items
        .iter()
        .filter_map(|item| match item {
            DocumentItem::Table(t) => Some(&t.value),
            _ => None,
        })
        .collect();

    assert_eq!(tables.len(), 2);
}

#[test]
fn test_complex_document() {
    let input = r#"
# This is a TOML document

title = "Example"

[owner]
name = "John Doe"
enabled = true

[database]
server = "192.168.1.1"
ports = [8001, 8002, 8003]
connection_max = 5000

[servers.alpha]
ip = "10.0.0.1"

[servers.beta]
ip = "10.0.0.2"
"#;

    let doc = parse_doc(input);

    // Should have parsed without error
    let tables: Vec<_> = doc
        .items
        .iter()
        .filter_map(|item| match item {
            DocumentItem::Table(t) => Some(&t.value),
            _ => None,
        })
        .collect();

    assert_eq!(tables.len(), 4); // owner, database, servers.alpha, servers.beta
}

#[test]
fn test_empty_document() {
    let doc = parse_doc("");
    assert!(doc.items.is_empty());
}

#[test]
fn test_comments_only() {
    let doc = parse_doc(
        r#"
# Comment 1
# Comment 2
"#,
    );

    // Document should have trivia items
    let trivia_count = doc
        .items
        .iter()
        .filter(|item| matches!(item, DocumentItem::Trivia(_)))
        .count();
    assert!(trivia_count > 0);
}

#[test]
fn test_trailing_comma_in_array() {
    let doc = parse_doc("arr = [1, 2, 3,]");
    match &doc.items[0] {
        DocumentItem::KeyValue(kv) => {
            if let Value::Array(arr) = &kv.value.value.value {
                assert_eq!(arr.items.len(), 3);
            } else {
                panic!("expected array");
            }
        }
        _ => panic!("expected key-value"),
    }
}

#[test]
fn test_multiline_array() {
    let doc = parse_doc(
        r#"arr = [
    1,
    2,
    3
]"#,
    );
    match &doc.items[0] {
        DocumentItem::KeyValue(kv) => {
            if let Value::Array(arr) = &kv.value.value.value {
                assert_eq!(arr.items.len(), 3);
            } else {
                panic!("expected array");
            }
        }
        _ => panic!("expected key-value"),
    }
}
