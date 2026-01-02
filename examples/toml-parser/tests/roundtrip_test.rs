//! Round-trip tests: parse TOML, print it back, verify output

use toml_parser::*;

fn roundtrip(input: &str) -> String {
    let mut stream = TokenStream::lex(input).unwrap();
    let doc: Spanned<Document> = stream.parse().unwrap();
    doc.value.to_string_formatted()
}

#[test]
fn test_roundtrip_simple_kv() {
    // Note: roundtrip normalizes spacing
    let output = roundtrip(r#"key = "value""#);
    assert!(output.contains("key"));
    assert!(output.contains("="));
    assert!(output.contains("value"));
}

#[test]
fn test_roundtrip_integer() {
    let output = roundtrip("count = 42");
    assert!(output.contains("42"));
}

#[test]
fn test_roundtrip_boolean() {
    let output = roundtrip("enabled = true");
    assert!(output.contains("true"));
}

#[test]
fn test_roundtrip_array() {
    let output = roundtrip("arr = [1, 2, 3]");
    assert!(output.contains("["));
    assert!(output.contains("]"));
    assert!(output.contains("1"));
    assert!(output.contains("2"));
    assert!(output.contains("3"));
}

#[test]
fn test_roundtrip_inline_table() {
    let output = roundtrip("point = { x = 1, y = 2 }");
    assert!(output.contains("{"));
    assert!(output.contains("}"));
    assert!(output.contains("x"));
    assert!(output.contains("y"));
}

#[test]
fn test_roundtrip_table() {
    let input = r#"[package]
name = "test"
"#;
    let output = roundtrip(input);
    assert!(output.contains("[package]"));
    assert!(output.contains("name"));
}

#[test]
fn test_roundtrip_dotted_key() {
    let output = roundtrip("server.host = \"localhost\"");
    assert!(output.contains("server.host"));
}

#[test]
fn test_roundtrip_quoted_key() {
    let output = roundtrip(r#""special.key" = "value""#);
    assert!(output.contains("\"special.key\""));
}

#[test]
fn test_roundtrip_nested_table() {
    let input = r#"[server.http]
port = 8080
"#;
    let output = roundtrip(input);
    assert!(output.contains("[server.http]"));
    assert!(output.contains("port"));
}

#[test]
fn test_roundtrip_complex() {
    let input = r#"title = "My App"

[database]
server = "localhost"
ports = [8001, 8002]
enabled = true
"#;

    let output = roundtrip(input);

    // Verify key components are preserved
    assert!(output.contains("title"));
    assert!(output.contains("My App"));
    assert!(output.contains("[database]"));
    assert!(output.contains("server"));
    assert!(output.contains("localhost"));
    assert!(output.contains("ports"));
    assert!(output.contains("8001"));
    assert!(output.contains("8002"));
    assert!(output.contains("enabled"));
    assert!(output.contains("true"));
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    fn parse_and_format(input: &str) -> String {
        let mut stream = TokenStream::lex(input).unwrap();
        let doc: Spanned<Document> = stream.parse().unwrap();
        doc.value.to_string_formatted()
    }

    #[test]
    fn snapshot_basic_toml() {
        let input = r#"name = "test"
version = "1.0.0"
"#;
        let output = parse_and_format(input);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_with_table() {
        let input = r#"[package]
name = "my-package"
version = "0.1.0"
"#;
        let output = parse_and_format(input);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_with_arrays() {
        let input = r#"numbers = [1, 2, 3]
strings = ["a", "b", "c"]
"#;
        let output = parse_and_format(input);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_inline_table() {
        let input = r#"point = { x = 10, y = 20 }
"#;
        let output = parse_and_format(input);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_complex_document() {
        let input = r#"title = "Example"

[owner]
name = "Alice"
active = true

[database]
host = "localhost"
port = 5432
"#;
        let output = parse_and_format(input);
        insta::assert_snapshot!(output);
    }
}
