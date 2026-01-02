# Security Considerations

synkit is designed for parsing untrusted input. This page documents the security model, protections, and best practices for generated parsers.

## No Unsafe Code

synkit uses zero `unsafe` blocks in core, macros, and kit crates. Memory safety is guaranteed by the Rust compiler.

```bash
# Verify yourself
grep -r "unsafe" core/src macros/src kit/src
# Returns no matches
```

## Resource Exhaustion Protection

### Recursion Limits

Deeply nested input like `[[[[[[...]]]]]]` can cause stack overflow. synkit provides configurable recursion limits:

```rust
use synkit::ParseConfig;

// Default: 128 levels (matches serde_json)
let config = ParseConfig::default();

// Stricter limit for untrusted input
let config = ParseConfig::new()
    .with_max_recursion_depth(32);

// Track depth manually in your parser
use synkit::RecursionGuard;

struct MyParser {
    depth: RecursionGuard,
    config: ParseConfig,
}

impl MyParser {
    fn parse_nested(&mut self) -> Result<(), MyError> {
        self.depth.enter(self.config.max_recursion_depth)?;
        // ... parse nested content ...
        self.depth.exit();
        Ok(())
    }
}
```

### Token Limits

Prevent CPU exhaustion from extremely long inputs:

```rust
let config = ParseConfig::new()
    .with_max_tokens(100_000);  // Fail after 100k tokens
```

### Buffer Limits (Streaming)

For incremental parsing, `StreamConfig` controls memory usage:

```rust
use synkit::StreamConfig;

let config = StreamConfig {
    max_chunk_size: 16 * 1024,      // 16KB max per chunk
    token_buffer_size: 1024,        // Token buffer capacity
    ast_buffer_size: 64,            // AST node buffer
    ..StreamConfig::default()
};
```

Exceeding limits produces explicit errors:

| Error                           | Trigger                               |
| ------------------------------- | ------------------------------------- |
| `StreamError::ChunkTooLarge`    | Input chunk > `max_chunk_size`        |
| `StreamError::BufferOverflow`   | Token buffer exceeded capacity        |
| `StreamError::ResourceLimit`    | Generic limit exceeded                |
| `Error::RecursionLimitExceeded` | Nesting depth > `max_recursion_depth` |
| `Error::TokenLimitExceeded`     | Token count > `max_tokens`            |

## Integer Safety

All span arithmetic uses saturating operations to prevent overflow panics:

```rust
// Span length - saturating subtraction
fn len(&self) -> usize {
    self.end().saturating_sub(self.start())
}

// Recursion guard - saturating increment
self.depth = self.depth.saturating_add(1);

// Cursor bounds - clamped to valid range
self.cursor = pos.clamp(self.range_start, self.range_end);
```

See [Safety & Clamping](safety.md) for detailed behavior documentation.

## Memory Safety

Generated `TokenStream` uses `Arc` for shared ownership:

```rust
pub struct TokenStream {
    source: Arc<str>,           // Shared source text
    tokens: Arc<Vec<Token>>,    // Shared token buffer
    // ... cursors are Copy types
}
```

Benefits:

- `fork()` is zero-copy (Arc::clone only)
- Thread-safe: `TokenStream` is `Send + Sync`
- No dangling references possible

## Fuzz Testing

synkit includes continuous fuzz testing via `cargo-fuzz`:

```bash
# Run lexer fuzzer
cargo +nightly fuzz run fuzz_lexer

# Run parser fuzzer
cargo +nightly fuzz run fuzz_parser
```

Fuzz targets exercise:

- Arbitrary UTF-8 input
- Edge cases in span arithmetic
- Token stream operations
- Incremental buffer management

### Adding Fuzz Tests for Your Parser

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Ignore lex errors, just ensure no panics
        let _ = my_parser::TokenStream::lex(s);
    }
});
```

## Security Checklist

When building a parser for untrusted input:

- [ ] Set `max_recursion_depth` appropriate for your format
- [ ] Set `max_tokens` to prevent CPU exhaustion
- [ ] Use `StreamConfig` limits for streaming parsers
- [ ] Handle all error variants (don't unwrap)
- [ ] Add fuzz tests for your grammar
- [ ] Consider timeout limits at the application layer

## Threat Model

synkit protects against:

| Threat             | Protection                 |
| ------------------ | -------------------------- |
| Stack overflow     | Recursion limits           |
| Memory exhaustion  | Buffer limits, Arc sharing |
| CPU exhaustion     | Token limits               |
| Integer overflow   | Saturating arithmetic      |
| Undefined behavior | No unsafe code             |

synkit does NOT protect against:

| Threat                     | Mitigation                       |
| -------------------------- | -------------------------------- |
| Regex backtracking (logos) | Use logos' regex restrictions    |
| Application-level DoS      | Add timeouts in your application |
| Malicious AST semantics    | Validate AST after parsing       |

## Reporting Vulnerabilities

Please open a Github security advisory.
