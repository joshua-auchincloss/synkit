# synkit

[![Crates.io Version](https://img.shields.io/crates/v/synkit?style=flat-square)](https://crates.io/crates/synkit)
[![Crates.io Version](https://img.shields.io/crates/v/synkit-macros?style=flat-square)](https://crates.io/crates/synkit-macros)
[![Crates.io Version](https://img.shields.io/crates/v/synkit-core?style=flat-square)](https://crates.io/crates/synkit-core)

A toolkit for building round-trip parsers with [logos](https://github.com/maciejhirsz/logos). Generates syn-like parsing infrastructure from token definitions.

> [!NOTE]
> This project is not affiliated with or endorsed by the logos project. It is an independent extension library built on top of logos.

> [!NOTE]
> Originally extracted from [kintsu/kintsu](https://github.com/kintsu/kintsu) (`parser/`) to eliminate boilerplate across projects.

## Features

- Declarative token and grammar definition via proc macros
- Auto-generated `Parse`, `Peek`, `ToTokens`, `Diagnostic` traits with concrete types
- Span tracking and spanned error propagation
- Whitespace-skipping token streams with fork/rewind
- Round-trip formatting via `Printer` trait
- Delimiter extraction helpers (braces, parens, brackets)
- **Async streaming support** with `IncrementalParse` trait (tokio/futures)
- Stream validation with `ensure_consumed()` helper

## Installation

```toml
[dependencies]
synkit = "0.1"
thiserror = "2"

# Optional: for async streaming
synkit = { version = "0.1", features = ["tokio"] }
# or
synkit = { version = "0.1", features = ["futures"] }
```

## Example

```rust,ignore
use thiserror::Error;

#[derive(Error, Debug, Clone, Default, PartialEq)]
pub enum LexError {
    #[default]
    #[error("unknown")]
    Unknown,
}

synkit::parser_kit! {
    error: LexError,
    skip_tokens: [Space],
    tokens: {
        #[token(" ")]
        Space,

        #[token("let")]
        KwLet,

        #[token("=")]
        Eq,

        #[regex(r"[a-z_][a-z0-9_]*", |lex| lex.slice().to_string())]
        #[fmt("identifier")]
        Ident(String),

        #[regex(r"[0-9]+", |lex| lex.slice().parse().ok())]
        #[fmt("number")]
        Number(i64),
    },
    delimiters: {},
    span_derives: [Debug, Clone, PartialEq, Eq, Hash],
    token_derives: [Clone, PartialEq, Debug],
}

// AST node
pub struct LetBinding {
    pub kw_let: Spanned<tokens::KwLetToken>,
    pub name: Spanned<tokens::IdentToken>,
    pub eq: Spanned<tokens::EqToken>,
    pub value: Spanned<tokens::NumberToken>,
}

impl Parse for LetBinding {
    fn parse(stream: &mut TokenStream) -> Result<Self, LexError> {
        Ok(Self {
            kw_let: stream.parse()?,
            name: stream.parse()?,
            eq: stream.parse()?,
            value: stream.parse()?,
        })
    }
}

// Usage
fn main() -> Result<(), LexError> {
    let mut stream = TokenStream::lex("let x = 42")?;
    let binding: Spanned<LetBinding> = stream.parse()?;

    assert_eq!(*binding.name.value, "x");
    assert_eq!(binding.value.value.0, 42);

    // Validate all tokens consumed
    stream.ensure_consumed()?;
    Ok(())
}
```

## Async Streaming

synkit supports incremental parsing for streaming scenarios (network data, large files):

```rust,ignore
use synkit::async_stream::{IncrementalParse, ParseCheckpoint};
use synkit::async_stream::tokio_impl::AstStream;
use tokio::sync::mpsc;

// Implement IncrementalParse for your AST nodes
impl IncrementalParse for MyNode {
    fn parse_incremental(
        tokens: &[Token],
        checkpoint: &ParseCheckpoint,
    ) -> Result<(Option<Self>, ParseCheckpoint), MyError> {
        // Parse from token buffer, return None if more tokens needed
    }

    fn can_parse(tokens: &[Token], checkpoint: &ParseCheckpoint) -> bool {
        checkpoint.cursor < tokens.len()
    }
}

// Stream tokens through channels
let (token_tx, token_rx) = mpsc::channel(32);
let (ast_tx, mut ast_rx) = mpsc::channel(16);

tokio::spawn(async move {
    let mut parser = AstStream::<MyNode, Token>::new(token_rx, ast_tx);
    parser.run().await?;
});

// Consume AST nodes as they're parsed
while let Some(node) = ast_rx.recv().await {
    process(node);
}
```

## Documentation

Full documentation: [TODO: docs.rs link]

Book: [TODO: book link]

## License

MIT
