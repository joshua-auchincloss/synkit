# Stress Testing

> **📦 Source**: [examples/jsonl-parser/tests/stress_tests.rs](https://github.com/joshua-auchincloss/synkit/blob/main/examples/jsonl-parser/tests/stress_tests.rs)

Validate incremental parsers handle high throughput without memory leaks.

## Test Strategy

1. **Volume** - Process millions of events
2. **Memory stability** - Track buffer sizes, detect leaks
3. **Varied input** - Different object sizes and structures
4. **Buffer compaction** - Verify consumed tokens are released

## Million Event Test

```rust,ignore
#[test]
fn test_million_events_no_memory_leak() {
    let config = StressConfig {
        event_count: 1_000_000,
        chunk_size: 4096,
        memory_check_interval: 100_000,
        max_memory_growth: 2.0,
    };

    let input = r#"{"id": 1, "name": "test", "value": 42.5}\n"#;

    let mut lexer = JsonIncrementalLexer::new();
    let mut token_buffer: Vec<Spanned<Token>> = Vec::new();
    let mut checkpoint = ParseCheckpoint::default();
    let mut total_parsed = 0;
    let mut memory_tracker = MemoryTracker::new();

    for i in 0..config.event_count {
        // Feed one line
        token_buffer.extend(lexer.feed(&input)?);

        // Parse available
        loop {
            match JsonLine::parse_incremental(&token_buffer, &checkpoint) {
                Ok((Some(_line), new_cp)) => {
                    total_parsed += 1;
                    checkpoint = new_cp;
                }
                Ok((None, _)) => break,
                Err(e) => panic!("Parse error at event {}: {}", i, e),
            }
        }

        // Compact frequently to avoid memory growth
        if checkpoint.tokens_consumed > 500 {
            token_buffer.drain(..checkpoint.tokens_consumed);
            checkpoint.cursor -= checkpoint.tokens_consumed;
            checkpoint.tokens_consumed = 0;
        }

        // Memory sampling
        if i % config.memory_check_interval == 0 {
            memory_tracker.sample(token_buffer.len(), 0);
        }
    }

    assert_eq!(total_parsed, config.event_count);
    assert!(memory_tracker.is_stable(config.max_memory_growth));
}
```

## Memory Tracking

```rust,ignore
struct MemoryTracker {
    initial_estimate: usize,
    samples: Vec<usize>,
}

impl MemoryTracker {
    fn sample(&mut self, token_buffer_size: usize, line_buffer_size: usize) {
        let estimate = token_buffer_size * size_of::<Spanned<Token>>()
            + line_buffer_size * size_of::<JsonLine>();

        if self.initial_estimate == 0 {
            self.initial_estimate = estimate.max(1);
        }
        self.samples.push(estimate);
    }

    fn max_growth_ratio(&self) -> f64 {
        let max = self.samples.iter().max().copied().unwrap_or(0);
        max as f64 / self.initial_estimate as f64
    }

    fn is_stable(&self, max_growth: f64) -> bool {
        self.max_growth_ratio() <= max_growth
    }
}
```

## Varied Input Test

Test with different JSON structures:

```rust,ignore
#[test]
fn test_varied_objects_stress() {
    let objects = vec![
        r#"{"type": "simple", "value": 1}"#,
        r#"{"type": "nested", "data": {"inner": true}}"#,
        r#"{"type": "array", "items": [1, 2, 3, 4, 5]}"#,
        r#"{"type": "complex", "users": [{"name": "a"}], "count": 2}"#,
    ];

    for i in 0..500_000 {
        let obj = objects[i % objects.len()];
        let input = format!("{}\n", obj);

        // Feed, parse, verify...
    }
}
```

## Buffer Compaction

Critical for memory stability:

```rust,ignore
// Bad: Buffer grows unbounded
loop {
    token_buffer.extend(lexer.feed(chunk)?);
    while let Some(line) = parse(&token_buffer)? {
        // Parse but never compact
    }
}

// Good: Compact after consuming
loop {
    token_buffer.extend(lexer.feed(chunk)?);

    while let Some(line) = parse(&token_buffer)? {
        checkpoint = new_checkpoint;
    }

    // Compact when enough consumed
    if checkpoint.tokens_consumed > THRESHOLD {
        token_buffer.drain(..checkpoint.tokens_consumed);
        checkpoint.cursor -= checkpoint.tokens_consumed;
        checkpoint.tokens_consumed = 0;
    }
}
```

## Performance Metrics

Track throughput:

```rust,ignore
let start = Instant::now();

// ... process events ...

let elapsed = start.elapsed();
let rate = total_parsed as f64 / elapsed.as_secs_f64();
println!(
    "Processed {} events in {:?} ({:.0} events/sec)",
    total_parsed, elapsed, rate
);
```

Expected performance (rough guidelines):
- Simple objects: 500K-1M events/sec
- Complex nested: 100K-300K events/sec
- Memory growth: <2x initial

## Running Tests

```bash
# Run stress tests (may take minutes)
cd examples/jsonl-parser
cargo test stress -- --nocapture

# With release optimizations
cargo test --release stress -- --nocapture
```

## Summary

Incremental parsing requires careful attention to:

1. **Buffer management** - Compact regularly
2. **Memory bounds** - Track growth, fail on overflow
3. **Throughput** - Profile hot paths
4. **Correctness** - Same results as sync parsing

The JSONL parser demonstrates these patterns at scale.
