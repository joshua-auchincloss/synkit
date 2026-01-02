//! Incremental Parsing for TOML
//!
//! This module implements the `IncrementalLexer`, `IncrementalParse`, and
//! `ChunkBoundary` traits for streaming TOML parsing.
//!
//! # Architecture
//!
//! The incremental parser uses the same parsing infrastructure as synchronous
//! parsing. The only difference is:
//! 1. `ChunkBoundary` defines where to split the token stream (newlines at depth 0)
//! 2. `IncrementalLexer` buffers partial lines until complete
//! 3. Actual parsing uses the standard `Parse` trait implementations
//!
//! This design ensures consistent parsing behavior and avoids duplicating logic.

use crate::{
    Parse, Span, Spanned, TokenStream, TomlError,
    ast::{Key, KeyValue, Trivia},
    tokens::{self, Token},
};
use synkit::async_stream::{
    ChunkBoundary, IncrementalBuffer, IncrementalLexer, IncrementalParse, LexerCapacityHint,
    ParseCheckpoint,
};

// ANCHOR: chunk_boundary
/// Implements `ChunkBoundary` for TOML document items.
///
/// In TOML, items are separated by newlines. A complete item is:
/// - A trivia line (newline or comment)
/// - A key = value pair ending with newline
/// - A table header [name] followed by optional content
///
/// Boundaries are newlines at nesting depth 0 (outside arrays/inline tables).
impl ChunkBoundary for IncrementalDocumentItem {
    type Token = Token;

    #[inline]
    fn is_boundary_token(token: &Token) -> bool {
        matches!(token, Token::Newline)
    }

    #[inline]
    fn depth_delta(token: &Token) -> i32 {
        match token {
            Token::LBracket | Token::LBrace => 1,
            Token::RBracket | Token::RBrace => -1,
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
/// Incremental lexer for TOML tokens.
///
/// Buffers partial input and produces tokens when complete lines are available.
/// Uses newlines as safe split points for TOML format.
pub struct TomlIncrementalLexer {
    /// Accumulated source text
    buffer: String,
    /// Current byte offset in overall source
    offset: usize,
    /// Pre-allocated token buffer capacity hint
    token_hint: usize,
}

impl TomlIncrementalLexer {
    fn lex_complete_lines(&mut self) -> Result<Vec<Spanned<Token>>, TomlError> {
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
            let token = result.map_err(|_| TomlError::Unknown)?;
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

impl IncrementalLexer for TomlIncrementalLexer {
    type Token = Token;
    type Span = Span;
    type Spanned = Spanned<Token>;
    type Error = TomlError;

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
            let token = result.map_err(|_| TomlError::Unknown)?;
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
            let token = result.map_err(|_| TomlError::Unknown)?;
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

/// A document item that can be parsed incrementally.
///
/// This is similar to `DocumentItem` but designed for streaming scenarios
/// where we emit items as soon as they're complete.
#[derive(Debug, Clone)]
pub enum IncrementalDocumentItem {
    /// A trivia item (newline or comment)
    Trivia(Trivia),
    /// A key-value pair
    KeyValue(Spanned<KeyValue>),
    /// A table header (the `[name]` part, items follow separately)
    TableHeader {
        lbracket: Spanned<tokens::LBracketToken>,
        name: Spanned<Key>,
        rbracket: Spanned<tokens::RBracketToken>,
    },
}

impl AsRef<Token> for Spanned<Token> {
    #[inline]
    fn as_ref(&self) -> &Token {
        &self.value
    }
}

// ANCHOR: incremental_parse
/// Implements `IncrementalParse` for `IncrementalDocumentItem`.
///
/// This implementation leverages `ChunkBoundary` for boundary detection and
/// uses the standard `Parse` trait for actual parsing, avoiding code duplication.
impl IncrementalParse for IncrementalDocumentItem {
    type Token = Token;
    type Error = TomlError;

    fn parse_incremental<S>(
        tokens: &[S],
        checkpoint: &ParseCheckpoint,
    ) -> Result<(Option<Self>, ParseCheckpoint), Self::Error>
    where
        S: AsRef<Self::Token>,
    {
        let start = checkpoint.cursor;

        if start >= tokens.len() {
            return Ok((None, checkpoint.clone()));
        }

        // Use ChunkBoundary to find the next complete chunk
        let remaining = &tokens[start..];
        let boundary = match Self::find_boundary(remaining, 0) {
            Some(b) => b,
            None => {
                // Check for EOF case: complete item without trailing newline
                if Self::is_complete_at_eof(remaining) {
                    remaining.len()
                } else {
                    return Ok((None, checkpoint.clone()));
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

        // Skip empty lines (just whitespace/newlines)
        let has_content = chunk
            .iter()
            .any(|t| !matches!(t.as_ref(), Token::Space | Token::Tab | Token::Newline));

        if !has_content {
            // Just a blank line - emit as trivia if there's a newline
            if boundary > 0
                && remaining
                    .get(boundary - 1)
                    .map(|t| matches!(t.as_ref(), Token::Newline))
                    .unwrap_or(false)
            {
                let span = Span::new(0, 0); // Placeholder span
                let new_cursor = start + boundary;
                return Ok((
                    Some(IncrementalDocumentItem::Trivia(Trivia::Newline(Spanned {
                        value: tokens::NewlineToken,
                        span,
                    }))),
                    ParseCheckpoint {
                        cursor: new_cursor,
                        tokens_consumed: new_cursor,
                        state: 0,
                    },
                ));
            }

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
        let item = Self::parse_chunk(chunk)?;

        let new_cursor = start + boundary;
        let new_checkpoint = ParseCheckpoint {
            cursor: new_cursor,
            tokens_consumed: new_cursor,
            state: 0,
        };

        Ok((Some(item), new_checkpoint))
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

impl IncrementalDocumentItem {
    /// Check if we have a complete item at EOF (no trailing newline needed).
    ///
    /// For TOML, we're conservative and only consider items complete at EOF if:
    /// - It's a comment (single token)
    /// - It's a table header `[name]` (matched brackets)
    /// - It's a key-value with a non-bracket value (key = value)
    ///
    /// We don't try to parse incomplete key-values that might span lines.
    fn is_complete_at_eof<S: AsRef<Token>>(tokens: &[S]) -> bool {
        let non_ws: Vec<_> = tokens
            .iter()
            .map(|t| t.as_ref())
            .filter(|t| !matches!(t, Token::Space | Token::Tab))
            .collect();

        if non_ws.is_empty() {
            return false;
        }

        // Comment is complete by itself
        if non_ws.len() == 1 && matches!(non_ws[0], Token::Comment) {
            return true;
        }

        // Table header: [name] - must have balanced brackets
        if matches!(non_ws.first(), Some(Token::LBracket)) {
            let mut depth = 0;
            for tok in &non_ws {
                match tok {
                    Token::LBracket => depth += 1,
                    Token::RBracket => depth -= 1,
                    _ => {}
                }
            }
            // Complete if balanced and ends with ]
            return depth == 0 && matches!(non_ws.last(), Some(Token::RBracket));
        }

        // Key-value pair: key = value
        // Must have: key, =, value (and value must not be incomplete bracket)
        if matches!(
            non_ws.first(),
            Some(Token::BareKey(_)) | Some(Token::BasicString(_))
        ) {
            // Need at least key = value (3 tokens minimum)
            if non_ws.len() < 3 {
                return false;
            }

            // Check for = after key
            if !matches!(non_ws.get(1), Some(Token::Eq)) {
                return false;
            }

            // Check that brackets are balanced (for arrays/inline tables)
            let mut depth = 0;
            for tok in non_ws.iter().skip(2) {
                match tok {
                    Token::LBracket | Token::LBrace => depth += 1,
                    Token::RBracket | Token::RBrace => depth -= 1,
                    _ => {}
                }
            }

            return depth == 0;
        }

        false
    }

    /// Parse a chunk of tokens into an IncrementalDocumentItem using standard Parse trait.
    fn parse_chunk<S: AsRef<Token>>(tokens: &[S]) -> Result<Self, TomlError> {
        // Build source from tokens for the standard lexer-based stream
        let source = tokens_to_source(tokens);
        let mut stream = TokenStream::lex(&source).map_err(|_| TomlError::Unknown)?;

        // Determine item type by first non-whitespace token
        let first_token = tokens
            .iter()
            .find(|t| !matches!(t.as_ref(), Token::Space | Token::Tab))
            .map(|t| t.as_ref());

        match first_token {
            Some(Token::Comment) => {
                // Comment trivia
                let trivia = Trivia::parse(&mut stream)?;
                Ok(IncrementalDocumentItem::Trivia(trivia))
            }
            Some(Token::Newline) => {
                // Newline trivia
                let trivia = Trivia::parse(&mut stream)?;
                Ok(IncrementalDocumentItem::Trivia(trivia))
            }
            Some(Token::LBracket) => {
                // Table header - parse just the header part [name]
                let lbracket: Spanned<tokens::LBracketToken> = stream.parse()?;
                let name: Spanned<Key> = stream.parse()?;
                let rbracket: Spanned<tokens::RBracketToken> = stream.parse()?;

                Ok(IncrementalDocumentItem::TableHeader {
                    lbracket,
                    name,
                    rbracket,
                })
            }
            Some(Token::BareKey(_)) | Some(Token::BasicString(_)) => {
                // Key-value pair
                let kv: Spanned<KeyValue> = stream.parse()?;
                Ok(IncrementalDocumentItem::KeyValue(kv))
            }
            Some(other) => Err(TomlError::Expected {
                expect: "key, table header, or trivia",
                found: format!("{:?}", other),
            }),
            None => Err(TomlError::Empty {
                expect: "document item",
            }),
        }
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
            Token::LBracket => source.push('['),
            Token::RBracket => source.push(']'),
            Token::LBrace => source.push('{'),
            Token::RBrace => source.push('}'),
            Token::Eq => source.push('='),
            Token::Dot => source.push('.'),
            Token::Comma => source.push(','),
            Token::True => source.push_str("true"),
            Token::False => source.push_str("false"),
            Token::Comment => source.push_str("# comment"), // Placeholder
            Token::BareKey(s) => source.push_str(s),
            Token::BasicString(s) => {
                source.push('"');
                source.push_str(s);
                source.push('"');
            }
            Token::Integer(n) => {
                use std::fmt::Write;
                write!(source, "{}", n).ok();
            }
        }
    }

    source
}
// ANCHOR_END: incremental_parse

/// Parse all available TOML items from a buffer, reusing allocations.
///
/// This is the recommended way to parse incrementally as it:
/// - Reuses the token buffer across chunks
/// - Uses `ChunkBoundary` for efficient boundary detection
/// - Delegates to standard `Parse` for actual parsing
///
/// # Example
/// ```ignore
/// let mut buffer = IncrementalBuffer::with_capacity(1024);
/// let mut lexer = TomlIncrementalLexer::with_capacity_hint(LexerCapacityHint::medium());
///
/// for chunk in chunks {
///     lexer.feed_into(chunk, buffer.remaining_mut())?;
///     let items = parse_buffered_items(&mut buffer)?;
///     for item in items {
///         process(item);
///     }
/// }
/// ```
pub fn parse_buffered_items(
    buffer: &mut IncrementalBuffer<Spanned<Token>>,
) -> Result<Vec<IncrementalDocumentItem>, TomlError> {
    let mut results = Vec::new();

    loop {
        let remaining = buffer.remaining();
        match IncrementalDocumentItem::find_boundary(remaining, 0) {
            Some(boundary) => {
                // Determine actual chunk end (exclude newline)
                let chunk_end = if boundary > 0
                    && remaining
                        .get(boundary - 1)
                        .map(|t| matches!(t.value, Token::Newline))
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
                    .any(|t| !matches!(t.value, Token::Space | Token::Tab | Token::Newline));

                if has_content {
                    let item = IncrementalDocumentItem::parse_chunk(chunk)?;
                    results.push(item);
                }

                buffer.consume(boundary);
            }
            None => break,
        }
    }

    // Compact periodically to release memory
    if buffer.consumed_pending() > 1000 {
        buffer.compact();
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_boundary_simple() {
        let tokens: Vec<Spanned<Token>> = vec![
            Spanned {
                value: Token::BareKey("key".into()),
                span: Span::new(0, 3),
            },
            Spanned {
                value: Token::Eq,
                span: Span::new(4, 5),
            },
            Spanned {
                value: Token::BasicString("value".into()),
                span: Span::new(6, 13),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(13, 14),
            },
        ];

        assert!(IncrementalDocumentItem::has_complete_chunk(&tokens, 0));
        assert_eq!(IncrementalDocumentItem::find_boundary(&tokens, 0), Some(4));
    }

    #[test]
    fn test_chunk_boundary_array() {
        let tokens: Vec<Spanned<Token>> = vec![
            Spanned {
                value: Token::BareKey("arr".into()),
                span: Span::new(0, 3),
            },
            Spanned {
                value: Token::Eq,
                span: Span::new(4, 5),
            },
            Spanned {
                value: Token::LBracket,
                span: Span::new(6, 7),
            },
            Spanned {
                value: Token::Integer(1),
                span: Span::new(7, 8),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(8, 9),
            }, // Inside array - NOT boundary
            Spanned {
                value: Token::Integer(2),
                span: Span::new(9, 10),
            },
            Spanned {
                value: Token::RBracket,
                span: Span::new(10, 11),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(11, 12),
            }, // At depth 0 - IS boundary
        ];

        assert_eq!(IncrementalDocumentItem::find_boundary(&tokens, 0), Some(8));
    }

    #[test]
    fn test_incremental_lexer_single_chunk() {
        let mut lexer = TomlIncrementalLexer::new();
        let tokens = lexer.feed("key = \"value\"\n").unwrap();
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_incremental_lexer_split_chunks() {
        let mut lexer = TomlIncrementalLexer::new();

        let tokens1 = lexer.feed("key = ").unwrap();
        assert!(tokens1.is_empty()); // No newline yet

        let tokens2 = lexer.feed("\"value\"\n").unwrap();
        assert!(!tokens2.is_empty());
    }

    #[test]
    fn test_incremental_parse_key_value() {
        let tokens: Vec<Spanned<Token>> = vec![
            Spanned {
                value: Token::BareKey("name".into()),
                span: Span::new(0, 4),
            },
            Spanned {
                value: Token::Eq,
                span: Span::new(5, 6),
            },
            Spanned {
                value: Token::BasicString("John".into()),
                span: Span::new(7, 13),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(13, 14),
            },
        ];

        let checkpoint = ParseCheckpoint::default();
        let (result, new_checkpoint) =
            IncrementalDocumentItem::parse_incremental(&tokens, &checkpoint).unwrap();

        assert!(result.is_some());
        assert!(matches!(
            result.unwrap(),
            IncrementalDocumentItem::KeyValue(_)
        ));
        assert_eq!(new_checkpoint.cursor, 4);
    }

    #[test]
    fn test_incremental_parse_table_header() {
        let tokens: Vec<Spanned<Token>> = vec![
            Spanned {
                value: Token::LBracket,
                span: Span::new(0, 1),
            },
            Spanned {
                value: Token::BareKey("server".into()),
                span: Span::new(1, 7),
            },
            Spanned {
                value: Token::RBracket,
                span: Span::new(7, 8),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(8, 9),
            },
        ];

        let checkpoint = ParseCheckpoint::default();
        let (result, new_checkpoint) =
            IncrementalDocumentItem::parse_incremental(&tokens, &checkpoint).unwrap();

        assert!(result.is_some());
        assert!(matches!(
            result.unwrap(),
            IncrementalDocumentItem::TableHeader { .. }
        ));
        assert_eq!(new_checkpoint.cursor, 4);
    }

    #[test]
    fn test_incremental_parse_needs_more() {
        // Incomplete key-value (missing value, no newline)
        let tokens: Vec<Spanned<Token>> = vec![
            Spanned {
                value: Token::BareKey("name".into()),
                span: Span::new(0, 4),
            },
            Spanned {
                value: Token::Eq,
                span: Span::new(5, 6),
            },
        ];

        let checkpoint = ParseCheckpoint::default();
        let (result, _) = IncrementalDocumentItem::parse_incremental(&tokens, &checkpoint).unwrap();

        // Should return None indicating more tokens needed
        assert!(result.is_none());
    }

    #[test]
    fn test_can_parse() {
        let tokens: Vec<Spanned<Token>> = vec![
            Spanned {
                value: Token::BareKey("key".into()),
                span: Span::new(0, 3),
            },
            Spanned {
                value: Token::Eq,
                span: Span::new(4, 5),
            },
            Spanned {
                value: Token::Integer(42),
                span: Span::new(6, 8),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(8, 9),
            },
        ];

        let checkpoint = ParseCheckpoint::default();
        assert!(IncrementalDocumentItem::can_parse(&tokens, &checkpoint));

        let consumed = ParseCheckpoint {
            cursor: 4,
            ..Default::default()
        };
        assert!(!IncrementalDocumentItem::can_parse(&tokens, &consumed));
    }

    #[test]
    fn test_parse_buffered_items() {
        let mut buffer = IncrementalBuffer::with_capacity(64);

        buffer.extend(vec![
            Spanned {
                value: Token::BareKey("name".into()),
                span: Span::new(0, 4),
            },
            Spanned {
                value: Token::Eq,
                span: Span::new(5, 6),
            },
            Spanned {
                value: Token::BasicString("test".into()),
                span: Span::new(7, 13),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(13, 14),
            },
            Spanned {
                value: Token::BareKey("port".into()),
                span: Span::new(14, 18),
            },
            Spanned {
                value: Token::Eq,
                span: Span::new(19, 20),
            },
            Spanned {
                value: Token::Integer(8080),
                span: Span::new(21, 25),
            },
            Spanned {
                value: Token::Newline,
                span: Span::new(25, 26),
            },
        ]);

        let items = parse_buffered_items(&mut buffer).unwrap();
        assert_eq!(items.len(), 2);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_buffer_reuse() {
        let mut buffer = IncrementalBuffer::with_capacity(64);
        let mut lexer = TomlIncrementalLexer::with_capacity_hint(LexerCapacityHint::small());

        // First batch
        lexer.feed_into("key = 42\n", buffer.tokens_mut()).unwrap();
        let items1 = parse_buffered_items(&mut buffer).unwrap();
        assert_eq!(items1.len(), 1);

        // Second batch reuses the buffer
        lexer
            .feed_into("port = 8080\n", buffer.tokens_mut())
            .unwrap();
        let items2 = parse_buffered_items(&mut buffer).unwrap();
        assert_eq!(items2.len(), 1);
    }

    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_streaming_toml_parse() {
        let (tx, mut rx) = mpsc::channel::<Spanned<Token>>(32);

        // Simulate streaming tokens
        tokio::spawn(async move {
            let mut lexer = TomlIncrementalLexer::new();

            // Chunk 1: start of document
            for token in lexer.feed("name = \"test\"\n").unwrap() {
                tx.send(token).await.unwrap();
            }

            // Chunk 2: table header
            for token in lexer.feed("[server]\n").unwrap() {
                tx.send(token).await.unwrap();
            }

            // Chunk 3: table content
            for token in lexer.feed("port = 8080\n").unwrap() {
                tx.send(token).await.unwrap();
            }

            // Finish
            for token in lexer.finish().unwrap() {
                tx.send(token).await.unwrap();
            }
        });

        // Collect tokens
        let mut tokens = Vec::new();
        while let Some(token) = rx.recv().await {
            tokens.push(token);
        }

        // Verify we got meaningful tokens
        assert!(!tokens.is_empty());

        // Parse incrementally
        let mut checkpoint = ParseCheckpoint::default();
        let mut items = Vec::new();

        loop {
            match IncrementalDocumentItem::parse_incremental(&tokens, &checkpoint) {
                Ok((Some(item), new_cp)) => {
                    items.push(item);
                    checkpoint = new_cp;
                }
                Ok((None, _)) => break,
                Err(_) => {
                    // Skip unparsable tokens in this simple test
                    checkpoint.cursor += 1;
                    checkpoint.tokens_consumed += 1;
                    if checkpoint.cursor >= tokens.len() {
                        break;
                    }
                }
            }
        }

        // Should have parsed some items
        assert!(!items.is_empty());
    }
}
