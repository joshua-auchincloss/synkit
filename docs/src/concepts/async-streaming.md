# Async Streaming

synkit supports incremental, asynchronous parsing for scenarios where data arrives in chunks:

- Network streams (HTTP, WebSocket, TCP)
- Large file processing
- Real-time log parsing
- Interactive editors

## Architecture

```text
┌─────────────┐     chunks      ┌──────────────────┐
│   Source    │ ──────────────► │ IncrementalLexer │
│ (network,   │                 │   (tokenizer)    │
│  file, etc) │                 └────────┬─────────┘
└─────────────┘                          │
                                   tokens│
                                         ▼
                                ┌────────────────┐
                                │ IncrementalParse│
                                │   (parser)      │
                                └────────┬───────┘
                                         │
                                  AST    │
                                  nodes  ▼
                                ┌────────────────┐
                                │   Consumer     │
                                └────────────────┘
```

## Key Traits

### IncrementalLexer

Lex source text incrementally as chunks arrive:

```rust,ignore
pub trait IncrementalLexer: Sized {
    type Token: Clone;
    type Span: Clone;
    type Spanned: Clone;
    type Error: Display;

    fn new() -> Self;
    fn feed(&mut self, chunk: &str) -> Result<Vec<Self::Spanned>, Self::Error>;
    fn finish(self) -> Result<Vec<Self::Spanned>, Self::Error>;
    fn offset(&self) -> usize;
}
```

### IncrementalParse

Parse AST nodes incrementally from token buffers:

```rust,ignore
pub trait IncrementalParse: Sized {
    type Token: Clone;
    type Error: Display;

    fn parse_incremental<S>(
        tokens: &[S],
        checkpoint: &ParseCheckpoint,
    ) -> Result<(Option<Self>, ParseCheckpoint), Self::Error>
    where
        S: AsRef<Self::Token>;

    fn can_parse<S>(tokens: &[S], checkpoint: &ParseCheckpoint) -> bool
    where
        S: AsRef<Self::Token>;
}
```

## ParseCheckpoint

Track parser state across incremental calls:

```rust,ignore
pub struct ParseCheckpoint {
    pub cursor: usize,         // Position in token buffer
    pub tokens_consumed: usize, // Total tokens processed
    pub state: u64,            // Parser-specific state
}
```

## Feature Flags

Enable async streaming with feature flags:

```toml
# Tokio-based (channels, spawn)
synkit = { version = "0.1", features = ["tokio"] }

# Futures-based (runtime-agnostic Stream trait)
synkit = { version = "0.1", features = ["futures"] }
```

## Tokio Integration

With the `tokio` feature, use channel-based streaming:

```rust,ignore
use synkit::async_stream::tokio_impl::{AsyncTokenStream, AstStream};
use tokio::sync::mpsc;

async fn parse_stream<L, T>(mut source_rx: mpsc::Receiver<String>)
where
    L: IncrementalLexer,
    T: IncrementalParse<Token = L::Token>,
{
    let (token_tx, token_rx) = mpsc::channel(32);
    let (ast_tx, mut ast_rx) = mpsc::channel(16);

    // Lexer task
    tokio::spawn(async move {
        let mut lexer = AsyncTokenStream::<L>::new(token_tx);
        while let Some(chunk) = source_rx.recv().await {
            lexer.feed(&chunk).await?;
        }
        lexer.finish().await?;
    });

    // Parser task
    tokio::spawn(async move {
        let mut parser = AstStream::<T, L::Token>::new(token_rx, ast_tx);
        parser.run().await?;
    });

    // Consume AST nodes
    while let Some(node) = ast_rx.recv().await {
        process(node);
    }
}
```

## Futures Integration

With the `futures` feature, use the `Stream` trait:

```rust,ignore
use synkit::async_stream::futures_impl::ParseStream;
use futures::StreamExt;

async fn parse_tokens<S, T>(tokens: S)
where
    S: Stream<Item = Token>,
    T: IncrementalParse<Token = Token>,
{
    let mut parse_stream: ParseStream<_, T, _> = ParseStream::new(tokens);

    while let Some(result) = parse_stream.next().await {
        match result {
            Ok(node) => process(node),
            Err(e) => handle_error(e),
        }
    }
}
```

## Error Handling

The `StreamError` enum covers streaming-specific failures:

```rust,ignore
pub enum StreamError {
    ChannelClosed,           // Channel unexpectedly closed
    LexError(String),        // Lexer error
    ParseError(String),      // Parser error
    IncompleteInput,         // EOF with incomplete input
}
```

## Configuration

Customize buffer sizes and limits:

```rust,ignore
let config = StreamConfig {
    token_buffer_size: 1024,   // Token buffer capacity
    ast_buffer_size: 64,       // AST node buffer capacity
    max_chunk_size: 64 * 1024, // Max input chunk size
};

let stream = AsyncTokenStream::with_config(tx, config);
```

## Best Practices

1. **Return `None` when incomplete**: If `parse_incremental` can't complete a node, return `Ok((None, checkpoint))` rather than an error.

2. **Implement `can_parse`**: This optimization prevents unnecessary parse attempts when tokens are clearly insufficient.

3. **Use checkpoints for backtracking**: Store parser state in `checkpoint.state` for complex grammars.

4. **Handle `IncompleteInput`**: At stream end, incomplete input may be valid (e.g., truncated file) or an error depending on your grammar.

5. **Buffer management**: The `AstStream` automatically compacts its buffer. For custom implementations, drain consumed tokens periodically.
