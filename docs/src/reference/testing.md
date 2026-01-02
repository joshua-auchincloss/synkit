# Testing Generated Code

This guide covers testing strategies for parsers built with synkit, from unit tests to fuzz testing.

## Unit Testing

### Token-Level Tests

Test individual token recognition:

```rust
#[test]
fn test_lex_identifier() {
    let stream = TokenStream::lex("foo_bar").unwrap();
    let tok = stream.peek_token().unwrap();

    assert!(matches!(tok.value, Token::Ident(_)));
    if let Token::Ident(s) = &tok.value {
        assert_eq!(s, "foo_bar");
    }
}

#[test]
fn test_lex_rejects_invalid() {
    // Logos returns errors for unrecognized input
    let result = TokenStream::lex("\x00\x01\x02");
    assert!(result.is_err());
}
```

### Span Accuracy Tests

Verify spans point to correct source locations:

```rust
#[test]
fn test_span_accuracy() {
    let source = "let x = 42";
    let mut stream = TokenStream::lex(source).unwrap();

    let kw: Spanned<LetToken> = stream.parse().unwrap();
    assert_eq!(&source[kw.span.start()..kw.span.end()], "let");

    let name: Spanned<IdentToken> = stream.parse().unwrap();
    assert_eq!(&source[name.span.start()..name.span.end()], "x");
}
```

### Parse Tests

Test AST construction:

```rust
#[test]
fn test_parse_key_value() {
    let mut stream = TokenStream::lex("name = \"Alice\"").unwrap();
    let kv: Spanned<KeyValue> = stream.parse().unwrap();

    assert!(matches!(kv.key.value, Key::Bare(_)));
    assert!(matches!(kv.value.value, Value::String(_)));
}

#[test]
fn test_parse_error_recovery() {
    let mut stream = TokenStream::lex("= value").unwrap();
    let result: Result<Spanned<KeyValue>, _> = stream.parse();

    assert!(result.is_err());
    // Verify error message is helpful
    let err = result.unwrap_err();
    assert!(err.to_string().contains("expected"));
}
```

## Round-Trip Testing

Verify parse-then-print produces equivalent output:

```rust
#[test]
fn test_roundtrip() {
    let original = "name = \"value\"\ncount = 42";

    let mut stream = TokenStream::lex(original).unwrap();
    let doc: Document = stream.parse().unwrap();

    let mut printer = Printer::new();
    doc.write(&mut printer);
    let output = printer.into_string();

    // Re-parse and compare AST
    let mut stream2 = TokenStream::lex(&output).unwrap();
    let doc2: Document = stream2.parse().unwrap();

    assert_eq!(format!("{:?}", doc), format!("{:?}", doc2));
}
```

## Snapshot Testing

Use `insta` for golden-file testing:

```rust
use insta::assert_snapshot;

#[test]
fn snapshot_complex_document() {
    let input = include_str!("fixtures/complex.toml");
    let mut stream = TokenStream::lex(input).unwrap();
    let doc: Document = stream.parse().unwrap();

    assert_snapshot!(format!("{:#?}", doc));
}

#[test]
fn snapshot_formatted_output() {
    let input = "messy   =   \"spacing\"";
    let doc: Document = parse(input).unwrap();

    let mut printer = Printer::new();
    doc.write(&mut printer);

    assert_snapshot!(printer.into_string());
}
```

## Parameterized Tests

Use `test-case` for table-driven tests:

```rust
use test_case::test_case;

#[test_case("42", Value::Integer(42); "positive integer")]
#[test_case("-17", Value::Integer(-17); "negative integer")]
#[test_case("true", Value::Bool(true); "boolean true")]
#[test_case("false", Value::Bool(false); "boolean false")]
fn test_parse_value(input: &str, expected: Value) {
    let mut stream = TokenStream::lex(input).unwrap();
    let value: Spanned<Value> = stream.parse().unwrap();
    assert_eq!(value.value, expected);
}
```

## Edge Case Testing

Test boundary conditions:

```rust
#[test]
fn test_empty_input() {
    let stream = TokenStream::lex("").unwrap();
    assert!(stream.is_empty());
}

#[test]
fn test_whitespace_only() {
    let mut stream = TokenStream::lex("   \t\n  ").unwrap();
    // peek_token skips whitespace
    assert!(stream.peek_token().is_none());
}

#[test]
fn test_max_nesting() {
    let nested = "[".repeat(200) + &"]".repeat(200);
    let result = parse_array(&nested);

    // Should fail with recursion limit error
    assert!(matches!(
        result,
        Err(MyError::RecursionLimit { .. })
    ));
}

#[test]
fn test_unicode_boundaries() {
    // Multi-byte UTF-8: emoji is 4 bytes
    let input = "key = \"hello 🦀 world\"";
    let mut stream = TokenStream::lex(input).unwrap();
    let kv: Spanned<KeyValue> = stream.parse().unwrap();

    // Spans should be valid UTF-8 boundaries
    let slice = &input[kv.span.start()..kv.span.end()];
    assert!(slice.is_char_boundary(0));
}
```

## Fuzz Testing

### Setup

Add fuzz targets to your project:

```toml
# fuzz/Cargo.toml
[package]
name = "my-parser-fuzz"
version = "0.0.0"
publish = false
edition = "2021"

[package.metadata]
cargo-fuzz = true

[[bin]]
name = "fuzz_lexer"
path = "fuzz_targets/fuzz_lexer.rs"
test = false
doc = false
bench = false

[[bin]]
name = "fuzz_parser"
path = "fuzz_targets/fuzz_parser.rs"
test = false
doc = false
bench = false

[dependencies]
libfuzzer-sys = "0.4"
my-parser = { path = ".." }
```

### Lexer Fuzzing

```rust
// fuzz/fuzz_targets/fuzz_lexer.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Should never panic
        let _ = my_parser::TokenStream::lex(s);
    }
});
```

### Parser Fuzzing

```rust
// fuzz/fuzz_targets/fuzz_parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(mut stream) = my_parser::TokenStream::lex(s) {
            // Parse should never panic, only return errors
            let _: Result<Document, _> = stream.parse();
        }
    }
});
```

### Running Fuzzers

```bash
# Install cargo-fuzz (requires nightly)
cargo install cargo-fuzz

# Run lexer fuzzer
cargo +nightly fuzz run fuzz_lexer

# Run with timeout and iterations
cargo +nightly fuzz run fuzz_parser -- -max_total_time=60

# Run with corpus
cargo +nightly fuzz run fuzz_parser corpus/parser/
```

## Integration Testing

Test complete workflows:

```rust
#[test]
fn test_parse_real_file() {
    let content = std::fs::read_to_string("fixtures/config.toml").unwrap();
    let doc = parse(&content).expect("should parse real config file");

    // Verify expected structure
    assert!(doc.get_table("server").is_some());
    assert!(doc.get_value("server.port").is_some());
}
```

## Benchmarking

Use `divan` or `criterion` for performance testing:

```rust
use divan::Bencher;

#[divan::bench]
fn bench_lex_small(bencher: Bencher) {
    let input = include_str!("fixtures/small.toml");
    bencher.bench(|| TokenStream::lex(input).unwrap());
}

#[divan::bench(args = [100, 1000, 10000])]
fn bench_lex_lines(bencher: Bencher, lines: usize) {
    let input = "key = \"value\"\n".repeat(lines);
    bencher.bench(|| TokenStream::lex(&input).unwrap());
}
```

## CI Configuration

Example GitHub Actions workflow:

```yaml
name: Test
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all-features

  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - run: cargo install cargo-fuzz
      - run: cargo +nightly fuzz run fuzz_lexer -- -max_total_time=30
      - run: cargo +nightly fuzz run fuzz_parser -- -max_total_time=30
```

## Test Coverage

Use `cargo-llvm-cov` for coverage reports:

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --html
open target/llvm-cov/html/index.html
```

Aim for high coverage on:

- All token variants
- All AST node types
- Error paths
- Edge cases (empty, whitespace, limits)
