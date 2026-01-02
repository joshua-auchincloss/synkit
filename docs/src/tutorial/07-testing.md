# Testing

Verify parsing correctness and round-trip fidelity.

## Parse Tests

Test that parsing produces expected AST:

```rust,ignore
#[test]
fn test_simple_key_value() {
    let mut stream = TokenStream::lex("key = \"value\"").unwrap();
    let kv: Spanned<KeyValue> = stream.parse().unwrap();

    match &kv.value.key.value {
        Key::Bare(tok) => assert_eq!(&**tok, "key"),
        _ => panic!("expected bare key"),
    }

    match &kv.value.value.value {
        Value::String(tok) => assert_eq!(&**tok, "value"),
        _ => panic!("expected string value"),
    }
}
```

## Round-trip Tests

Verify parse → print produces equivalent output:

```rust,ignore
fn roundtrip(input: &str) -> String {
    let mut stream = TokenStream::lex(input).unwrap();
    let doc: Spanned<Document> = stream.parse().unwrap();
    doc.value.to_string_formatted()
}

#[test]
fn test_roundtrip_simple() {
    let input = "key = \"value\"";
    assert_eq!(roundtrip(input), input);
}

#[test]
fn test_roundtrip_table() {
    let input = "[section]\nkey = 42";
    assert_eq!(roundtrip(input), input);
}
```

## Snapshot Testing with insta

For complex outputs, use snapshot testing:

```rust,ignore
use insta::assert_yaml_snapshot;

#[test]
fn snapshot_complex_document() {
    let input = r#"
# Header comment
title = "Example"

[server]
host = "localhost"
port = 8080
"#.trim();

    let mut stream = TokenStream::lex(input).unwrap();
    let doc: Spanned<Document> = stream.parse().unwrap();
    let output = doc.value.to_string_formatted();

    assert_yaml_snapshot!(output);
}
```

Run `cargo insta test` to review and accept snapshots.

## Error Tests

Verify error handling:

```rust,ignore
#[test]
fn test_error_missing_value() {
    let mut stream = TokenStream::lex("key =").unwrap();
    let result: Result<Spanned<KeyValue>, _> = stream.parse();
    assert!(result.is_err());
}

#[test]
fn test_error_invalid_token() {
    let result = TokenStream::lex("@invalid");
    assert!(result.is_err());
}
```

## Visitor Tests

```rust,ignore
#[test]
fn test_key_collector() {
    let input = "a = 1\nb = 2\n[section]\nc = 3";
    let mut stream = TokenStream::lex(input).unwrap();
    let doc: Spanned<Document> = stream.parse().unwrap();

    let mut collector = KeyCollector::new();
    collector.visit_document(&doc.value);

    assert_eq!(collector.keys, vec!["a", "b", "c"]);
}
```

## Test Organization

```
tests/
├── parse_test.rs      # Parse correctness
├── roundtrip_test.rs  # Round-trip fidelity
└── visitor_test.rs    # Visitor behavior
```

## Testing Tips

### Test Edge Cases

```rust,ignore
#[test] fn test_empty_document() { /* ... */ }
#[test] fn test_trailing_comma() { /* ... */ }
#[test] fn test_nested_tables() { /* ... */ }
#[test] fn test_unicode_strings() { /* ... */ }
```

### Property-Based Testing

With proptest:

```rust,ignore
proptest! {
    #[test]
    fn roundtrip_integers(n: i64) {
        let input = format!("x = {}", n);
        let output = roundtrip(&input);
        assert_eq!(input, output);
    }
}
```

### Debug Output

```rust,ignore
#[test]
fn debug_parse() {
    let mut stream = TokenStream::lex("key = [1, 2]").unwrap();
    let doc: Spanned<Document> = stream.parse().unwrap();

    // AST structure
    dbg!(&doc);

    // Formatted output
    println!("{}", doc.value.to_string_formatted());
}
```

## Running Tests

```bash
# All tests
cargo test

# Specific test file
cargo test --test parse_test

# Update snapshots
cargo insta test --accept
```
