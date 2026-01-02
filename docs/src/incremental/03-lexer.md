# Incremental Lexer

> **📦 Source**: [examples/jsonl-parser/src/incremental.rs](https://github.com/joshua-auchincloss/synkit/blob/main/examples/jsonl-parser/src/incremental.rs)

The `IncrementalLexer` trait enables lexing input that arrives in chunks.

## The Problem

Network data arrives in arbitrary chunks:

```text
Chunk 1: {"name": "ali
Chunk 2: ce"}\n{"name
Chunk 3: e": "bob"}\n
```

We need to:
1. Buffer incomplete tokens across chunks
2. Emit complete tokens as soon as available
3. Track source positions across all chunks

## IncrementalLexer Trait

```rust,ignore
pub trait IncrementalLexer: Sized {
    type Token: Clone;
    type Span: Clone;
    type Spanned: Clone;
    type Error: fmt::Display;

    /// Create with default capacity
    fn new() -> Self;

    /// Create with capacity hints for pre-allocation
    fn with_capacity_hint(hint: LexerCapacityHint) -> Self;

    /// Feed a chunk, return complete tokens
    fn feed(&mut self, chunk: &str) -> Result<Vec<Self::Spanned>, Self::Error>;

    /// Feed into existing buffer (avoids allocation)
    fn feed_into(
        &mut self,
        chunk: &str,
        buffer: &mut Vec<Self::Spanned>
    ) -> Result<usize, Self::Error>;

    /// Finish and return remaining tokens
    fn finish(self) -> Result<Vec<Self::Spanned>, Self::Error>;

    /// Current byte offset
    fn offset(&self) -> usize;
}
```

## JSONL Implementation

```rust,ignore
pub struct JsonIncrementalLexer {
    buffer: String,      // Accumulated input
    offset: usize,       // Total bytes processed
    token_hint: usize,   // Capacity hint
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

    fn finish(self) -> Result<Vec<Self::Spanned>, Self::Error> {
        if self.buffer.is_empty() {
            return Ok(Vec::new());
        }
        // Lex remaining buffer
        self.lex_buffer(&self.buffer)
    }

    fn offset(&self) -> usize {
        self.offset
    }
}
```

## Key Implementation: `lex_complete_lines`

```rust,ignore
fn lex_complete_lines(&mut self) -> Result<Vec<Spanned<Token>>, JsonError> {
    use logos::Logos;

    // Find last newline - only lex complete lines
    let split_pos = self.buffer.rfind('\n').map(|p| p + 1);

    let (to_lex, remainder) = match split_pos {
        Some(pos) if pos < self.buffer.len() => {
            // Have remainder after newline
            let (prefix, suffix) = self.buffer.split_at(pos);
            (prefix.to_string(), suffix.to_string())
        }
        Some(pos) if pos == self.buffer.len() => {
            // Newline at end, no remainder
            (std::mem::take(&mut self.buffer), String::new())
        }
        _ => return Ok(Vec::new()), // No complete lines yet
    };

    // Lex the complete portion
    let mut tokens = Vec::with_capacity(self.token_hint);
    let mut lexer = Token::lexer(&to_lex);

    while let Some(result) = lexer.next() {
        let token = result.map_err(|_| JsonError::Unknown)?;
        let span = lexer.span();
        tokens.push(Spanned {
            value: token,
            // Adjust span by global offset
            span: Span::new(
                self.offset + span.start,
                self.offset + span.end
            ),
        });
    }

    // Update state
    self.offset += to_lex.len();
    self.buffer = remainder;

    Ok(tokens)
}
```

## Capacity Hints

Pre-allocate buffers based on expected input:

```rust,ignore
// Small: <1KB inputs
let hint = LexerCapacityHint::small();

// Medium: 1KB-64KB (default)
let hint = LexerCapacityHint::medium();

// Large: >64KB
let hint = LexerCapacityHint::large();

// Custom: from expected chunk size
let hint = LexerCapacityHint::from_chunk_size(4096);

let lexer = JsonIncrementalLexer::with_capacity_hint(hint);
```

## Using `feed_into` for Zero-Copy

Avoid repeated allocations with `feed_into`:

```rust,ignore
let mut lexer = JsonIncrementalLexer::new();
let mut token_buffer = Vec::with_capacity(1024);

while let Some(chunk) = source.next_chunk().await {
    let added = lexer.feed_into(&chunk, &mut token_buffer)?;
    println!("Added {} tokens", added);

    // Process and drain tokens...
}
```

## Span Tracking

All spans are global - they reference positions in the complete input:

```text
Chunk 1 (offset 0):    {"a":1}\n
Spans:                 0-1, 1-4, 4-5, 5-6, 6-7, 7-8

Chunk 2 (offset 8):    {"b":2}\n
Spans:                 8-9, 9-12, 12-13, 13-14, 14-15, 15-16
                       ^
                       offset added
```

## Next

[Chapter 4: Incremental Parse →](04-parse.md)
