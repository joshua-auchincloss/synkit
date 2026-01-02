# Spans & Errors

Spans track source locations for error reporting and source mapping.

## Span Types

### `RawSpan`

Byte offsets into source:

```rust,ignore
pub struct RawSpan {
    pub start: usize,
    pub end: usize,
}
```

### `Span`

Handles both known and synthetic locations:

```rust,ignore
pub enum Span {
    CallSite,           // No source location (generated code)
    Known(RawSpan),     // Actual source position
}
```

## `Spanned<T>`

Wraps a value with its source span:

```rust,ignore
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}
```

**Always use `Spanned<T>` for AST node fields:**

```rust,ignore
pub struct KeyValue {
    pub key: Spanned<Key>,        // ✓
    pub eq: Spanned<EqToken>,     // ✓
    pub value: Spanned<Value>,    // ✓
}
```

This enables:
- Precise error locations
- Source mapping for transformations
- Hover information in editors

## Error Handling

### Error Type Pattern

```rust,ignore
#[derive(Error, Debug, Clone, Default, PartialEq)]
pub enum MyError {
    #[default]
    #[error("unknown token")]
    Unknown,

    #[error("expected {expect}, found {found}")]
    Expected { expect: &'static str, found: String },

    #[error("expected {expect}")]
    Empty { expect: &'static str },

    #[error("{source}")]
    Spanned {
        #[source]
        source: Box<MyError>,
        span: Span,
    },
}
```

### SpannedError Trait

Attach spans to errors:

```rust,ignore
impl synkit::SpannedError for MyError {
    type Span = Span;

    fn with_span(self, span: Span) -> Self {
        Self::Spanned {
            source: Box::new(self),
            span,
        }
    }

    fn span(&self) -> Option<&Span> {
        match self {
            Self::Spanned { span, .. } => Some(span),
            _ => None,
        }
    }
}
```

### Diagnostic Trait

Provide display names for error messages:

```rust,ignore
pub trait Diagnostic {
    fn fmt() -> &'static str;
}

// Auto-implemented for tokens using #[fmt(...)] or snake_case name
impl Diagnostic for IdentToken {
    fn fmt() -> &'static str { "identifier" }
}
```

### Error Helpers

```rust,ignore
impl MyError {
    pub fn expected<D: Diagnostic>(found: &Token) -> Self {
        Self::Expected {
            expect: D::fmt(),
            found: format!("{}", found),
        }
    }

    pub fn empty<D: Diagnostic>() -> Self {
        Self::Empty { expect: D::fmt() }
    }
}
```

## Error Propagation

Parse implementations automatically wrap errors with spans:

```rust,ignore
impl Parse for KeyValue {
    fn parse(stream: &mut TokenStream) -> Result<Self, MyError> {
        Ok(Self {
            // If parse fails, error includes span of failed token
            key: stream.parse()?,
            eq: stream.parse()?,
            value: stream.parse()?,
        })
    }
}
```

## Accessing Spans

```rust,ignore
let kv: Spanned<KeyValue> = stream.parse()?;

// Get span of entire key-value
let full_span = &kv.span;

// Get span of just the key
let key_span = &kv.value.key.span;

// Get span of the value
let value_span = &kv.value.value.span;
```
