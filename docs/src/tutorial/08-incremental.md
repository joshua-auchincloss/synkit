# Incremental Parsing

This chapter demonstrates how to add incremental parsing support to the TOML parser for streaming scenarios.

## Overview

Incremental parsing allows processing TOML data as it arrives in chunks, useful for:

- Parsing large configuration files without loading entirely into memory
- Processing TOML streams from network connections
- Real-time parsing in editors

## Implementing IncrementalLexer

First, wrap the logos lexer with incremental capabilities:

```rust,ignore
use synkit::async_stream::IncrementalLexer;

pub struct TomlIncrementalLexer {
    buffer: String,
    offset: usize,
    pending_tokens: Vec<Spanned<Token>>,
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
            pending_tokens: Vec::new(),
        }
    }

    fn feed(&mut self, chunk: &str) -> Result<Vec<Self::Spanned>, Self::Error> {
        use logos::Logos;

        self.buffer.push_str(chunk);
        let mut tokens = Vec::new();
        let mut lexer = Token::lexer(&self.buffer);

        while let Some(result) = lexer.next() {
            let span = lexer.span();
            let token = result.map_err(|_| TomlError::Unknown)?;
            tokens.push(Spanned {
                value: token,
                span: Span::new(self.offset + span.start, self.offset + span.end),
            });
        }

        // Handle chunk boundaries - hold back potentially incomplete tokens
        let emit_count = if self.buffer.ends_with('\n') {
            tokens.len()
        } else {
            tokens.len().saturating_sub(1)
        };

        let to_emit: Vec<_> = tokens.drain(..emit_count).collect();
        self.pending_tokens = tokens;

        if let Some(last) = to_emit.last() {
            let consumed = last.span.end() - self.offset;
            self.buffer.drain(..consumed);
            self.offset = last.span.end();
        }

        Ok(to_emit)
    }

    fn finish(mut self) -> Result<Vec<Self::Spanned>, Self::Error> {
        // Process remaining buffer
        if !self.buffer.is_empty() {
            use logos::Logos;
            let mut lexer = Token::lexer(&self.buffer);
            while let Some(result) = lexer.next() {
                let span = lexer.span();
                let token = result.map_err(|_| TomlError::Unknown)?;
                self.pending_tokens.push(Spanned {
                    value: token,
                    span: Span::new(self.offset + span.start, self.offset + span.end),
                });
            }
        }
        Ok(self.pending_tokens)
    }

    fn offset(&self) -> usize {
        self.offset
    }
}
```

## Implementing IncrementalParse

Define an incremental document item that emits as soon as parseable:

```rust,ignore
use synkit::async_stream::{IncrementalParse, ParseCheckpoint};

#[derive(Debug, Clone)]
pub enum IncrementalDocumentItem {
    Trivia(Trivia),
    KeyValue(Spanned<KeyValue>),
    TableHeader {
        lbracket: Spanned<tokens::LBracketToken>,
        name: Spanned<Key>,
        rbracket: Spanned<tokens::RBracketToken>,
    },
}

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
        let cursor = checkpoint.cursor;

        if cursor >= tokens.len() {
            return Ok((None, checkpoint.clone()));
        }

        let token = tokens[cursor].as_ref();

        match token {
            // Newline trivia - emit immediately
            Token::Newline => {
                let item = IncrementalDocumentItem::Trivia(/* ... */);
                let new_cp = ParseCheckpoint {
                    cursor: cursor + 1,
                    tokens_consumed: checkpoint.tokens_consumed + 1,
                    state: 0,
                };
                Ok((Some(item), new_cp))
            }

            // Table header: need [, name, ]
            Token::LBracket => {
                if cursor + 2 >= tokens.len() {
                    // Need more tokens
                    return Ok((None, checkpoint.clone()));
                }
                // Parse [name] and emit TableHeader
                // ...
            }

            // Key-value: need key, =, value
            Token::BareKey(_) | Token::BasicString(_) => {
                if cursor + 2 >= tokens.len() {
                    return Ok((None, checkpoint.clone()));
                }
                // Parse key = value and emit KeyValue
                // ...
            }

            // Skip whitespace
            Token::Space | Token::Tab => {
                let new_cp = ParseCheckpoint {
                    cursor: cursor + 1,
                    tokens_consumed: checkpoint.tokens_consumed + 1,
                    state: checkpoint.state,
                };
                Self::parse_incremental(tokens, &new_cp)
            }

            _ => Err(TomlError::Expected {
                expect: "key, table header, or trivia",
                found: format!("{:?}", token),
            }),
        }
    }

    fn can_parse<S>(tokens: &[S], checkpoint: &ParseCheckpoint) -> bool
    where
        S: AsRef<Self::Token>,
    {
        checkpoint.cursor < tokens.len()
    }
}
```

## Using with Tokio

Stream TOML parsing with tokio channels:

```rust,ignore
use synkit::async_stream::tokio_impl::AstStream;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (source_tx, mut source_rx) = mpsc::channel::<String>(8);
    let (token_tx, token_rx) = mpsc::channel(32);
    let (ast_tx, mut ast_rx) = mpsc::channel(16);

    // Lexer task
    tokio::spawn(async move {
        let mut lexer = TomlIncrementalLexer::new();
        while let Some(chunk) = source_rx.recv().await {
            for token in lexer.feed(&chunk).unwrap() {
                token_tx.send(token).await.unwrap();
            }
        }
        for token in lexer.finish().unwrap() {
            token_tx.send(token).await.unwrap();
        }
    });

    // Parser task
    tokio::spawn(async move {
        let mut parser = AstStream::<IncrementalDocumentItem, Spanned<Token>>::new(
            token_rx,
            ast_tx
        );
        parser.run().await.unwrap();
    });

    // Feed source chunks
    source_tx.send("[server]\n".to_string()).await.unwrap();
    source_tx.send("host = \"localhost\"\n".to_string()).await.unwrap();
    source_tx.send("port = 8080\n".to_string()).await.unwrap();
    drop(source_tx);

    // Process items as they arrive
    while let Some(item) = ast_rx.recv().await {
        match item {
            IncrementalDocumentItem::TableHeader { name, .. } => {
                println!("Found table: {:?}", name);
            }
            IncrementalDocumentItem::KeyValue(kv) => {
                println!("Found key-value: {:?}", kv.value.key);
            }
            IncrementalDocumentItem::Trivia(_) => {}
        }
    }
}
```

## Testing Incremental Parsing

Test with various chunk boundaries:

```rust,ignore
#[test]
fn test_incremental_lexer_chunked() {
    let mut lexer = TomlIncrementalLexer::new();

    // Split across chunk boundary
    let t1 = lexer.feed("ke").unwrap();
    let t2 = lexer.feed("y = ").unwrap();
    let t3 = lexer.feed("42\n").unwrap();

    let remaining = lexer.finish().unwrap();
    let total = t1.len() + t2.len() + t3.len() + remaining.len();

    // Should produce: key, =, 42, newline
    assert!(total >= 4);
}

#[test]
fn test_incremental_parse_needs_more() {
    let tokens = vec![
        Spanned { value: Token::BareKey("name".into()), span: Span::new(0, 4) },
        Spanned { value: Token::Eq, span: Span::new(5, 6) },
        // Missing value!
    ];

    let checkpoint = ParseCheckpoint::default();
    let (result, _) = IncrementalDocumentItem::parse_incremental(&tokens, &checkpoint).unwrap();

    // Should return None, not error
    assert!(result.is_none());
}
```

## Summary

Key points for incremental parsing:

1. **Buffer management**: Hold back tokens at chunk boundaries that might be incomplete
2. **Return `None` for incomplete**: Don't error when more tokens are needed
3. **Track offset**: Maintain byte offset across chunks for correct spans
4. **Emit early**: Emit AST nodes as soon as they're complete
5. **Test boundaries**: Test parsing with data split at various points
