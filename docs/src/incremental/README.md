# Tutorial: JSONL Incremental Parser

Build a high-performance streaming JSON Lines parser using synkit's incremental parsing infrastructure.

## Source Code

> **📦 Complete source**: [examples/jsonl-parser](https://github.com/joshua-auchincloss/synkit/tree/main/examples/jsonl-parser)

## What You'll Learn

1. **ChunkBoundary** - Define where to split token streams
2. **IncrementalLexer** - Buffer partial input, emit complete tokens
3. **IncrementalParse** - Parse from token buffers with checkpoints
4. **Async streaming** - tokio and futures integration
5. **Stress testing** - Validate memory stability under load

## JSON Lines Format

[JSON Lines](https://jsonlines.org/) uses newline-delimited JSON:

```text
{"user": "alice", "action": "login"}
{"user": "bob", "action": "purchase", "amount": 42.50}
{"user": "alice", "action": "logout"}
```

Each line is a complete JSON value. This makes JSONL ideal for:

- Log processing
- Event streams
- Large dataset processing
- Network protocols

## Why Incremental Parsing?

Traditional parsing loads entire input into memory:

```rust,ignore
let input = fs::read_to_string("10gb_logs.jsonl")?;  // ❌ OOM
let docs: Vec<Log> = parse(&input)?;
```

Incremental parsing processes chunks:

```rust,ignore
let mut lexer = JsonIncrementalLexer::new();
while let Some(chunk) = reader.read_chunk().await {
    for token in lexer.feed(&chunk)? {
        // Process tokens as they arrive
    }
}
```

## Prerequisites

- Completed the [TOML Parser Tutorial](../tutorial/README.md) (or familiarity with synkit basics)
- Understanding of async Rust (for chapters 5-6)

## Chapters

| Chapter | Topic | Key Concepts |
|---------|-------|--------------|
| [1. Token Definition](01-tokens.md) | Token enum and `parser_kit!` | logos patterns, `#[no_to_tokens]` |
| [2. Chunk Boundaries](02-boundaries.md) | `ChunkBoundary` trait | depth tracking, boundary detection |
| [3. Incremental Lexer](03-lexer.md) | `IncrementalLexer` trait | buffering, offset tracking |
| [4. Incremental Parse](04-parse.md) | `IncrementalParse` trait | checkpoints, partial results |
| [5. Async Streaming](05-async.md) | tokio/futures integration | channels, backpressure |
| [6. Stress Testing](06-testing.md) | Memory stability | 1M+ events, leak detection |
