//! Span Accuracy Tests
//!
//! Tests that generated code produces correct source spans for all token types
//! and AST nodes. Verifies exact byte offsets for span accuracy.

use synkit::{SpanLike, SpannedLike, TokenStream as _};
use thiserror::Error;

#[derive(Error, Debug, Clone, Default, PartialEq)]
pub enum TestError {
    #[default]
    #[error("unknown error")]
    Unknown,

    #[error("expected {expect}, found {found}")]
    Expected { expect: &'static str, found: String },

    #[error("expected {expect}, found EOF")]
    Empty { expect: &'static str },
}

synkit::parser_kit! {
    error: TestError,

    skip_tokens: [Whitespace],

    tokens: {
        #[token(" ", priority = 0)]
        #[token("\t", priority = 0)]
        #[token("\n", priority = 0)]
        Whitespace,

        #[token("struct")]
        KwStruct,

        #[token("enum")]
        KwEnum,

        #[token("fn")]
        KwFn,

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

        #[token("->")]
        Arrow,

        #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
        Ident(String),

        #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().unwrap())]
        Number(i64),

        #[regex(r#""[^"]*""#, |lex| {
            let s = lex.slice();
            s[1..s.len()-1].to_string()
        })]
        String(String),
    },

    delimiters: {
        Brace => (LBrace, RBrace),
        Paren => (LParen, RParen),
    },

    span_derives: [Debug, Clone, PartialEq, Eq, Hash, Copy],
    token_derives: [Clone, PartialEq, Debug],
}

/// Helper to assert exact span position
fn assert_span(span: &span::Span, source: &str, expected_text: &str, expected_start: usize) {
    let start = span.start();
    let end = span.end();
    let actual_text = &source[start..end];

    assert_eq!(
        actual_text, expected_text,
        "span text mismatch at {}..{}: expected {:?}, got {:?}",
        start, end, expected_text, actual_text
    );
    assert_eq!(
        start, expected_start,
        "span start mismatch: expected {}, got {}",
        expected_start, start
    );
    assert_eq!(
        span.len(),
        expected_text.len(),
        "span len mismatch: expected {}, got {}",
        expected_text.len(),
        span.len()
    );
}

mod token_span_tests {
    use super::*;

    #[test]
    fn span_single_keyword() {
        let source = "struct";
        let ts = stream::TokenStream::lex(source).expect("lexing failed");
        let all = ts.all();

        assert_eq!(all.len(), 1);
        assert_span(&all[0].span, source, "struct", 0);
    }

    #[test]
    fn span_keyword_with_whitespace() {
        let source = "  struct  ";
        let ts = stream::TokenStream::lex(source).expect("lexing failed");
        let all = ts.all();

        // Tokens: ws, ws, struct, ws, ws
        assert_eq!(all.len(), 5);
        assert_span(&all[0].span, source, " ", 0);
        assert_span(&all[1].span, source, " ", 1);
        assert_span(&all[2].span, source, "struct", 2);
        assert_span(&all[3].span, source, " ", 8);
        assert_span(&all[4].span, source, " ", 9);
    }

    #[test]
    fn span_multiple_keywords() {
        let source = "struct enum fn";
        let ts = stream::TokenStream::lex(source).expect("lexing failed");
        let all = ts.all();

        // struct(0..6), space(6..7), enum(7..11), space(11..12), fn(12..14)
        assert_eq!(all.len(), 5);
        assert_span(&all[0].span, source, "struct", 0);
        assert_span(&all[1].span, source, " ", 6);
        assert_span(&all[2].span, source, "enum", 7);
        assert_span(&all[3].span, source, " ", 11);
        assert_span(&all[4].span, source, "fn", 12);
    }

    #[test]
    fn span_identifier() {
        let source = "foo_bar_123";
        let ts = stream::TokenStream::lex(source).expect("lexing failed");
        let all = ts.all();

        assert_eq!(all.len(), 1);
        assert_span(&all[0].span, source, "foo_bar_123", 0);
    }

    #[test]
    fn span_number() {
        let source = "12345";
        let ts = stream::TokenStream::lex(source).expect("lexing failed");
        let all = ts.all();

        assert_eq!(all.len(), 1);
        assert_span(&all[0].span, source, "12345", 0);
    }

    #[test]
    fn span_string_literal() {
        let source = r#""hello world""#;
        let ts = stream::TokenStream::lex(source).expect("lexing failed");
        let all = ts.all();

        assert_eq!(all.len(), 1);
        assert_span(&all[0].span, source, r#""hello world""#, 0);
    }

    #[test]
    fn span_multi_char_token() {
        let source = "->";
        let ts = stream::TokenStream::lex(source).expect("lexing failed");
        let all = ts.all();

        assert_eq!(all.len(), 1);
        assert_span(&all[0].span, source, "->", 0);
    }

    #[test]
    fn span_delimiters() {
        let source = "{}()";
        let ts = stream::TokenStream::lex(source).expect("lexing failed");
        let all = ts.all();

        assert_eq!(all.len(), 4);
        assert_span(&all[0].span, source, "{", 0);
        assert_span(&all[1].span, source, "}", 1);
        assert_span(&all[2].span, source, "(", 2);
        assert_span(&all[3].span, source, ")", 3);
    }

    #[test]
    fn span_complex_source() {
        let source = "struct Foo { x: i32, y: i64 }";
        let ts = stream::TokenStream::lex(source).expect("lexing failed");
        let all = ts.all();

        // struct(0..6) sp(6..7) Foo(7..10) sp(10..11) {(11..12) sp(12..13)
        // x(13..14) :(14..15) sp(15..16) i32(16..19) ,(19..20) sp(20..21)
        // y(21..22) :(22..23) sp(23..24) i64(24..27) sp(27..28) }(28..29)
        assert_span(&all[0].span, source, "struct", 0);
        assert_span(&all[2].span, source, "Foo", 7);
        assert_span(&all[4].span, source, "{", 11);
        assert_span(&all[6].span, source, "x", 13);
        assert_span(&all[7].span, source, ":", 14);
        assert_span(&all[9].span, source, "i32", 16);
        assert_span(&all[10].span, source, ",", 19);
    }

    #[test]
    fn span_multiline() {
        let source = "struct\nFoo\n{\n}";
        let ts = stream::TokenStream::lex(source).expect("lexing failed");
        let all = ts.all();

        // struct(0..6) \n(6..7) Foo(7..10) \n(10..11) {(11..12) \n(12..13) }(13..14)
        assert_span(&all[0].span, source, "struct", 0);
        assert_span(&all[1].span, source, "\n", 6);
        assert_span(&all[2].span, source, "Foo", 7);
        assert_span(&all[3].span, source, "\n", 10);
        assert_span(&all[4].span, source, "{", 11);
        assert_span(&all[5].span, source, "\n", 12);
        assert_span(&all[6].span, source, "}", 13);
    }
}

mod span_arithmetic_tests {
    use super::*;

    #[test]
    fn span_join_adjacent() {
        let source = "struct Foo";
        let ts = stream::TokenStream::lex(source).expect("lexing failed");
        let all = ts.all();

        // Tokens: struct, space, Foo
        let struct_span = &all[0].span;
        let foo_span = &all[2].span;

        let joined = struct_span.join(foo_span);
        assert_eq!(joined.start(), 0);
        assert_eq!(joined.end(), 10);
        assert_eq!(&source[joined.start()..joined.end()], "struct Foo");
    }

    #[test]
    fn span_join_same() {
        let span = span::Span::new(5, 10);
        let joined = span.join(&span);
        assert_eq!(joined.start(), 5);
        assert_eq!(joined.end(), 10);
    }

    #[test]
    fn span_join_overlapping() {
        let a = span::Span::new(0, 10);
        let b = span::Span::new(5, 15);
        let joined = a.join(&b);
        assert_eq!(joined.start(), 0);
        assert_eq!(joined.end(), 15);
    }

    #[test]
    fn span_join_non_adjacent() {
        let a = span::Span::new(0, 5);
        let b = span::Span::new(10, 15);
        let joined = a.join(&b);
        assert_eq!(joined.start(), 0);
        assert_eq!(joined.end(), 15);
    }

    #[test]
    fn span_empty() {
        let span = span::Span::new(5, 5);
        assert!(span.is_empty());
        assert_eq!(span.len(), 0);
    }

    #[test]
    fn span_call_site() {
        let span = span::Span::call_site();
        assert!(span.is_empty());
        assert_eq!(span.start(), 0);
        assert_eq!(span.end(), 0);
    }
}

mod stream_span_tests {
    use super::*;

    #[test]
    fn cursor_span_at_start() {
        let source = "struct Foo";
        let ts = stream::TokenStream::lex(source).expect("lexing failed");

        let span = ts.cursor_span().expect("should have cursor span");
        assert_eq!(span.start(), 0);
        assert_eq!(span.end(), 6);
    }

    #[test]
    fn cursor_span_after_consume() {
        let source = "struct Foo";
        let mut ts = stream::TokenStream::lex(source).expect("lexing failed");

        // Consume struct keyword (ts.next() skips whitespace)
        let _ = ts.next();

        // cursor_span returns the raw cursor position, which is after struct
        // but before the skip logic finds Foo
        let span = ts.cursor_span().expect("should have cursor span");
        // Raw cursor is at position 1 (the space token at 6..7)
        assert_eq!(span.start(), 6);
        assert_eq!(span.end(), 7);
    }

    #[test]
    fn last_span_after_consume() {
        let source = "struct Foo";
        let mut ts = stream::TokenStream::lex(source).expect("lexing failed");

        // Consume struct keyword (skips whitespace)
        let _ = ts.next();

        // last_span should be the struct keyword we just consumed
        let span = ts.last_span().expect("should have last span");
        assert_eq!(span.start(), 0);
        assert_eq!(span.end(), 6);
    }

    #[test]
    fn span_at_specific_position() {
        let source = "struct Foo { }";
        let ts = stream::TokenStream::lex(source).expect("lexing failed");

        // Position 0 is "struct"
        let span = ts.span_at(0).expect("should have span at 0");
        assert_eq!(&source[span.start()..span.end()], "struct");

        // Position 2 is "Foo" (skipping whitespace token at 1)
        let span = ts.span_at(2).expect("should have span at 2");
        assert_eq!(&source[span.start()..span.end()], "Foo");
    }

    #[test]
    fn span_range() {
        let source = "struct";
        let ts = stream::TokenStream::lex(source).expect("lexing failed");

        let span = ts.span_range(0..6);
        assert_eq!(span.start(), 0);
        assert_eq!(span.end(), 6);
    }
}

mod spanned_value_tests {
    use super::*;

    #[test]
    fn parsed_value_spans() {
        // "struct Point { x: i32, y: i64 }"
        //  0123456789012345678901234567890
        //            1111111111222222222233
        let source = "struct Point { x: i32, y: i64 }";
        let mut ts = stream::TokenStream::lex(source).expect("lexing failed");

        // Parse struct keyword - exact span verification
        let kw: span::Spanned<tokens::KwStructToken> = ts.parse().expect("parse kw");
        assert_eq!(kw.span().start(), 0);
        assert_eq!(kw.span().end(), 6);
        assert_eq!(&source[kw.span().start()..kw.span().end()], "struct");

        // Parse name - exact span verification (space at 6, "Point" at 7..12)
        let name: span::Spanned<tokens::IdentToken> = ts.parse().expect("parse name");
        assert_eq!(name.span().start(), 7);
        assert_eq!(name.span().end(), 12);
        assert_eq!(&source[name.span().start()..name.span().end()], "Point");
        assert_eq!(&name.value.0, "Point");

        // Parse opening brace (space at 12, "{" at 13)
        let brace: span::Spanned<tokens::LBraceToken> = ts.parse().expect("parse {");
        assert_eq!(&source[brace.span().start()..brace.span().end()], "{");

        // Parse first field name (space at 14, "x" at 15)
        let field_name: span::Spanned<tokens::IdentToken> = ts.parse().expect("parse field");
        assert_eq!(&field_name.value.0, "x");
        assert_eq!(
            &source[field_name.span().start()..field_name.span().end()],
            "x"
        );

        // Parse colon (immediately after x)
        let colon: span::Spanned<tokens::ColonToken> = ts.parse().expect("parse :");
        assert_eq!(&source[colon.span().start()..colon.span().end()], ":");

        // Parse type (space after colon, then "i32")
        let ty: span::Spanned<tokens::IdentToken> = ts.parse().expect("parse type");
        assert_eq!(&ty.value.0, "i32");
        assert_eq!(&source[ty.span().start()..ty.span().end()], "i32");
    }

    #[test]
    fn parsed_number_span() {
        let source = "12345";
        let mut ts = stream::TokenStream::lex(source).expect("lexing failed");

        let num: span::Spanned<tokens::NumberToken> = ts.parse().expect("parse number");
        assert_eq!(num.value.0, 12345);
        assert_eq!(num.span().start(), 0);
        assert_eq!(num.span().end(), 5);
    }

    #[test]
    fn parsed_string_span() {
        let source = r#""hello world""#;
        let mut ts = stream::TokenStream::lex(source).expect("lexing failed");

        let s: span::Spanned<tokens::StringToken> = ts.parse().expect("parse string");
        assert_eq!(&s.value.0, "hello world");
        assert_eq!(s.span().start(), 0);
        assert_eq!(s.span().end(), 13);
    }

    #[test]
    fn parsed_arrow_span() {
        // "fn foo() -> i32"
        //  012345678901234
        //            11111
        let source = "fn foo() -> i32";
        let mut ts = stream::TokenStream::lex(source).expect("lexing failed");

        // Skip to arrow
        let _: span::Spanned<tokens::KwFnToken> = ts.parse().expect("fn");
        let _: span::Spanned<tokens::IdentToken> = ts.parse().expect("foo");
        let _: span::Spanned<tokens::LParenToken> = ts.parse().expect("(");
        let _: span::Spanned<tokens::RParenToken> = ts.parse().expect(")");

        let arrow: span::Spanned<tokens::ArrowToken> = ts.parse().expect("parse arrow");
        assert_eq!(&source[arrow.span().start()..arrow.span().end()], "->");
    }
}

mod fork_rewind_span_tests {
    use super::*;

    #[test]
    fn fork_preserves_spans() {
        let source = "struct Foo";
        let mut ts = stream::TokenStream::lex(source).expect("lexing failed");

        let forked = ts.fork();

        // Original and fork should have same cursor span
        assert_eq!(ts.cursor_span(), forked.cursor_span());

        // Consume from original
        let _ = ts.next();

        // Original cursor moved, fork unchanged
        assert_ne!(ts.cursor_span(), forked.cursor_span());
    }

    #[test]
    fn rewind_restores_span() {
        let source = "struct Foo { }";
        let mut ts = stream::TokenStream::lex(source).expect("lexing failed");

        let initial_pos = ts.cursor();
        let initial_span = ts.cursor_span();

        // Consume some tokens
        let _ = ts.next();
        let _ = ts.next();

        assert_ne!(ts.cursor_span(), initial_span);

        // Rewind
        ts.rewind(initial_pos);

        assert_eq!(ts.cursor_span(), initial_span);
    }
}
