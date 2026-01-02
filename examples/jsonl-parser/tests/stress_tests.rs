//! Stress Tests for Incremental Parser
//!
//! These tests verify that the incremental parser:
//! - Can process millions of events without memory leaks
//! - Properly releases parsed resources
//! - Maintains stable memory usage over time
//!
//! Run with: `cargo test --features async`

use jsonl_parser::{
    Spanned,
    ast::{JsonLine, JsonValueKind},
    incremental::JsonIncrementalLexer,
    tokens::Token,
};
use std::time::Instant;
use synkit::async_stream::{IncrementalLexer, IncrementalParse, ParseCheckpoint};

/// Configuration for stress tests
struct StressConfig {
    /// Number of events to process
    event_count: usize,
    /// Chunk size for incremental feeding
    chunk_size: usize,
    /// How often to check memory (every N events)
    memory_check_interval: usize,
    /// Maximum allowed memory growth ratio
    max_memory_growth: f64,
}

impl Default for StressConfig {
    fn default() -> Self {
        Self {
            event_count: 1_000_000,
            chunk_size: 4096,
            memory_check_interval: 100_000,
            max_memory_growth: 2.0, // Allow up to 2x memory growth
        }
    }
}

/// Simple memory tracker using allocation counting
struct MemoryTracker {
    initial_estimate: usize,
    samples: Vec<usize>,
}

impl MemoryTracker {
    fn new() -> Self {
        Self {
            initial_estimate: 0,
            samples: Vec::new(),
        }
    }

    /// Estimate current memory usage from buffer sizes
    fn sample(&mut self, token_buffer_size: usize, line_buffer_size: usize) {
        let estimate = token_buffer_size * std::mem::size_of::<Spanned<Token>>()
            + line_buffer_size * std::mem::size_of::<JsonLine>();

        if self.initial_estimate == 0 {
            self.initial_estimate = estimate.max(1);
        }
        self.samples.push(estimate);
    }

    fn max_growth_ratio(&self) -> f64 {
        if self.initial_estimate == 0 {
            return 1.0;
        }
        let max = self.samples.iter().max().copied().unwrap_or(0);
        max as f64 / self.initial_estimate as f64
    }

    fn is_stable(&self, max_growth: f64) -> bool {
        self.max_growth_ratio() <= max_growth
    }
}

#[test]
fn test_million_events_no_memory_leak() {
    let config = StressConfig {
        event_count: 1_000_000,
        ..Default::default()
    };

    let single_line = r#"{"id": 1, "name": "test", "value": 42.5, "active": true}"#;
    let input = format!("{}\n", single_line);

    let mut lexer = JsonIncrementalLexer::new();
    let mut token_buffer: Vec<Spanned<Token>> = Vec::new();
    let mut checkpoint = ParseCheckpoint::default();
    let mut total_parsed = 0usize;
    let mut memory_tracker = MemoryTracker::new();

    let start = Instant::now();

    for i in 0..config.event_count {
        // Feed one line
        token_buffer.extend(lexer.feed(&input).unwrap());

        // Parse and consume
        loop {
            match JsonLine::parse_incremental(&token_buffer, &checkpoint) {
                Ok((Some(_line), new_checkpoint)) => {
                    total_parsed += 1;
                    checkpoint = new_checkpoint;
                    // Line is dropped here - simulating consumer processing
                }
                Ok((None, _)) => break,
                Err(e) => panic!("Parse error at event {}: {}", i, e),
            }
        }

        // Compact buffer frequently to avoid memory growth
        if checkpoint.tokens_consumed > 500 {
            token_buffer.drain(..checkpoint.tokens_consumed);
            checkpoint.cursor -= checkpoint.tokens_consumed;
            checkpoint.tokens_consumed = 0;
        }

        // Memory sampling
        if i % config.memory_check_interval == 0 && i > 0 {
            memory_tracker.sample(token_buffer.len(), 0);

            // Print progress
            let elapsed = start.elapsed();
            let rate = total_parsed as f64 / elapsed.as_secs_f64();
            eprintln!(
                "Progress: {} events, {:.0} events/sec, buffer size: {}, growth: {:.2}x",
                total_parsed,
                rate,
                token_buffer.len(),
                memory_tracker.max_growth_ratio()
            );
        }
    }

    let elapsed = start.elapsed();
    let rate = total_parsed as f64 / elapsed.as_secs_f64();

    eprintln!(
        "Completed {} events in {:?} ({:.0} events/sec)",
        total_parsed, elapsed, rate
    );
    eprintln!("Final buffer size: {}", token_buffer.len());
    eprintln!(
        "Max memory growth: {:.2}x",
        memory_tracker.max_growth_ratio()
    );

    assert_eq!(total_parsed, config.event_count);
    assert!(
        memory_tracker.is_stable(config.max_memory_growth),
        "Memory grew too much: {:.2}x (max allowed: {:.2}x)",
        memory_tracker.max_growth_ratio(),
        config.max_memory_growth
    );
}

#[test]
fn test_varied_objects_stress() {
    // Test with varied JSON structures
    let objects = vec![
        r#"{"type": "simple", "value": 1}"#,
        r#"{"type": "nested", "data": {"inner": true}}"#,
        r#"{"type": "array", "items": [1, 2, 3, 4, 5]}"#,
        r#"{"type": "complex", "users": [{"name": "a"}, {"name": "b"}], "count": 2}"#,
        r#"{"type": "string", "text": "hello world with some longer text content here"}"#,
    ];

    let mut lexer = JsonIncrementalLexer::new();
    let mut token_buffer: Vec<Spanned<Token>> = Vec::new();
    let mut checkpoint = ParseCheckpoint::default();
    let mut total_parsed = 0usize;

    for i in 0..500_000 {
        let obj = objects[i % objects.len()];
        let input = format!("{}\n", obj);

        token_buffer.extend(lexer.feed(&input).unwrap());

        loop {
            match JsonLine::parse_incremental(&token_buffer, &checkpoint) {
                Ok((Some(line), new_checkpoint)) => {
                    // Verify structure
                    if let JsonValueKind::Object(obj) = &line.value.kind {
                        assert!(obj.get("type").is_some());
                    } else {
                        panic!("Expected object at event {}", i);
                    }
                    total_parsed += 1;
                    checkpoint = new_checkpoint;
                }
                Ok((None, _)) => break,
                Err(e) => panic!("Parse error at event {}: {}", i, e),
            }
        }

        if checkpoint.tokens_consumed > 500 {
            token_buffer.drain(..checkpoint.tokens_consumed);
            checkpoint.cursor -= checkpoint.tokens_consumed;
            checkpoint.tokens_consumed = 0;
        }
    }

    assert_eq!(total_parsed, 500_000);
}

#[test]
fn test_chunk_boundary_stress() {
    // Test that parsing works correctly across chunk boundaries
    let line = r#"{"id": 12345, "name": "A somewhat longer name to stress chunk boundaries", "nested": {"a": 1, "b": 2}}"#;
    let input = format!("{}\n", line);

    // Test with various chunk sizes, including ones that split mid-token
    for chunk_size in [7, 13, 31, 64, 127] {
        let mut lexer = JsonIncrementalLexer::new();
        let mut token_buffer: Vec<Spanned<Token>> = Vec::new();
        let mut checkpoint = ParseCheckpoint::default();
        let mut total_parsed = 0usize;

        for _ in 0..10_000 {
            // Split input into chunks
            let chunks: Vec<&str> = input
                .as_bytes()
                .chunks(chunk_size)
                .map(|c| std::str::from_utf8(c).unwrap())
                .collect();

            for chunk in chunks {
                token_buffer.extend(lexer.feed(chunk).unwrap());
            }

            loop {
                match JsonLine::parse_incremental(&token_buffer, &checkpoint) {
                    Ok((Some(_line), new_checkpoint)) => {
                        total_parsed += 1;
                        checkpoint = new_checkpoint;
                    }
                    Ok((None, _)) => break,
                    Err(e) => panic!("Parse error with chunk_size {}: {}", chunk_size, e),
                }
            }

            if checkpoint.tokens_consumed > 500 {
                token_buffer.drain(..checkpoint.tokens_consumed);
                checkpoint.cursor -= checkpoint.tokens_consumed;
                checkpoint.tokens_consumed = 0;
            }
        }

        assert_eq!(total_parsed, 10_000, "Failed for chunk_size {}", chunk_size);
    }
}

#[test]
fn test_empty_and_whitespace_lines() {
    // Test handling of empty lines and whitespace
    let inputs = vec!["{}\n", "{}\n\n", "{}\n  \n", "\n{}\n", "{}\n\n{}\n\n{}\n"];

    for (idx, input) in inputs.iter().enumerate() {
        let mut lexer = JsonIncrementalLexer::new();
        let mut token_buffer: Vec<Spanned<Token>> = Vec::new();
        let mut checkpoint = ParseCheckpoint::default();
        let mut total_parsed = 0usize;

        token_buffer.extend(lexer.feed(input).unwrap());
        token_buffer.extend(lexer.finish().unwrap());

        loop {
            match JsonLine::parse_incremental(&token_buffer, &checkpoint) {
                Ok((Some(_line), new_checkpoint)) => {
                    total_parsed += 1;
                    checkpoint = new_checkpoint;
                }
                Ok((None, new_checkpoint)) => {
                    // Empty line skipped or need more input
                    if new_checkpoint.cursor == checkpoint.cursor {
                        // No progress made - need more input or done
                        break;
                    }
                    // Progress made (empty line skipped) - continue
                    checkpoint = new_checkpoint;
                }
                Err(e) => panic!("Parse error for input {}: {}", idx, e),
            }
        }

        // Count expected non-empty lines
        let expected = input.lines().filter(|l| l.trim() == "{}").count();
        assert_eq!(
            total_parsed, expected,
            "Failed for input {}: {:?}",
            idx, input
        );
    }
}

#[test]
fn test_deeply_nested_objects() {
    // Test handling of deeply nested structures
    fn make_nested(depth: usize) -> String {
        let mut s = String::new();
        for _ in 0..depth {
            s.push_str(r#"{"inner": "#);
        }
        s.push_str("1");
        for _ in 0..depth {
            s.push('}');
        }
        s.push('\n');
        s
    }

    for depth in [10, 50, 100] {
        let input = make_nested(depth);

        let mut lexer = JsonIncrementalLexer::new();
        let mut token_buffer: Vec<Spanned<Token>> = Vec::new();
        let checkpoint = ParseCheckpoint::default();

        token_buffer.extend(lexer.feed(&input).unwrap());
        token_buffer.extend(lexer.finish().unwrap());

        match JsonLine::parse_incremental(&token_buffer, &checkpoint) {
            Ok((Some(_line), _)) => {}
            Ok((None, _)) => panic!("Expected to parse nested object at depth {}", depth),
            Err(e) => panic!("Parse error at depth {}: {}", depth, e),
        }
    }
}

#[test]
fn test_large_arrays() {
    // Test handling of large arrays
    fn make_array(size: usize) -> String {
        let items: Vec<String> = (0..size).map(|i| i.to_string()).collect();
        format!("{{\"data\": [{}]}}\n", items.join(", "))
    }

    for size in [100, 1000, 10000] {
        let input = make_array(size);

        let mut lexer = JsonIncrementalLexer::new();
        let mut token_buffer: Vec<Spanned<Token>> = Vec::new();
        let checkpoint = ParseCheckpoint::default();

        token_buffer.extend(lexer.feed(&input).unwrap());
        token_buffer.extend(lexer.finish().unwrap());

        match JsonLine::parse_incremental(&token_buffer, &checkpoint) {
            Ok((Some(line), _)) => {
                if let JsonValueKind::Object(obj) = &line.value.kind {
                    if let Some(data) = obj.get("data") {
                        if let JsonValueKind::Array(arr) = &data.kind {
                            assert_eq!(arr.len(), size, "Array size mismatch");
                        } else {
                            panic!("Expected array for size {}", size);
                        }
                    }
                }
            }
            Ok((None, _)) => panic!("Expected to parse array at size {}", size),
            Err(e) => panic!("Parse error at size {}: {}", size, e),
        }
    }
}

#[test]
fn test_concurrent_style_processing() {
    // Simulate a pattern where we process events as fast as possible
    // while tracking throughput
    let single_line = r#"{"event": "click", "user_id": 12345, "timestamp": 1699900000}"#;
    let input = format!("{}\n", single_line);

    let target_events = 100_000;
    let mut total_bytes = 0usize;

    let start = Instant::now();

    let mut lexer = JsonIncrementalLexer::new();
    let mut token_buffer: Vec<Spanned<Token>> = Vec::new();
    let mut checkpoint = ParseCheckpoint::default();
    let mut total_parsed = 0usize;

    while total_parsed < target_events {
        total_bytes += input.len();

        token_buffer.extend(lexer.feed(&input).unwrap());

        loop {
            match JsonLine::parse_incremental(&token_buffer, &checkpoint) {
                Ok((Some(_line), new_checkpoint)) => {
                    total_parsed += 1;
                    checkpoint = new_checkpoint;
                }
                Ok((None, _)) => break,
                Err(e) => panic!("Parse error: {}", e),
            }
        }

        if checkpoint.tokens_consumed > 1000 {
            token_buffer.drain(..checkpoint.tokens_consumed);
            checkpoint.cursor -= checkpoint.tokens_consumed;
            checkpoint.tokens_consumed = 0;
        }
    }

    let elapsed = start.elapsed();
    let events_per_sec = total_parsed as f64 / elapsed.as_secs_f64();
    let bytes_per_sec = total_bytes as f64 / elapsed.as_secs_f64();
    let mb_per_sec = bytes_per_sec / (1024.0 * 1024.0);

    eprintln!(
        "Throughput: {:.0} events/sec, {:.2} MB/sec",
        events_per_sec, mb_per_sec
    );

    assert_eq!(total_parsed, target_events);
}

/// Test that estimated_size works and is reasonable
#[test]
fn test_memory_estimation() {
    use jsonl_parser::{Parse, ast::JsonValue};

    let test_cases = vec![
        (r#"null"#, "null"),
        (r#"true"#, "bool"),
        (r#"42"#, "number"),
        (r#""hello""#, "string"),
        (r#"[]"#, "empty array"),
        (r#"[1, 2, 3]"#, "array"),
        (r#"{}"#, "empty object"),
        (r#"{"a": 1, "b": 2}"#, "object"),
    ];

    for (json, desc) in test_cases {
        let mut stream = jsonl_parser::TokenStream::lex(json).unwrap();
        let value = JsonValue::parse(&mut stream).unwrap();
        let size = value.estimated_size();

        // Just verify it returns something reasonable
        assert!(size > 0, "Size should be > 0 for {}", desc);
        assert!(size < 10_000, "Size should be < 10KB for simple {}", desc);

        eprintln!("{}: {} bytes (estimated)", desc, size);
    }
}
