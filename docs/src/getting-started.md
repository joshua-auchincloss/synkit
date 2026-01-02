# Getting Started

## Installation

Add synkit and logos to your `Cargo.toml`:

```toml
[dependencies]
synkit = "0.1"
logos = "0.15"
thiserror = "2"  # recommended for error types
```

### Optional Features

```toml
# For async streaming with tokio
synkit = { version = "0.1", features = ["tokio"] }

# For async streaming with futures (runtime-agnostic)
synkit = { version = "0.1", features = ["futures"] }

# For std::error::Error implementations
synkit = { version = "0.1", features = ["std"] }
```

## Minimal Example

A complete parser in ~30 lines:

```rust,ignore
use thiserror::Error;

#[derive(Error, Debug, Clone, Default, PartialEq)]
pub enum LexError {
    #[default]
    #[error("unknown token")]
    Unknown,
    #[error("expected {expect}, found {found}")]
    Expected { expect: &'static str, found: String },
    #[error("expected {expect}")]
    Empty { expect: &'static str },
}

synkit::parser_kit! {
    error: LexError,
    skip_tokens: [Space],
    tokens: {
        #[token(" ")]
        Space,

        #[token("=")]
        Eq,

        #[regex(r"[a-z]+", |lex| lex.slice().to_string())]
        #[fmt("identifier")]
        Ident(String),

        #[regex(r"[0-9]+", |lex| lex.slice().parse().ok())]
        #[fmt("number")]
        Number(i64),
    },
    delimiters: {},
    span_derives: [Debug, Clone, PartialEq],
    token_derives: [Debug, Clone, PartialEq],
}
```

## Using the Generated Code

After `parser_kit!`, you have access to:

```rust,ignore
use crate::{
    // Span types
    Span, Spanned,
    // Token enum and structs
    tokens::{Token, EqToken, IdentToken, NumberToken},
    // Parsing infrastructure
    stream::TokenStream,
    // Traits
    Parse, Peek, ToTokens, Diagnostic,
};

// Lex source into tokens
let mut stream = TokenStream::lex("x = 42")?;

// Parse tokens
let name: Spanned<IdentToken> = stream.parse()?;
let eq: Spanned<EqToken> = stream.parse()?;
let value: Spanned<NumberToken> = stream.parse()?;

assert_eq!(*name.value, "x");
assert_eq!(value.value.0, 42);
```

## Generated Modules

`parser_kit!` generates these modules in your crate:

| Module | Contents |
|--------|----------|
| `span` | `Span`, `RawSpan`, `Spanned<T>` |
| `tokens` | `Token` enum, `*Token` structs, `Tok!`/`SpannedTok!` macros |
| `stream` | `TokenStream`, `MutTokenStream` |
| `printer` | `Printer` implementation |
| `delimiters` | Delimiter structs (e.g., `Bracket`, `Brace`) |
| `traits` | `Parse`, `Peek`, `ToTokens`, `Diagnostic` |

## Error Type Requirements

Your error type must:

1. Implement `Default` (for unknown token errors from logos)
2. Have variants for parse errors (recommended pattern):

```rust,ignore
#[derive(Error, Debug, Clone, Default, PartialEq)]
pub enum MyError {
    #[default]
    #[error("unknown")]
    Unknown,

    #[error("expected {expect}, found {found}")]
    Expected { expect: &'static str, found: String },

    #[error("expected {expect}")]
    Empty { expect: &'static str },
}
```

## Next Steps

- [Concepts](concepts/README.md) - Understand tokens, parsing, spans
- [Tutorial](tutorial/README.md) - Build a complete TOML parser
- [Reference](reference/parser-kit.md) - Full macro documentation
