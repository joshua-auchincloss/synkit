# Async Streaming

> **📦 Source**: [examples/jsonl-parser/src/incremental.rs](https://github.com/joshua-auchincloss/synkit/blob/main/examples/jsonl-parser/src/incremental.rs)

synkit provides async streaming support via `tokio` and `futures` feature flags.

## Feature Flags

```toml
# Cargo.toml

# For tokio runtime
synkit = { version = "0.1", features = ["tokio"] }

# For runtime-agnostic futures
synkit = { version = "0.1", features = ["futures"] }

# For both
synkit = { version = "0.1", features = ["tokio", "futures"] }
```

## Architecture

```text
┌──────────┐     ┌───────────────────┐     ┌──────────────┐
│  Source  │────▶│ AsyncTokenStream  │────▶│  AstStream   │────▶ Consumer
│ (chunks) │     │     (lexer)       │     │   (parser)   │
└──────────┘     └───────────────────┘     └──────────────┘
                         │                        │
                    mpsc::channel            mpsc::channel
```

## Tokio Implementation

### AsyncTokenStream

Receives source chunks, emits tokens:

```rust,ignore
use synkit::async_stream::tokio_impl::AsyncTokenStream;
use tokio::sync::mpsc;

let (token_tx, token_rx) = mpsc::channel(1024);
let mut lexer = AsyncTokenStream::<JsonIncrementalLexer>::new(token_tx);

// Feed chunks
lexer.feed(chunk).await?;

// Signal completion
lexer.finish().await?;
```

### AstStream

Receives tokens, emits AST nodes:

```rust,ignore
use synkit::async_stream::tokio_impl::AstStream;

let (ast_tx, mut ast_rx) = mpsc::channel(64);
let mut parser = AstStream::<JsonLine, Token>::new(token_rx, ast_tx);

// Run until token stream exhausted
parser.run().await?;
```

### Full Pipeline

```rust,ignore
use synkit::async_stream::{StreamConfig, tokio_impl::*};
use tokio::sync::mpsc;

async fn process_jsonl_stream(
    mut source: impl Stream<Item = String>,
) -> Result<Vec<JsonLine>, StreamError> {
    let config = StreamConfig::medium();

    // Create channels
    let (token_tx, token_rx) = mpsc::channel(config.token_buffer_size);
    let (ast_tx, mut ast_rx) = mpsc::channel(config.ast_buffer_size);

    // Spawn lexer task
    let lexer_handle = tokio::spawn(async move {
        let mut lexer = AsyncTokenStream::<JsonIncrementalLexer>::with_config(
            token_tx,
            config.clone()
        );
        while let Some(chunk) = source.next().await {
            lexer.feed(&chunk).await?;
        }
        lexer.finish().await
    });

    // Spawn parser task
    let parser_handle = tokio::spawn(async move {
        let mut parser = AstStream::<JsonLine, Token>::with_config(
            token_rx,
            ast_tx,
            config,
        );
        parser.run().await
    });

    // Collect results
    let mut results = Vec::new();
    while let Some(line) = ast_rx.recv().await {
        results.push(line);
    }

    // Wait for tasks
    lexer_handle.await??;
    parser_handle.await??;

    Ok(results)
}
```

## StreamConfig

Configure buffer sizes and limits:

```rust,ignore
let config = StreamConfig {
    token_buffer_size: 1024,  // Channel + buffer capacity
    ast_buffer_size: 64,       // AST channel capacity
    max_chunk_size: 64 * 1024, // Reject chunks > 64KB
    lexer_hint: LexerCapacityHint::medium(),
};

// Or use presets
let config = StreamConfig::small();   // <1KB inputs
let config = StreamConfig::medium();  // 1KB-64KB (default)
let config = StreamConfig::large();   // >64KB inputs
```

## Futures Implementation

For runtime-agnostic streaming, use `ParseStream`:

```rust,ignore
use synkit::async_stream::futures_impl::ParseStream;
use futures_util::StreamExt;

let token_stream: impl Stream<Item = Token> = /* from lexer */;
let mut parse_stream = ParseStream::<_, JsonLine, _>::new(token_stream);

while let Some(result) = parse_stream.next().await {
    match result {
        Ok(line) => process(line),
        Err(e) => handle_error(e),
    }
}
```

## Error Handling

`StreamError` covers all streaming failure modes:

```rust,ignore
pub enum StreamError {
    ChannelClosed,              // Unexpected channel closure
    LexError(String),           // Lexer failure
    ParseError(String),         // Parser failure
    IncompleteInput,            // EOF with partial data
    ChunkTooLarge { size, max }, // Input exceeds limit
    BufferOverflow { current, max }, // Buffer exceeded
    Timeout,                    // Deadline exceeded
    ResourceLimit { resource, current, max },
}
```

Handle errors appropriately:

```rust,ignore
match parser.run().await {
    Ok(()) => println!("Complete"),
    Err(StreamError::IncompleteInput) => {
        eprintln!("Warning: truncated input");
    }
    Err(StreamError::ParseError(msg)) => {
        eprintln!("Parse error: {}", msg);
        // Could log and continue with next record
    }
    Err(e) => return Err(e.into()),
}
```

## Backpressure

Channel-based streaming provides natural backpressure:

- If consumer is slow, channels fill up
- Producers block on `send().await`
- Memory usage stays bounded

Configure based on throughput needs:

```rust,ignore
// High throughput, more memory
let (tx, rx) = mpsc::channel(4096);

// Low latency, less memory
let (tx, rx) = mpsc::channel(16);
```

## Next

[Chapter 6: Stress Testing →](06-testing.md)
