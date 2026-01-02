#![no_main]

use libfuzzer_sys::fuzz_target;
use synkit::parser_kit;

#[derive(Debug, Clone, Default, PartialEq)]
pub enum LexError {
    #[default]
    Unknown,
    Expected {
        expect: &'static str,
        found: String,
    },
    Empty {
        expect: &'static str,
    },
}

parser_kit! {
    error: LexError,
    skip_tokens: [Space],
    tokens: {
        #[token(" ", priority = 0)]
        Space,
        #[token("{")]
        LBrace,
        #[token("}")]
        RBrace,
        #[token("(")]
        LParen,
        #[token(")")]
        RParen,
        #[token(":")]
        Colon,
        #[token(",")]
        Comma,
        #[regex(r"[A-Za-z_][A-Za-z0-9_]*", |lex| lex.slice().to_string())]
        Ident(String),
        #[regex(r"[0-9]+", |lex| lex.slice().parse().ok())]
        Number(i64),
    },
    delimiters: {
        Brace => (LBrace, RBrace),
        Paren => (LParen, RParen),
    },
}

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(stream) = stream::TokenStream::lex(s) {
            use synkit::TokenStream as _;

            for tok in stream.all() {
                let _ = tok.span.len();
                let _ = tok.span.is_empty();
            }
        }
    }
});
