#![deny(
    unsafe_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::dbg_macro
)]

//! TOML Parser Example
//!
//! This example demonstrates building a TOML parser using synkit.
//! It supports a minimal TOML subset including:
//! - Key-value pairs
//! - Tables and nested tables
//! - Basic strings, integers, and booleans
//! - Arrays and inline tables

use thiserror::Error;

// ANCHOR: error_type
#[derive(Error, Debug, Clone, Default, PartialEq)]
pub enum TomlError {
    #[default]
    #[error("unknown lexing error")]
    Unknown,

    #[error("expected {expect}, found {found}")]
    Expected { expect: &'static str, found: String },

    #[error("expected {expect}, found EOF")]
    Empty { expect: &'static str },

    #[error("unclosed string")]
    UnclosedString,

    #[error("{source}")]
    Spanned {
        #[source]
        source: Box<TomlError>,
        span: Span,
    },
}
// ANCHOR_END: error_type

// ANCHOR: token_def
synkit::parser_kit! {
    error: TomlError,

    skip_tokens: [Space, Tab],

    tokens: {
        // Whitespace
        #[token(" ", priority = 0)]
        Space,

        #[token("\t", priority = 0)]
        Tab,

        #[regex(r"\r?\n")]
        #[fmt("newline")]
        #[no_to_tokens]
        Newline,

        // Comments
        #[regex(r"#[^\n]*", allow_greedy = true)]
        #[fmt("comment")]
        Comment,

        // Punctuation
        #[token("=")]
        Eq,

        #[token(".")]
        Dot,

        #[token(",")]
        Comma,

        #[token("[")]
        LBracket,

        #[token("]")]
        RBracket,

        #[token("{")]
        LBrace,

        #[token("}")]
        RBrace,

        // Keywords/literals
        #[token("true")]
        True,

        #[token("false")]
        False,

        // Bare keys: alphanumeric, underscores, dashes
        #[regex(r"[A-Za-z0-9_-]+", |lex| lex.slice().to_string(), priority = 1)]
        #[fmt("bare key")]
        #[derive(PartialOrd, Ord, Hash, Eq)]
        BareKey(String),

        // Basic strings (double-quoted) - needs custom ToTokens for quote handling
        #[regex(r#""([^"\\]|\\.)*""#, |lex| {
            let s = lex.slice();
            // Remove surrounding quotes
            s[1..s.len()-1].to_string()
        })]
        #[fmt("string")]
        #[no_to_tokens]
        BasicString(String),

        // Integers
        #[regex(r"-?[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
        #[fmt("integer")]
        Integer(i64),
    },

    delimiters: {
        Bracket => (LBracket, RBracket),
        Brace => (LBrace, RBrace),
    },

    span_derives: [Debug, Clone, PartialEq, Eq, Hash, Copy],
    token_derives: [Clone, PartialEq, Debug],
}
// ANCHOR_END: token_def

// ANCHOR: error_impl
impl TomlError {
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

impl synkit::SpannedError for TomlError {
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
pub mod print;
pub mod visitor;

pub mod incremental;

pub use ast::*;
