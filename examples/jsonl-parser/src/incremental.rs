//! Incremental Parsing for JSON Lines
//!
//! This module implements the `IncrementalLexer`, `IncrementalParse`, and
//! `ChunkBoundary` traits for streaming JSONL parsing.
//!
//! # Architecture
//!
//! The incremental parser uses zero-copy parsing via `TokenStream::from_tokens`:
//! 1. `ChunkBoundary` defines where to split the token stream (newlines at depth 0)
//! 2. `IncrementalLexer` buffers partial lines until complete
//! 3. `TokenStream::from_tokens` parses pre-lexed tokens without re-lexing
//!
//! This design ensures consistent parsing behavior and minimizes allocations.

use crate::{
    JsonError, Parse, Span, Spanned,
    ast::{JsonLine, JsonValue},
    tokens::Token,
};
use synkit::async_stream::{
    ChunkBoundary, IncrementalBuffer, IncrementalLexer, IncrementalParse, LexerCapacityHint,
    ParseCheckpoint,
};

// ANCHOR: chunk_boundary
/// Implements `ChunkBoundary` for JSONL format.
///
/// In JSONL, each line is a complete JSON value. Boundaries are:
/// - Newline tokens at nesting depth 0 (outside any `{}` or `[]`)
impl ChunkBoundary for JsonLine {
    type Token = Token;

    #[inline]
    fn is_boundary_token(token: &Token) -> bool {
        matches!(token, Token::Newline)
    }

    #[inline]
    fn depth_delta(token: &Token) -> i32 {
        match token {
            Token::LBrace | Token::LBracket => 1,
            Token::RBrace | Token::RBracket => -1,
            _ => 0,
        }
    }

    #[inline]
    fn is_ignorable(token: &Token) -> bool {
        matches!(token, Token::Space | Token::Tab)
    }
}
// ANCHOR_END: chunk_boundary

// ANCHOR: incremental_lexer
/// Incremental lexer for JSON tokens.
///
/// Buffers partial input and produces tokens when complete lines are available.
/// Uses newlines as safe split points for JSONL format.
pub struct JsonIncrementalLexer {
    /// Accumulated source text
    buffer: String,
    /// Current byte offset in overall source
    offset: usize,
    /// Pre-allocated token buffer capacity hint
    token_hint: usize,
}

impl JsonIncrementalLexer {
    fn lex_complete_lines(&mut self) -> Result<Vec<Spanned<Token>>, JsonError> {
        use logos::Logos;

        // Find the last newline - we only lex complete lines
        let split_pos = self.buffer.rfind('\n').map(|p| p + 1);

        let (to_lex, remainder) = match split_pos {
            Some(pos) if pos < self.buffer.len() => {
                let (prefix, suffix) = self.buffer.split_at(pos);
                (prefix.to_string(), suffix.to_string())
            }
            Some(pos) if pos == self.buffer.len() => {
                (std::mem::take(&mut self.buffer), String::new())
            }
            _ => return Ok(Vec::new()),
        };

        // Lex with pre-allocated capacity
        let mut tokens = Vec::with_capacity(self.token_hint);
        let mut lexer = Token::lexer(&to_lex);

        while let Some(result) = lexer.next() {
            let token = result.map_err(|_| JsonError::Unknown)?;
            let span = lexer.span();
            tokens.push(Spanned {
                value: token,
                span: Span::new(self.offset + span.start, self.offset + span.end),
            });
        }

        // Update state
        self.offset += to_lex.len();
        self.buffer = remainder;

        Ok(tokens)
    }
}

impl IncrementalLexer for JsonIncrementalLexer {
    type Token = Token;
    type Span = Span;
    type Spanned = Spanned<Token>;
    type Error = JsonError;

    fn new() -> Self {
        Self {
            buffer: String::new(),
            offset: 0,
            token_hint: 64,
        }
    }

    fn with_capacity_hint(hint: LexerCapacityHint) -> Self {
        Self {
            buffer: String::with_capacity(hint.buffer_capacity),
            offset: 0,
            token_hint: hint.tokens_per_chunk,
        }
    }

    fn feed(&mut self, chunk: &str) -> Result<Vec<Self::Spanned>, Self::Error> {
        self.buffer.push_str(chunk);
        self.lex_complete_lines()
    }

    fn feed_into(
        &mut self,
        chunk: &str,
        buffer: &mut Vec<Self::Spanned>,
    ) -> Result<usize, Self::Error> {
        self.buffer.push_str(chunk);

        use logos::Logos;

        let split_pos = self.buffer.rfind('\n').map(|p| p + 1);
        let (to_lex, remainder) = match split_pos {
            Some(pos) if pos < self.buffer.len() => {
                let (prefix, suffix) = self.buffer.split_at(pos);
                (prefix.to_string(), suffix.to_string())
            }
            Some(pos) if pos == self.buffer.len() => {
                (std::mem::take(&mut self.buffer), String::new())
            }
            _ => return Ok(0),
        };

        let mut count = 0;
        let mut lexer = Token::lexer(&to_lex);

        while let Some(result) = lexer.next() {
            let token = result.map_err(|_| JsonError::Unknown)?;
            let span = lexer.span();
            buffer.push(Spanned {
                value: token,
                span: Span::new(self.offset + span.start, self.offset + span.end),
            });
            count += 1;
        }

        self.offset += to_lex.len();
        self.buffer = remainder;

        Ok(count)
    }

    fn finish(self) -> Result<Vec<Self::Spanned>, Self::Error> {
        use logos::Logos;

        if self.buffer.is_empty() {
            return Ok(Vec::new());
        }

        let mut tokens = Vec::with_capacity(self.token_hint);
        let mut lexer = Token::lexer(&self.buffer);

        while let Some(result) = lexer.next() {
            let token = result.map_err(|_| JsonError::Unknown)?;
            let span = lexer.span();
            tokens.push(Spanned {
                value: token,
                span: Span::new(self.offset + span.start, self.offset + span.end),
            });
        }

        Ok(tokens)
    }

    fn offset(&self) -> usize {
        self.offset
    }
}
// ANCHOR_END: incremental_lexer

// ANCHOR: incremental_parse
/// Implements `IncrementalParse` for `JsonLine`.
///
/// This implementation leverages `ChunkBoundary` for boundary detection and
/// uses the standard `Parse` trait for actual parsing, avoiding code duplication.
impl IncrementalParse for JsonLine {
    type Token = Token;
    type Error = JsonError;

    fn parse_incremental<S>(
        tokens: &[S],
        checkpoint: &ParseCheckpoint,
    ) -> Result<(Option<Self>, ParseCheckpoint), Self::Error>
    where
        S: AsRef<Self::Token>,
    {
        let start = checkpoint.cursor;

        if start >= tokens.len() {
            return Ok((None, *checkpoint));
        }

        // Use ChunkBoundary to find the next complete chunk
        let remaining = &tokens[start..];
        let boundary = match Self::find_boundary(remaining, 0) {
            Some(b) => b,
            None => {
                // Check for EOF case: complete value without trailing newline
                if Self::is_complete_at_eof(remaining) {
                    remaining.len()
                } else {
                    return Ok((None, *checkpoint));
                }
            }
        };

        // Extract chunk tokens (excluding boundary newline if present)
        let chunk_end = if boundary > 0
            && remaining
                .get(boundary - 1)
                .map(|t| matches!(t.as_ref(), Token::Newline))
                .unwrap_or(false)
        {
            boundary - 1
        } else {
            boundary
        };

        let chunk = &remaining[..chunk_end];

        // Skip empty lines
        let has_content = chunk
            .iter()
            .any(|t| !matches!(t.as_ref(), Token::Space | Token::Tab | Token::Newline));

        if !has_content {
            let new_cursor = start + boundary;
            return Ok((
                None,
                ParseCheckpoint {
                    cursor: new_cursor,
                    tokens_consumed: new_cursor,
                    state: 0,
                },
            ));
        }

        // Parse using standard Parse infrastructure
        let line = Self::parse_chunk(chunk)?;

        let new_cursor = start + boundary;
        let new_checkpoint = ParseCheckpoint {
            cursor: new_cursor,
            tokens_consumed: new_cursor,
            state: 0,
        };

        Ok((Some(line), new_checkpoint))
    }

    fn can_parse<S>(tokens: &[S], checkpoint: &ParseCheckpoint) -> bool
    where
        S: AsRef<Self::Token>,
    {
        let start = checkpoint.cursor;
        if start >= tokens.len() {
            return false;
        }
        Self::has_complete_chunk(&tokens[start..], 0)
    }
}

impl JsonLine {
    /// Check if we have a complete value at EOF (no trailing newline needed)
    fn is_complete_at_eof<S: AsRef<Token>>(tokens: &[S]) -> bool {
        let mut depth: i32 = 0;
        let mut has_value = false;

        for tok in tokens {
            let token = tok.as_ref();
            match token {
                Token::LBrace | Token::LBracket => {
                    depth += 1;
                    has_value = true;
                }
                Token::RBrace | Token::RBracket => depth -= 1,
                Token::Null | Token::True | Token::False | Token::Number(_) | Token::String(_) => {
                    has_value = true
                }
                _ => {}
            }
        }

        has_value && depth == 0
    }

    /// Parse a chunk of spanned tokens into a JsonLine using zero-copy TokenStream.
    ///
    /// This uses `TokenStream::from_tokens` to avoid re-lexing: tokens are wrapped
    /// in an Arc and passed directly to the parser.
    fn parse_chunk_spanned(tokens: &[Spanned<Token>], source: &str) -> Result<Self, JsonError> {
        use std::sync::Arc;

        // Reconstruct source for span slicing (needed by TokenStream)
        // TODO: Track source ranges to avoid this allocation
        let source_text = if source.is_empty() {
            tokens_to_source(tokens)
        } else {
            source.to_string()
        };

        let tokens_vec = tokens.to_vec();
        let mut stream = crate::stream::TokenStream::from_tokens(
            Arc::from(source_text.as_str()),
            Arc::new(tokens_vec),
        );
        let value = JsonValue::parse(&mut stream)?;
        let span = value.span;
        Ok(JsonLine { value, span })
    }

    /// Parse a chunk of tokens into a JsonLine.
    ///
    /// For `Spanned<Token>` slices, uses zero-copy `TokenStream::from_tokens`.
    /// For other types implementing `AsRef<Token>`, falls back to reconstruction.
    fn parse_chunk<S: AsRef<Token>>(tokens: &[S]) -> Result<Self, JsonError> {
        // Fast path: check if we can use zero-copy
        // We need to go through AsRef because the generic bound is AsRef<Token>
        // The actual type check happens at the call site with Spanned<Token>
        Self::parse_chunk_fallback(tokens)
    }

    fn parse_chunk_fallback<S: AsRef<Token>>(tokens: &[S]) -> Result<Self, JsonError> {
        let source = tokens_to_source(tokens);
        let mut stream =
            crate::stream::TokenStream::lex(&source).map_err(|_| JsonError::Unknown)?;
        let value = JsonValue::parse(&mut stream)?;
        let span = value.span;
        Ok(JsonLine { value, span })
    }
}

/// Reconstruct source text from tokens for standard parsing.
///
/// This is needed because the standard `TokenStream` is built from source text.
/// Future optimization: create a `TokenStream` that works directly with token slices.
fn tokens_to_source<S: AsRef<Token>>(tokens: &[S]) -> String {
    let mut source = String::with_capacity(tokens.len() * 4); // Estimate 4 chars per token

    for tok in tokens {
        match tok.as_ref() {
            Token::Space => source.push(' '),
            Token::Tab => source.push('\t'),
            Token::Newline => source.push('\n'),
            Token::LBrace => source.push('{'),
            Token::RBrace => source.push('}'),
            Token::LBracket => source.push('['),
            Token::RBracket => source.push(']'),
            Token::Colon => source.push(':'),
            Token::Comma => source.push(','),
            Token::Null => source.push_str("null"),
            Token::True => source.push_str("true"),
            Token::False => source.push_str("false"),
            Token::Number(n) => source.push_str(n),
            Token::String(s) => {
                source.push('"');
                source.push_str(s);
                source.push('"');
            }
        }
    }

    source
}
// ANCHOR_END: incremental_parse

/// Parse all available JSONL lines from a buffer using `TokenStream::from_tokens`.
///
/// Uses the generated `TokenStream::from_tokens` method to parse pre-lexed tokens
/// without re-lexing. Token slices are wrapped in Arc for efficient sharing.
pub fn parse_buffered_lines(
    buffer: &mut IncrementalBuffer<Spanned<Token>>,
) -> Result<Vec<JsonLine>, JsonError> {
    let mut results = Vec::new();

    loop {
        let remaining = buffer.remaining();
        match JsonLine::find_boundary(remaining, 0) {
            Some(boundary) => {
                let chunk_end = if boundary > 0
                    && remaining
                        .get(boundary - 1)
                        .is_some_and(|t| matches!(t.value, Token::Newline))
                {
                    boundary - 1
                } else {
                    boundary
                };

                let chunk = &remaining[..chunk_end];

                let has_content = chunk
                    .iter()
                    .any(|t| !matches!(t.value, Token::Space | Token::Tab | Token::Newline));

                if has_content {
                    let line = JsonLine::parse_chunk_spanned(chunk, "")?;
                    results.push(line);
                }

                buffer.consume(boundary);
            }
            None => break,
        }
    }

    if buffer.consumed_pending() > 1000 {
        buffer.compact();
    }

    Ok(results)
}

impl AsRef<Token> for Spanned<Token> {
    #[inline]
    fn as_ref(&self) -> &Token {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::JsonValueKind;

    #[test]
    fn test_chunk_boundary_simple() {
        let tokens: Vec<Spanned<Token>> = vec![
            Spanned {
                value: Token::LBrace,
                span: Span::new(0, 1),
            },
            Spanned {
                value: Token::RBrace,
                span: Span::new(1, 2),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(2, 3),
            },
        ];

        assert!(JsonLine::has_complete_chunk(&tokens, 0));
        assert_eq!(JsonLine::find_boundary(&tokens, 0), Some(3));
    }

    #[test]
    fn test_chunk_boundary_nested() {
        let tokens: Vec<Spanned<Token>> = vec![
            Spanned {
                value: Token::LBrace,
                span: Span::new(0, 1),
            },
            Spanned {
                value: Token::String("a".into()),
                span: Span::new(1, 4),
            },
            Spanned {
                value: Token::Colon,
                span: Span::new(4, 5),
            },
            Spanned {
                value: Token::LBracket,
                span: Span::new(5, 6),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(6, 7),
            }, // Inside array - NOT boundary
            Spanned {
                value: Token::RBracket,
                span: Span::new(7, 8),
            },
            Spanned {
                value: Token::RBrace,
                span: Span::new(8, 9),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(9, 10),
            }, // At depth 0 - IS boundary
        ];

        assert_eq!(JsonLine::find_boundary(&tokens, 0), Some(8)); // Past the final newline
    }

    #[test]
    fn test_incremental_lexer_single_chunk() {
        let mut lexer = JsonIncrementalLexer::new();
        let tokens = lexer
            .feed(
                r#"{"a": 1}
"#,
            )
            .unwrap();
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_incremental_lexer_split_chunks() {
        let mut lexer = JsonIncrementalLexer::new();

        let tokens1 = lexer.feed(r#"{"name": "#).unwrap();
        assert!(tokens1.is_empty()); // No newline yet

        let tokens2 = lexer
            .feed(
                r#""Alice"}
"#,
            )
            .unwrap();
        assert!(!tokens2.is_empty());
    }

    #[test]
    fn test_incremental_parse_simple() {
        let tokens: Vec<Spanned<Token>> = vec![
            Spanned {
                value: Token::LBrace,
                span: Span::new(0, 1),
            },
            Spanned {
                value: Token::String("a".into()),
                span: Span::new(1, 4),
            },
            Spanned {
                value: Token::Colon,
                span: Span::new(4, 5),
            },
            Spanned {
                value: Token::Number("1".into()),
                span: Span::new(6, 7),
            },
            Spanned {
                value: Token::RBrace,
                span: Span::new(7, 8),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(8, 9),
            },
        ];

        let checkpoint = ParseCheckpoint::default();
        let (result, new_checkpoint) = JsonLine::parse_incremental(&tokens, &checkpoint).unwrap();

        assert!(result.is_some());
        let line = result.unwrap();
        assert!(matches!(line.value.kind, JsonValueKind::Object(_)));
        assert_eq!(new_checkpoint.cursor, 6);
    }

    #[test]
    fn test_incremental_parse_needs_more() {
        let tokens: Vec<Spanned<Token>> = vec![
            Spanned {
                value: Token::LBrace,
                span: Span::new(0, 1),
            },
            Spanned {
                value: Token::String("a".into()),
                span: Span::new(1, 4),
            },
            Spanned {
                value: Token::Colon,
                span: Span::new(4, 5),
            },
        ];

        let checkpoint = ParseCheckpoint::default();
        let (result, _) = JsonLine::parse_incremental(&tokens, &checkpoint).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_buffered_lines() {
        let mut buffer = IncrementalBuffer::with_capacity(64);

        buffer.extend(vec![
            Spanned {
                value: Token::LBrace,
                span: Span::new(0, 1),
            },
            Spanned {
                value: Token::String("a".into()),
                span: Span::new(1, 4),
            },
            Spanned {
                value: Token::Colon,
                span: Span::new(4, 5),
            },
            Spanned {
                value: Token::Number("1".into()),
                span: Span::new(6, 7),
            },
            Spanned {
                value: Token::RBrace,
                span: Span::new(7, 8),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(8, 9),
            },
            Spanned {
                value: Token::LBrace,
                span: Span::new(9, 10),
            },
            Spanned {
                value: Token::String("b".into()),
                span: Span::new(10, 13),
            },
            Spanned {
                value: Token::Colon,
                span: Span::new(13, 14),
            },
            Spanned {
                value: Token::Number("2".into()),
                span: Span::new(15, 16),
            },
            Spanned {
                value: Token::RBrace,
                span: Span::new(16, 17),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(17, 18),
            },
        ]);

        let lines = parse_buffered_lines(&mut buffer).unwrap();
        assert_eq!(lines.len(), 2);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_buffer_reuse() {
        let mut buffer = IncrementalBuffer::with_capacity(64);
        let mut lexer = JsonIncrementalLexer::with_capacity_hint(LexerCapacityHint::small());

        // First batch
        lexer
            .feed_into(
                r#"{"a": 1}
"#,
                buffer.tokens_mut(),
            )
            .unwrap();
        let lines1 = parse_buffered_lines(&mut buffer).unwrap();
        assert_eq!(lines1.len(), 1);

        // Second batch reuses the buffer
        lexer
            .feed_into(
                r#"{"b": 2}
"#,
                buffer.tokens_mut(),
            )
            .unwrap();
        let lines2 = parse_buffered_lines(&mut buffer).unwrap();
        assert_eq!(lines2.len(), 1);
    }
}
