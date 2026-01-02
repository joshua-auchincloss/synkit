# Parse Implementations

Convert token streams into AST nodes.

## Basic Pattern

```rust,ignore
impl Parse for MyNode {
    fn parse(stream: &mut TokenStream) -> Result<Self, TomlError> {
        Ok(Self {
            field1: stream.parse()?,
            field2: stream.parse()?,
        })
    }
}
```

## Implementing Peek

For types used in conditionals:

```rust,ignore
{{#include ../../../examples/toml-parser/src/parse.rs:parse_simple_key}}
```

`Peek::is()` checks a token variant; `Peek::peek()` checks the stream's next token.

## Parsing Keys

```rust,ignore
{{#include ../../../examples/toml-parser/src/parse.rs:parse_key}}
```

## Parsing Values

Match on peeked token to determine variant:

```rust,ignore
{{#include ../../../examples/toml-parser/src/parse.rs:parse_value}}
```

## Arrays with Delimiters

Use the `bracket!` macro to extract delimited content:

```rust,ignore
{{#include ../../../examples/toml-parser/src/parse.rs:parse_array}}
```

Key points:
- `bracket!(inner in stream)` extracts content between `[` and `]`
- Returns a `Bracket` struct with span information
- `inner` is a new `TokenStream` containing only bracket contents

## Inline Tables

Similar pattern with `brace!`:

```rust,ignore
{{#include ../../../examples/toml-parser/src/parse.rs:parse_inline_table}}
```

## Tables and Documents

```rust,ignore
{{#include ../../../examples/toml-parser/src/parse.rs:parse_table}}
```

```rust,ignore
{{#include ../../../examples/toml-parser/src/parse.rs:parse_document}}
```

## Error Handling

### Expected Token Errors

```rust,ignore
Some(other) => Err(TomlError::Expected {
    expect: "key",
    found: format!("{}", other),
}),
```

### EOF Errors

```rust,ignore
None => Err(TomlError::Empty { expect: "key" }),
```

### Using Diagnostic

```rust,ignore
// Auto-generated for tokens
impl Diagnostic for BareKeyToken {
    fn fmt() -> &'static str { "bare key" }  // From #[fmt("bare key")]
}

// Use in errors
Err(TomlError::expected::<BareKeyToken>(found_token))
```

## Parsing Tips

### Use `peek` Before Consuming

```rust,ignore
if SimpleKey::peek(stream) {
    // Safe to parse
    let key: Spanned<SimpleKey> = stream.parse()?;
}
```

### Fork for Lookahead

```rust,ignore
let mut fork = stream.fork();
if try_parse(&mut fork).is_ok() {
    stream.advance_to(&fork);
}
```

### Handle Optional Elements

```rust,ignore
// Option<T> auto-implements Parse if T implements Peek
let comma: Option<Spanned<CommaToken>> = stream.parse()?;
```

### Raw Token Access

For tokens in `skip_tokens` (like `Newline`):

```rust,ignore
// Use peek_token_raw to see skipped tokens
fn peek_raw(stream: &TokenStream) -> Option<&Token> {
    stream.peek_token_raw().map(|t| &t.value)
}
```
