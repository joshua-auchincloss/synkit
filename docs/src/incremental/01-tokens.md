# Token Definition

> **📦 Source**: [examples/jsonl-parser/src/lib.rs](https://github.com/joshua-auchincloss/synkit/blob/main/examples/jsonl-parser/src/lib.rs)

## Error Type

Define a parser error type with `thiserror`:

```rust,ignore
use thiserror::Error;

#[derive(Error, Debug, Clone, Default, PartialEq)]
pub enum JsonError {
    #[default]
    #[error("unknown lexing error")]
    Unknown,

    #[error("expected {expect}, found {found}")]
    Expected { expect: &'static str, found: String },

    #[error("expected {expect}, found EOF")]
    Empty { expect: &'static str },

    #[error("invalid number: {0}")]
    InvalidNumber(String),

    #[error("invalid escape sequence")]
    InvalidEscape,

    #[error("{source}")]
    Spanned {
        #[source]
        source: Box<JsonError>,
        span: Span,
    },
}
```

## Token Definition

Use `parser_kit!` to define JSON tokens:

```rust,ignore
synkit::parser_kit! {
    error: JsonError,

    skip_tokens: [Space, Tab],

    tokens: {
        // Whitespace (auto-skipped during parsing)
        #[token(" ", priority = 0)]
        Space,

        #[token("\t", priority = 0)]
        Tab,

        // Newline is significant - it's our record delimiter
        #[regex(r"\r?\n")]
        #[fmt("newline")]
        #[no_to_tokens]  // Custom ToTokens impl
        Newline,

        // Structural tokens
        #[token("{")]
        LBrace,

        #[token("}")]
        RBrace,

        #[token("[")]
        LBracket,

        #[token("]")]
        RBracket,

        #[token(":")]
        Colon,

        #[token(",")]
        Comma,

        // Literals
        #[token("null")]
        Null,

        #[token("true")]
        True,

        #[token("false")]
        False,

        // Strings with escape sequences
        #[regex(r#""([^"\\]|\\.)*""#, |lex| {
            let s = lex.slice();
            s[1..s.len()-1].to_string()  // Strip quotes
        })]
        #[fmt("string")]
        #[no_to_tokens]
        String(String),

        // JSON numbers (integers and floats)
        #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?",
                |lex| lex.slice().to_string())]
        #[fmt("number")]
        Number(String),
    },

    delimiters: {
        Brace => (LBrace, RBrace),
        Bracket => (LBracket, RBracket),
    },
}
```

## Key Points

### `#[no_to_tokens]` for Custom Printing

Some tokens need custom `ToTokens` implementations:

```rust,ignore
impl traits::ToTokens for tokens::StringToken {
    fn write(&self, p: &mut printer::Printer) {
        use synkit::Printer as _;
        p.word("\"");
        for c in self.0.chars() {
            match c {
                '"' => p.word("\\\""),
                '\\' => p.word("\\\\"),
                '\n' => p.word("\\n"),
                '\r' => p.word("\\r"),
                '\t' => p.word("\\t"),
                c => p.char(c),
            }
        }
        p.word("\"");
    }
}
```

### Newline as Boundary

Unlike whitespace, `Newline` is semantically significant in JSONL - it separates records. Keep it in the token stream but handle it specially in parsing.

## Next

[Chapter 2: Chunk Boundaries →](02-boundaries.md)
