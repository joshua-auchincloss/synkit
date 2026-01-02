#![no_main]

use libfuzzer_sys::fuzz_target;
use synkit::parser_kit;

#[derive(Debug, Clone, Default, PartialEq)]
pub enum ParseError {
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
    error: ParseError,
    skip_tokens: [Space, Newline],
    tokens: {
        #[token(" ", priority = 0)]
        Space,
        #[regex(r"\r?\n")]
        Newline,
        #[token("{")]
        LBrace,
        #[token("}")]
        RBrace,
        #[token("(")]
        LParen,
        #[token(")")]
        RParen,
        #[token("[")]
        LBracket,
        #[token("]")]
        RBracket,
        #[token(":")]
        Colon,
        #[token(",")]
        Comma,
        #[token(";")]
        Semi,
        #[regex(r"[A-Za-z_][A-Za-z0-9_]*", |lex| lex.slice().to_string())]
        Ident(String),
        #[regex(r"-?[0-9]+", |lex| lex.slice().parse().ok())]
        Number(i64),
        #[regex(r#""([^"\\]|\\.)*""#, |lex| {
            let s = lex.slice();
            s[1..s.len().saturating_sub(1)].to_string()
        })]
        String(String),
    },
    delimiters: {
        Brace => (LBrace, RBrace),
        Paren => (LParen, RParen),
        Bracket => (LBracket, RBracket),
    },
}

// Use the generated Parse trait from the module
use traits::Parse;

#[derive(Debug, Clone)]
struct KeyValue {
    key: span::Spanned<tokens::IdentToken>,
    colon: span::Spanned<tokens::ColonToken>,
    value: Value,
}

#[derive(Debug, Clone)]
enum Value {
    Number(span::Spanned<tokens::NumberToken>),
    String(span::Spanned<tokens::StringToken>),
    Ident(span::Spanned<tokens::IdentToken>),
}

impl Parse for KeyValue {
    fn parse(stream: &mut stream::TokenStream) -> Result<Self, ParseError> {
        let key = stream.parse()?;
        let colon = stream.parse()?;
        let value: Value = Value::parse(stream)?;
        Ok(Self { key, colon, value })
    }
}

impl traits::Peek for Value {
    fn is(token: &tokens::Token) -> bool {
        matches!(
            token,
            tokens::Token::Number(_) | tokens::Token::String(_) | tokens::Token::Ident(_)
        )
    }
}

impl Parse for Value {
    fn parse(stream: &mut stream::TokenStream) -> Result<Self, ParseError> {
        use synkit::TokenStream as _;
        if let Some(tok) = stream.peek_token() {
            match &tok.value {
                tokens::Token::Number(_) => Ok(Value::Number(stream.parse()?)),
                tokens::Token::String(_) => Ok(Value::String(stream.parse()?)),
                tokens::Token::Ident(_) => Ok(Value::Ident(stream.parse()?)),
                _ => Err(ParseError::Expected {
                    expect: "value",
                    found: format!("{:?}", tok.value),
                }),
            }
        } else {
            Err(ParseError::Empty { expect: "value" })
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(mut stream) = stream::TokenStream::lex(s) {
            use synkit::{SpanLike, TokenStream as _};

            while stream.peek_token().is_some() {
                match stream.parse::<KeyValue>() {
                    Ok(kv) => {
                        let _ = kv.key.span.len();
                        let _ = kv.colon.span.join(&kv.key.span);
                    }
                    Err(_) => {
                        let _ = stream.next();
                    }
                }
            }
        }
    }
});
