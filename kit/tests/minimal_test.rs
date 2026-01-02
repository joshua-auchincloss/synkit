//! Minimal test to verify basic macro expansion.

use thiserror::Error;

#[derive(Error, Debug, Clone, Default, PartialEq)]
pub enum LexError {
    #[default]
    #[error("unknown")]
    Unknown,

    #[error("expected {expect}, found {found}")]
    Expected { expect: &'static str, found: String },

    #[error("expected {expect}, found EOF")]
    Empty { expect: &'static str },
}

synkit::parser_kit! {
    error: LexError,

    skip_tokens: [],

    tokens: {
        #[token("a")]
        A,
    },

    delimiters: {},

    span_derives: [Debug, Clone, PartialEq, Eq, Hash],
    token_derives: [Clone, PartialEq, Debug],
}

#[test]
fn basic_lex() {
    let ts = stream::TokenStream::lex("a").expect("lex failed");
    assert_eq!(ts.all().len(), 1);
}
