#![allow(clippy::len_without_is_empty)]
#![deny(
    unsafe_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::dbg_macro
)]

//! JSON Lines Parser Example
//!
//! This example demonstrates building a JSON Lines (JSONL) parser using synkit
//! for streaming/incremental parsing scenarios. JSON Lines is a format where
//! each line is a valid JSON object, making it ideal for streaming.
//!
//! # Features
//!
//! - Complete JSON value parsing (objects, arrays, strings, numbers, booleans, null)
//! - Incremental parsing with newline delimiters
//! - Designed for high-throughput streaming scenarios
//! - Memory-efficient: parsed values are emitted and released immediately
//!
//! # Format
//!
//! ```text
//! {"name": "Alice", "age": 30}
//! {"name": "Bob", "age": 25}
//! {"name": "Charlie", "age": 35}
//! ```

use thiserror::Error;

// ANCHOR: error_type
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
// ANCHOR_END: error_type

// ANCHOR: token_def
synkit::parser_kit! {
    error: JsonError,

    skip_tokens: [Space, Tab],

    tokens: {
        // Whitespace (skipped)
        #[token(" ", priority = 0)]
        Space,

        #[token("\t", priority = 0)]
        Tab,

        // Newline is significant for JSONL - it's our record delimiter
        #[regex(r"\r?\n")]
        #[fmt("newline")]
        #[no_to_tokens]
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

        // Strings (with escape handling)
        #[regex(r#""([^"\\]|\\.)*""#, |lex| {
            let s = lex.slice();
            // Remove surrounding quotes, keep escapes as-is for now
            s[1..s.len()-1].to_string()
        })]
        #[fmt("string")]
        #[no_to_tokens]
        String(String),

        // Numbers (integers and floats)
        #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", |lex| lex.slice().to_string())]
        #[fmt("number")]
        Number(String),
    },

    delimiters: {
        Brace => (LBrace, RBrace),
        Bracket => (LBracket, RBracket),
    },

    span_derives: [Debug, Clone, PartialEq, Eq, Hash, Copy],
    token_derives: [Clone, PartialEq, Debug],
}
// ANCHOR_END: token_def

// ANCHOR: to_tokens_impl
// Custom ToTokens implementations for tokens marked with #[no_to_tokens]
impl traits::ToTokens for tokens::NewlineToken {
    fn write(&self, p: &mut printer::Printer) {
        use synkit::Printer as _;
        p.newline();
    }
}

impl traits::ToTokens for tokens::StringToken {
    fn write(&self, p: &mut printer::Printer) {
        use synkit::Printer as _;
        // Output the string with surrounding quotes
        p.word("\"");
        // Escape special characters
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
// ANCHOR_END: to_tokens_impl

// ANCHOR: error_impl
impl JsonError {
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

impl synkit::SpannedError for JsonError {
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
// ANCHOR_END: error_impl

pub mod ast;
pub mod parse;

pub mod incremental;

pub use ast::*;

#[cfg(test)]
mod tests {
    use super::*;
    use synkit::TokenStream as _;

    #[test]
    fn test_lex_simple_object() {
        let input = r#"{"name": "Alice", "age": 30}"#;
        let stream = TokenStream::lex(input).unwrap();

        // Should lex without errors
        assert!(stream.peek_token().is_some());
    }

    #[test]
    fn test_lex_jsonl() {
        let input = r#"{"a": 1}
{"b": 2}
{"c": 3}"#;
        let mut stream = TokenStream::lex(input).unwrap();

        // Count newlines
        let mut newline_count = 0;
        while let Some(tok) = stream.next_raw() {
            if matches!(tok.value, Token::Newline) {
                newline_count += 1;
            }
        }
        assert_eq!(newline_count, 2);
    }

    #[test]
    fn test_lex_string_escapes() {
        let input = r#""hello \"world\"""#;
        let stream = TokenStream::lex(input).unwrap();
        let tok = stream.peek_token().unwrap();

        if let Token::String(s) = &tok.value {
            assert_eq!(s, r#"hello \"world\""#);
        } else {
            panic!("expected string token");
        }
    }

    #[test]
    fn test_lex_numbers() {
        let inputs = ["42", "-17", "3.14", "2.5e10", "-1.5E-3"];
        for input in inputs {
            let stream = TokenStream::lex(input).unwrap();
            let tok = stream.peek_token().unwrap();
            assert!(
                matches!(tok.value, Token::Number(_)),
                "failed for: {}",
                input
            );
        }
    }
}
