# Parsing

Parsing converts a token stream into an AST using the `Parse` and `Peek` traits.

## The `Parse` Trait

```rust,ignore
pub trait Parse: Sized {
    fn parse(stream: &mut TokenStream) -> Result<Self, Error>;
}
```

Token structs implement `Parse` automatically. For AST nodes, implement manually:

```rust,ignore
impl Parse for KeyValue {
    fn parse(stream: &mut TokenStream) -> Result<Self, TomlError> {
        Ok(Self {
            key: stream.parse()?,
            eq: stream.parse()?,
            value: stream.parse()?,
        })
    }
}
```

## The `Peek` Trait

Check the next token without consuming:

```rust,ignore
pub trait Peek {
    fn is(token: &Token) -> bool;
    fn peek(stream: &TokenStream) -> bool;
}
```

Use in conditionals and loops:

```rust,ignore
// Check before parsing
if SimpleKey::peek(stream) {
    let key: Spanned<SimpleKey> = stream.parse()?;
}

// Parse while condition holds
while Value::peek(stream) {
    items.push(stream.parse()?);
}
```

## TokenStream Operations

### Basic Operations

```rust,ignore
// Create from source
let mut stream = TokenStream::lex("x = 42")?;

// Parse with type inference
let token: Spanned<IdentToken> = stream.parse()?;

// Peek at next token
if stream.peek::<EqToken>() {
    // ...
}

// Get next raw token (including skipped)
let raw = stream.next_raw();
```

### Fork and Rewind

Speculatively parse without committing:

```rust,ignore
let fork = stream.fork();
if let Ok(result) = try_parse(&mut fork) {
    stream.advance_to(&fork);  // Commit
    return Ok(result);
}
// Didn't advance - stream unchanged
```

### Whitespace Handling

`skip_tokens` in `parser_kit!` defines tokens to skip:

```rust,ignore
skip_tokens: [Space, Tab],
```

- `stream.next()` - Skips whitespace
- `stream.next_raw()` - Includes whitespace
- `stream.peek_token()` - Skips whitespace
- `stream.peek_token_raw()` - Includes whitespace

## Parsing Patterns

### Sequential Fields

```rust,ignore
impl Parse for Assignment {
    fn parse(stream: &mut TokenStream) -> Result<Self, Error> {
        Ok(Self {
            name: stream.parse()?,   // Spanned<IdentToken>
            eq: stream.parse()?,     // Spanned<EqToken>
            value: stream.parse()?,  // Spanned<Value>
        })
    }
}
```

### Enum Variants

Use `peek` to determine variant:

```rust,ignore
impl Parse for Value {
    fn parse(stream: &mut TokenStream) -> Result<Self, Error> {
        if stream.peek::<IntegerToken>() {
            Ok(Value::Integer(stream.parse()?))
        } else if stream.peek::<StringToken>() {
            Ok(Value::String(stream.parse()?))
        } else {
            Err(Error::expected("value"))
        }
    }
}
```

### Optional Fields

```rust,ignore
// Option<T> auto-implements Parse via Peek
let comma: Option<Spanned<CommaToken>> = stream.parse()?;
```

### Repeated Items

```rust,ignore
// Manual loop
let mut items = Vec::new();
while Value::peek(stream) {
    items.push(stream.parse()?);
}

// Using synkit::Repeated
use synkit::Repeated;
let items: Repeated<Value, CommaToken, Spanned<Value>> =
    Repeated::parse(stream)?;
```

### Delimited Content

Extract content between delimiters:

```rust,ignore
// Using the bracket! macro
let mut inner;
let bracket = bracket!(inner in stream);

// inner is a new TokenStream with bracket contents
let items = parse_items(&mut inner)?;
```
