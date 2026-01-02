# Incremental Parse

> **📦 Source**: [examples/jsonl-parser/src/incremental.rs](https://github.com/joshua-auchincloss/synkit/blob/main/examples/jsonl-parser/src/incremental.rs)

The `IncrementalParse` trait enables parsing from a growing token buffer.

## IncrementalParse Trait

```rust,ignore
pub trait IncrementalParse: Sized {
    type Token: Clone;
    type Error: fmt::Display;

    /// Attempt to parse from tokens starting at checkpoint
    ///
    /// Returns:
    /// - `Ok((Some(node), new_checkpoint))` - Parsed successfully
    /// - `Ok((None, checkpoint))` - Need more tokens
    /// - `Err(error)` - Unrecoverable error
    fn parse_incremental<S>(
        tokens: &[S],
        checkpoint: &ParseCheckpoint,
    ) -> Result<(Option<Self>, ParseCheckpoint), Self::Error>
    where
        S: AsRef<Self::Token>;

    /// Check if parsing might succeed with current tokens
    fn can_parse<S>(tokens: &[S], checkpoint: &ParseCheckpoint) -> bool
    where
        S: AsRef<Self::Token>;
}
```

## ParseCheckpoint

Track parser state between parse attempts:

```rust,ignore
#[derive(Debug, Clone, Default)]
pub struct ParseCheckpoint {
    /// Position in token buffer
    pub cursor: usize,
    /// Tokens consumed (for buffer compaction)
    pub tokens_consumed: usize,
    /// Custom state (e.g., nesting depth)
    pub state: u64,
}
```

## JSONL Implementation Strategy

Rather than re-implementing parsing logic, we reuse the standard `Parse` trait:

```rust,ignore
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
        // 1. Find chunk boundary
        let boundary = match Self::find_boundary(tokens, checkpoint.cursor) {
            Some(b) => b,
            None => return Ok((None, checkpoint.clone())), // Need more
        };

        // 2. Extract chunk tokens
        let chunk = &tokens[checkpoint.cursor..boundary];

        // 3. Build TokenStream from chunk
        let stream_tokens: Vec<_> = chunk.iter()
            .map(|s| /* convert to SpannedToken */)
            .collect();

        let mut stream = TokenStream::from_tokens(/* ... */);

        // 4. Use standard Parse implementation
        let line = JsonLine::parse(&mut stream)?;

        // 5. Return with updated checkpoint
        let consumed = boundary - checkpoint.cursor;
        Ok((
            Some(line),
            ParseCheckpoint {
                cursor: boundary,
                tokens_consumed: checkpoint.tokens_consumed + consumed,
                state: 0,
            },
        ))
    }

    fn can_parse<S>(tokens: &[S], checkpoint: &ParseCheckpoint) -> bool
    where
        S: AsRef<Self::Token>,
    {
        // Can parse if there's a complete chunk
        Self::find_boundary(tokens, checkpoint.cursor).is_some()
    }
}
```

## Key Design: Reuse Parse Trait

The incremental parser delegates to the standard `Parse` implementation. This ensures:

1. **Consistency** - Same parsing logic for sync and async
2. **Maintainability** - One parser implementation to update
3. **Testing** - Sync tests validate incremental behavior

## Using IncrementalBuffer

The `IncrementalBuffer` helper manages tokens efficiently:

```rust,ignore
use synkit::async_stream::{IncrementalBuffer, parse_available_chunks};

let mut buffer = IncrementalBuffer::with_capacity(1024);
let mut lexer = JsonIncrementalLexer::new();

// Feed tokens
buffer.extend(lexer.feed(chunk)?);

// Parse all available chunks
let results = parse_available_chunks::<JsonLine, _, _, _, _>(
    &mut buffer,
    |tokens| {
        let mut stream = TokenStream::from_tokens(/* ... */);
        JsonLine::parse(&mut stream)
    },
)?;

// Compact buffer to release memory
buffer.compact();
```

## IncrementalBuffer Operations

```rust,ignore
// Access unconsumed tokens
let remaining = buffer.remaining();

// Mark tokens as consumed
buffer.consume(count);

// Remove consumed tokens from memory
buffer.compact();

// Check size
let len = buffer.len();         // Unconsumed count
let total = buffer.total_tokens(); // Including consumed
```

## Error Handling

Return errors for unrecoverable parsing failures:

```rust,ignore
fn parse_incremental<S>(
    tokens: &[S],
    checkpoint: &ParseCheckpoint,
) -> Result<(Option<Self>, ParseCheckpoint), Self::Error> {
    // ...
    match JsonLine::parse(&mut stream) {
        Ok(line) => Ok((Some(line), new_checkpoint)),
        Err(e) => {
            // For recoverable errors, could return Ok((None, ...))
            // For unrecoverable, propagate the error
            Err(e)
        }
    }
}
```

## Checkpoint State

Use `state: u64` for parser-specific context:

```rust,ignore
// Example: Track nesting depth
let checkpoint = ParseCheckpoint {
    cursor: 100,
    tokens_consumed: 50,
    state: 3, // Currently at depth 3
};
```

## Next

[Chapter 5: Async Streaming →](05-async.md)
