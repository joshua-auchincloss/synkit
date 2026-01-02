# Core Traits

synkit provides traits in two locations:
- **`synkit` (synkit-core)**: Generic traits for library-level abstractions
- **Generated `traits` module**: Concrete implementations for your grammar

## Parse

Convert tokens to AST nodes.

```rust,ignore
pub trait Parse: Sized {
    fn parse(stream: &mut TokenStream) -> Result<Self, Error>;
}
```

### Auto-implementations

Token structs implement `Parse` automatically:

```rust,ignore
// Generated for EqToken
impl Parse for EqToken {
    fn parse(stream: &mut TokenStream) -> Result<Self, Error> {
        match stream.next() {
            Some(tok) => match &tok.value {
                Token::Eq => Ok(EqToken::new()),
                other => Err(Error::expected::<Self>(other)),
            },
            None => Err(Error::empty::<Self>()),
        }
    }
}
```

### Blanket Implementations

```rust,ignore
// Option<T> parses if T::peek() succeeds
impl<T: Parse + Peek> Parse for Option<T> { ... }

// Box<T> wraps parsed value
impl<T: Parse> Parse for Box<T> { ... }

// Spanned<T> wraps with span
impl<T: Parse> Parse for Spanned<T> { ... }
```

## Peek

Check next token without consuming.

```rust,ignore
pub trait Peek {
    fn is(token: &Token) -> bool;
    fn peek(stream: &TokenStream) -> bool;
}
```

### Usage

```rust,ignore
// In conditionals
if Value::peek(stream) {
    let v: Spanned<Value> = stream.parse()?;
}

// In loops
while SimpleKey::peek(stream) {
    items.push(stream.parse()?);
}
```

## ToTokens

Convert AST back to formatted output.

```rust,ignore
pub trait ToTokens {
    fn write(&self, printer: &mut Printer);

    fn to_string_formatted(&self) -> String {
        let mut p = Printer::new();
        self.write(&mut p);
        p.finish()
    }
}
```

### Blanket Implementations

```rust,ignore
impl<T: ToTokens> ToTokens for Spanned<T> {
    fn write(&self, p: &mut Printer) {
        self.value.write(p);
    }
}

impl<T: ToTokens> ToTokens for Option<T> {
    fn write(&self, p: &mut Printer) {
        if let Some(v) = self { v.write(p); }
    }
}
```

## Diagnostic

Provide display name for error messages.

```rust,ignore
pub trait Diagnostic {
    fn fmt() -> &'static str;
}
```

Auto-generated using `#[fmt("...")]` or snake_case variant name.

## IncrementalParse

Parse AST nodes incrementally from a token buffer with checkpoint-based state.

```rust,ignore
pub trait IncrementalParse: Sized {
    fn parse_incremental(
        tokens: &[Token],
        checkpoint: &ParseCheckpoint,
    ) -> Result<(Option<Self>, ParseCheckpoint), Error>;

    fn can_parse(tokens: &[Token], checkpoint: &ParseCheckpoint) -> bool;
}
```

### Usage

```rust,ignore
impl IncrementalParse for KeyValue {
    fn parse_incremental(
        tokens: &[Token],
        checkpoint: &ParseCheckpoint,
    ) -> Result<(Option<Self>, ParseCheckpoint), TomlError> {
        let cursor = checkpoint.cursor;

        // Need at least 3 tokens: key = value
        if cursor + 2 >= tokens.len() {
            return Ok((None, checkpoint.clone()));
        }

        // Parse key = value pattern
        // ...

        let new_cp = ParseCheckpoint {
            cursor: cursor + 3,
            tokens_consumed: checkpoint.tokens_consumed + 3,
            state: 0,
        };
        Ok((Some(kv), new_cp))
    }

    fn can_parse(tokens: &[Token], checkpoint: &ParseCheckpoint) -> bool {
        checkpoint.cursor < tokens.len()
    }
}
```

### With Async Streaming

```rust,ignore
use synkit::async_stream::tokio_impl::AstStream;

let (token_tx, token_rx) = mpsc::channel(32);
let (ast_tx, mut ast_rx) = mpsc::channel(16);

tokio::spawn(async move {
    let mut parser = AstStream::<KeyValue, Token>::new(token_rx, ast_tx);
    parser.run().await?;
});

while let Some(kv) = ast_rx.recv().await {
    process_key_value(kv);
}
```

## TokenStream (core trait)

Generic stream interface from `synkit-core`:

```rust,ignore
pub trait TokenStream {
    type Token;
    type Span;
    type Spanned;
    type Error;

    fn next(&mut self) -> Option<Self::Spanned>;
    fn peek_token(&self) -> Option<&Self::Spanned>;
    fn next_raw(&mut self) -> Option<Self::Spanned>;
    fn peek_token_raw(&self) -> Option<&Self::Spanned>;
}
```

The generated `stream::TokenStream` implements this trait.

## Printer (core trait)

Generic printer interface from `synkit-core`:

```rust,ignore
pub trait Printer {
    fn word(&mut self, s: &str);
    fn token<T: std::fmt::Display>(&mut self, tok: &T);
    fn space(&mut self);
    fn newline(&mut self);
    fn open_block(&mut self);
    fn close_block(&mut self);
    fn indent(&mut self);
    fn dedent(&mut self);
    fn write_separated<T, F>(&mut self, items: &[T], sep: &str, f: F)
    where F: Fn(&T, &mut Self);
}
```

## SpannedError

Attach source spans to errors:

```rust,ignore
pub trait SpannedError: Sized {
    type Span;

    fn with_span(self, span: Self::Span) -> Self;
    fn span(&self) -> Option<&Self::Span>;
}
```

Implementation pattern:

```rust,ignore
impl SpannedError for MyError {
    type Span = Span;

    fn with_span(self, span: Span) -> Self {
        Self::Spanned { source: Box::new(self), span }
    }

    fn span(&self) -> Option<&Span> {
        match self {
            Self::Spanned { span, .. } => Some(span),
            _ => None,
        }
    }
}
```

## SpanLike / SpannedLike

Abstractions for span types:

```rust,ignore
pub trait SpanLike {
    fn call_site() -> Self;
    fn new(start: usize, end: usize) -> Self;
}

pub trait SpannedLike<T> {
    type Span: SpanLike;
    fn new(value: T, span: Self::Span) -> Self;
    fn value(&self) -> &T;
    fn span(&self) -> &Self::Span;
}
```

Enable generic code over different span implementations.
