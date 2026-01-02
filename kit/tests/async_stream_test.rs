//! Tests for async streaming parser support.
//!
//! These tests verify the incremental lexing and parsing infrastructure
//! works correctly with both tokio and futures runtimes.

use synkit::async_stream::{
    IncrementalLexer, IncrementalParse, ParseCheckpoint, ParseState, StreamConfig, StreamError,
};

#[derive(Debug, Clone, PartialEq)]
pub enum MockToken {
    Number(i64),
    Plus,
    Minus,
    Eof,
}

impl AsRef<MockToken> for MockToken {
    fn as_ref(&self) -> &MockToken {
        self
    }
}

pub struct MockLexer {
    buffer: String,
    offset: usize,
}

impl IncrementalLexer for MockLexer {
    type Token = MockToken;
    type Span = (usize, usize);
    type Spanned = MockToken; // For simplicity, tokens are their own spanned type
    type Error = String;

    fn new() -> Self {
        Self {
            buffer: String::new(),
            offset: 0,
        }
    }

    fn feed(&mut self, chunk: &str) -> Result<Vec<Self::Spanned>, Self::Error> {
        self.buffer.push_str(chunk);
        let mut tokens = Vec::new();

        let bytes = self.buffer.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            let c = bytes[i];
            match c {
                b' ' | b'\t' | b'\n' => {
                    i += 1;
                }
                b'+' => {
                    tokens.push(MockToken::Plus);
                    i += 1;
                }
                b'-' => {
                    tokens.push(MockToken::Minus);
                    i += 1;
                }
                b'0'..=b'9' => {
                    let start = i;
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                    let s = &self.buffer[start..i];
                    let n: i64 = s.parse().map_err(|e| format!("invalid number: {}", e))?;
                    tokens.push(MockToken::Number(n));
                }
                _ => {
                    return Err(format!("unexpected character: {}", c as char));
                }
            }
        }

        self.offset += self.buffer.len();
        self.buffer.clear();

        Ok(tokens)
    }

    fn finish(self) -> Result<Vec<Self::Spanned>, Self::Error> {
        if !self.buffer.is_empty() {
            Err("incomplete token at end of input".to_string())
        } else {
            Ok(vec![])
        }
    }

    fn offset(&self) -> usize {
        self.offset
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub value: i64,
}

impl IncrementalParse for Expr {
    type Token = MockToken;
    type Error = String;

    fn parse_incremental<S>(
        tokens: &[S],
        checkpoint: &ParseCheckpoint,
    ) -> Result<(Option<Self>, ParseCheckpoint), Self::Error>
    where
        S: AsRef<Self::Token>,
    {
        let mut cursor = checkpoint.cursor;

        if cursor >= tokens.len() {
            return Ok((None, checkpoint.clone()));
        }

        // Simple parser: expect a single number
        match tokens[cursor].as_ref() {
            MockToken::Number(n) => {
                let node = Expr { value: *n };
                let new_checkpoint = ParseCheckpoint {
                    cursor: cursor + 1,
                    tokens_consumed: checkpoint.tokens_consumed + 1,
                    state: 0,
                };
                Ok((Some(node), new_checkpoint))
            }
            MockToken::Plus | MockToken::Minus => {
                // Skip operators, look for next number
                cursor += 1;
                if cursor >= tokens.len() {
                    return Ok((None, checkpoint.clone()));
                }
                match tokens[cursor].as_ref() {
                    MockToken::Number(n) => {
                        let node = Expr { value: *n };
                        let new_checkpoint = ParseCheckpoint {
                            cursor: cursor + 1,
                            tokens_consumed: checkpoint.tokens_consumed + 2,
                            state: 0,
                        };
                        Ok((Some(node), new_checkpoint))
                    }
                    _ => Err("expected number after operator".to_string()),
                }
            }
            _ => Err(format!("unexpected token: {:?}", tokens[cursor].as_ref())),
        }
    }

    fn can_parse<S>(tokens: &[S], checkpoint: &ParseCheckpoint) -> bool
    where
        S: AsRef<Self::Token>,
    {
        checkpoint.cursor < tokens.len()
    }
}

#[test]
fn test_parse_state_enum() {
    assert_eq!(ParseState::NeedMore, ParseState::NeedMore);
    assert_eq!(ParseState::Complete, ParseState::Complete);
    assert_eq!(ParseState::Error, ParseState::Error);
    assert_ne!(ParseState::NeedMore, ParseState::Complete);
}

#[test]
fn test_parse_checkpoint_default() {
    let cp = ParseCheckpoint::default();
    assert_eq!(cp.cursor, 0);
    assert_eq!(cp.tokens_consumed, 0);
    assert_eq!(cp.state, 0);
}

#[test]
fn test_stream_error_display() {
    let err = StreamError::ChannelClosed;
    assert_eq!(format!("{}", err), "channel closed unexpectedly");

    let err = StreamError::LexError("bad token".to_string());
    assert_eq!(format!("{}", err), "lex error: bad token");

    let err = StreamError::ParseError("syntax error".to_string());
    assert_eq!(format!("{}", err), "parse error: syntax error");

    let err = StreamError::IncompleteInput;
    assert_eq!(format!("{}", err), "incomplete input at end of stream");
}

#[test]
fn test_stream_config_default() {
    let config = StreamConfig::default();
    assert_eq!(config.token_buffer_size, 1024);
    assert_eq!(config.ast_buffer_size, 64);
    assert_eq!(config.max_chunk_size, 64 * 1024);
}

#[test]
fn test_mock_lexer_basic() {
    let mut lexer = MockLexer::new();

    let tokens = lexer.feed("42").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], MockToken::Number(42));

    let tokens = lexer.feed(" + ").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], MockToken::Plus);

    let tokens = lexer.feed("123").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], MockToken::Number(123));

    let remaining = lexer.finish().unwrap();
    assert!(remaining.is_empty());
}

#[test]
fn test_mock_lexer_multiple_tokens() {
    let mut lexer = MockLexer::new();
    let tokens = lexer.feed("1 + 2 - 3").unwrap();

    assert_eq!(tokens.len(), 5);
    assert_eq!(tokens[0], MockToken::Number(1));
    assert_eq!(tokens[1], MockToken::Plus);
    assert_eq!(tokens[2], MockToken::Number(2));
    assert_eq!(tokens[3], MockToken::Minus);
    assert_eq!(tokens[4], MockToken::Number(3));
}

#[test]
fn test_mock_lexer_error() {
    let mut lexer = MockLexer::new();
    let err = lexer.feed("42 @ 7").unwrap_err();
    assert!(err.contains("unexpected character"));
}

#[test]
fn test_incremental_parse_basic() {
    let tokens = vec![MockToken::Number(42)];
    let checkpoint = ParseCheckpoint::default();

    let (result, new_cp) = Expr::parse_incremental(&tokens, &checkpoint).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, 42);
    assert_eq!(new_cp.cursor, 1);
    assert_eq!(new_cp.tokens_consumed, 1);
}

#[test]
fn test_incremental_parse_needs_more() {
    let tokens: Vec<MockToken> = vec![];
    let checkpoint = ParseCheckpoint::default();

    let (result, _) = Expr::parse_incremental(&tokens, &checkpoint).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_incremental_parse_with_operator() {
    let tokens = vec![MockToken::Plus, MockToken::Number(7)];
    let checkpoint = ParseCheckpoint::default();

    let (result, new_cp) = Expr::parse_incremental(&tokens, &checkpoint).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, 7);
    assert_eq!(new_cp.cursor, 2);
    assert_eq!(new_cp.tokens_consumed, 2);
}

#[test]
fn test_can_parse() {
    let tokens = vec![MockToken::Number(1)];
    let checkpoint = ParseCheckpoint::default();

    assert!(Expr::can_parse(&tokens, &checkpoint));

    let empty: Vec<MockToken> = vec![];
    assert!(!Expr::can_parse(&empty, &checkpoint));

    let consumed_cp = ParseCheckpoint {
        cursor: 1,
        tokens_consumed: 1,
        state: 0,
    };
    assert!(!Expr::can_parse(&tokens, &consumed_cp));
}

#[cfg(feature = "tokio")]
mod tokio_tests {
    use super::*;
    use synkit::async_stream::tokio_impl::AstStream;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_async_token_stream_basic() {
        let (tx, mut rx) = mpsc::channel::<MockToken>(32);

        let mut lexer = MockLexer::new();
        let tokens = lexer.feed("42 + 7").unwrap();

        for token in tokens {
            tx.send(token).await.unwrap();
        }
        drop(tx);

        let mut received = Vec::new();
        while let Some(token) = rx.recv().await {
            received.push(token);
        }

        assert_eq!(received.len(), 3);
        assert_eq!(received[0], MockToken::Number(42));
        assert_eq!(received[1], MockToken::Plus);
        assert_eq!(received[2], MockToken::Number(7));
    }

    #[tokio::test]
    async fn test_async_token_stream_chunked() {
        let (tx, mut rx) = mpsc::channel::<MockToken>(32);

        tokio::spawn(async move {
            let mut lexer = MockLexer::new();

            for token in lexer.feed("10 +").unwrap() {
                tx.send(token).await.unwrap();
            }

            for token in lexer.feed(" 20").unwrap() {
                tx.send(token).await.unwrap();
            }

            for token in lexer.finish().unwrap() {
                tx.send(token).await.unwrap();
            }
        });

        let mut received = Vec::new();
        while let Some(token) = rx.recv().await {
            received.push(token);
        }

        assert_eq!(received.len(), 3);
        assert_eq!(received[0], MockToken::Number(10));
        assert_eq!(received[1], MockToken::Plus);
        assert_eq!(received[2], MockToken::Number(20));
    }

    #[tokio::test]
    async fn test_ast_stream_basic() {
        let (token_tx, token_rx) = mpsc::channel::<MockToken>(32);
        let (ast_tx, mut ast_rx) = mpsc::channel::<Expr>(16);

        tokio::spawn(async move {
            let mut parser = AstStream::<Expr, MockToken>::new(token_rx, ast_tx);
            parser.run().await.unwrap();
        });

        let mut lexer = MockLexer::new();
        for token in lexer.feed("42").unwrap() {
            token_tx.send(token).await.unwrap();
        }
        drop(token_tx);

        let expr = ast_rx.recv().await;
        assert!(expr.is_some());
        assert_eq!(expr.unwrap().value, 42);
    }

    #[tokio::test]
    async fn test_ast_stream_multiple_nodes() {
        let (token_tx, token_rx) = mpsc::channel::<MockToken>(32);
        let (ast_tx, mut ast_rx) = mpsc::channel::<Expr>(16);

        tokio::spawn(async move {
            let mut parser = AstStream::<Expr, MockToken>::new(token_rx, ast_tx);
            let _ = parser.run().await;
        });

        let mut lexer = MockLexer::new();
        for token in lexer.feed("1 + 2 - 3").unwrap() {
            token_tx.send(token).await.unwrap();
        }
        drop(token_tx);

        let mut nodes = Vec::new();
        while let Some(expr) = ast_rx.recv().await {
            nodes.push(expr);
        }

        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].value, 1);
        assert_eq!(nodes[1].value, 2);
        assert_eq!(nodes[2].value, 3);
    }

    #[tokio::test]
    async fn test_channel_closure_detected_by_parser() {
        let (token_tx, token_rx) = mpsc::channel::<MockToken>(32);
        let (ast_tx, ast_rx) = mpsc::channel::<Expr>(16);

        drop(ast_rx);

        let handle = tokio::spawn(async move {
            let mut parser = AstStream::<Expr, MockToken>::new(token_rx, ast_tx);
            parser.run().await
        });

        let mut lexer = MockLexer::new();
        for token in lexer.feed("42").unwrap() {
            let _ = token_tx.send(token).await;
        }
        drop(token_tx);

        let result = handle.await.unwrap();
        assert!(matches!(result, Err(StreamError::ChannelClosed)));
    }

    #[tokio::test]
    async fn test_backpressure_with_small_buffer() {
        let (token_tx, token_rx) = mpsc::channel::<MockToken>(2);
        let (ast_tx, mut ast_rx) = mpsc::channel::<Expr>(1);

        tokio::spawn(async move {
            let mut parser = AstStream::<Expr, MockToken>::new(token_rx, ast_tx);
            let _ = parser.run().await;
        });

        let mut lexer = MockLexer::new();
        let tokens = lexer.feed("1 2 3 4 5").unwrap();

        let send_handle = tokio::spawn(async move {
            for token in tokens {
                token_tx.send(token).await.unwrap();
            }
        });

        let mut count = 0;
        while ast_rx.recv().await.is_some() {
            count += 1;
        }

        send_handle.await.unwrap();
        assert_eq!(count, 5);
    }
}

#[cfg(feature = "futures")]
mod futures_tests {
    use super::*;
    use futures_core::Stream;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use synkit::async_stream::futures_impl::ParseStream;

    struct TokenIter {
        tokens: Vec<MockToken>,
        index: usize,
    }

    impl TokenIter {
        fn new(tokens: Vec<MockToken>) -> Self {
            Self { tokens, index: 0 }
        }
    }

    impl Stream for TokenIter {
        type Item = MockToken;

        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if self.index < self.tokens.len() {
                let token = self.tokens[self.index].clone();
                self.index += 1;
                Poll::Ready(Some(token))
            } else {
                Poll::Ready(None)
            }
        }
    }

    #[test]
    fn test_parse_stream_creation() {
        let tokens = vec![MockToken::Number(42)];
        let token_stream = TokenIter::new(tokens);
        let _parse_stream: ParseStream<_, Expr, _> = ParseStream::new(token_stream);
    }
}

#[cfg(all(feature = "tokio", feature = "futures"))]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_pipeline_source_lexer_parser() {
        use synkit::async_stream::tokio_impl::AstStream;
        use tokio::sync::mpsc;

        let (source_tx, mut source_rx) = mpsc::channel::<String>(8);
        let (token_tx, token_rx) = mpsc::channel::<MockToken>(32);
        let (ast_tx, mut ast_rx) = mpsc::channel::<Expr>(16);

        let lexer_handle = tokio::spawn(async move {
            let mut lexer = MockLexer::new();
            while let Some(chunk) = source_rx.recv().await {
                match lexer.feed(&chunk) {
                    Ok(tokens) => {
                        for token in tokens {
                            if token_tx.send(token).await.is_err() {
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Lex error: {}", e);
                        return;
                    }
                }
            }
            if let Ok(tokens) = lexer.finish() {
                for token in tokens {
                    let _ = token_tx.send(token).await;
                }
            }
        });

        let parser_handle = tokio::spawn(async move {
            let mut parser = AstStream::<Expr, MockToken>::new(token_rx, ast_tx);
            let _ = parser.run().await;
        });

        source_tx.send("10 + ".to_string()).await.unwrap();
        source_tx.send("20 - ".to_string()).await.unwrap();
        source_tx.send("30".to_string()).await.unwrap();
        drop(source_tx);

        let mut results = Vec::new();
        while let Some(expr) = ast_rx.recv().await {
            results.push(expr.value);
        }

        lexer_handle.await.unwrap();
        parser_handle.await.unwrap();

        assert_eq!(results, vec![10, 20, 30]);
    }

    #[tokio::test]
    async fn test_error_propagation_on_invalid_input() {
        use tokio::sync::mpsc;

        let (source_tx, mut source_rx) = mpsc::channel::<String>(8);
        let (token_tx, _token_rx) = mpsc::channel::<MockToken>(32);

        let lexer_handle = tokio::spawn(async move {
            let mut lexer = MockLexer::new();
            while let Some(chunk) = source_rx.recv().await {
                match lexer.feed(&chunk) {
                    Ok(tokens) => {
                        for token in tokens {
                            if token_tx.send(token).await.is_err() {
                                return Err("channel closed");
                            }
                        }
                    }
                    Err(e) => {
                        return Err(Box::leak(e.into_boxed_str()) as &str);
                    }
                }
            }
            Ok(())
        });

        source_tx.send("42 @ 7".to_string()).await.unwrap();
        drop(source_tx);

        let result = lexer_handle.await.unwrap();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unexpected character"));
    }
}
