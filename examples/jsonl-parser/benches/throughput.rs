use divan::{Bencher, black_box};
use jsonl_parser::{Spanned, incremental::JsonIncrementalLexer, parse::parse_jsonl, tokens::Token};
use synkit::async_stream::{IncrementalLexer, IncrementalParse, ParseCheckpoint};

use divan::AllocProfiler;

#[allow(unused)]
#[cfg_attr(feature = "alloc", global_allocator)]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

// Sample JSON objects for benchmarking
const SAMPLE_OBJECTS: &[&str] = &[
    r#"{"id": 1, "name": "Alice", "active": true}"#,
    r#"{"id": 2, "name": "Bob", "active": false, "score": 95.5}"#,
    r#"{"id": 3, "name": "Charlie", "tags": ["rust", "parser", "benchmark"]}"#,
    r#"{"user": {"name": "Dave", "email": "dave@example.com"}, "timestamp": 1699900000}"#,
];

/// Generate a JSONL document with N repetitions of sample objects
fn generate_jsonl(count: usize) -> String {
    let mut result = String::with_capacity(count * 100);
    for i in 0..count {
        result.push_str(SAMPLE_OBJECTS[i % SAMPLE_OBJECTS.len()]);
        result.push('\n');
    }
    result
}

/// Generate chunked JSONL input simulating network packets
fn generate_chunks(input: &str, chunk_size: usize) -> Vec<String> {
    input
        .as_bytes()
        .chunks(chunk_size)
        .map(|c| String::from_utf8_lossy(c).to_string())
        .collect()
}

// ANCHOR: batch_benchmarks
#[divan::bench(
    name = "batch_parse",
    args = [100, 1000, 10_000, 100_000, 1_000_000],
)]
fn bench_batch_parse(bencher: Bencher, n: usize) {
    let input = generate_jsonl(n);

    bencher
        .with_inputs(|| input.clone())
        .bench_values(|input| black_box(parse_jsonl(&input).unwrap()));
}

#[divan::bench(
    name = "batch_parse_bytes_throughput",
    args = [100, 1000, 10_000, 100_000, 1_000_000],
)]
fn bench_batch_throughput(bencher: Bencher, n: usize) {
    let input = generate_jsonl(n);
    let bytes = input.len();

    bencher
        .counter(divan::counter::BytesCount::new(bytes))
        .with_inputs(|| input.clone())
        .bench_values(|input| black_box(parse_jsonl(&input).unwrap()));
}
// ANCHOR_END: batch_benchmarks

// ANCHOR: incremental_benchmarks
#[divan::bench(
    name = "incremental_lex",
    args = [100, 1000, 10_000, 100_000, 1_000_000],
)]
fn bench_incremental_lex(bencher: Bencher, n: usize) {
    let input = generate_jsonl(n);
    let chunks = generate_chunks(&input, 4096);

    bencher
        .with_inputs(|| chunks.clone())
        .bench_values(|chunks| {
            let mut lexer = JsonIncrementalLexer::new();
            let mut all_tokens = Vec::new();

            for chunk in chunks {
                all_tokens.extend(lexer.feed(&chunk).unwrap());
            }
            all_tokens.extend(lexer.finish().unwrap());

            black_box(all_tokens)
        });
}

#[divan::bench(
    name = "incremental_lex_bytes_throughput",
    args = [100, 1000, 10_000, 100_000, 1_000_000],
)]
fn bench_incremental_lex_throughput(bencher: Bencher, n: usize) {
    let input = generate_jsonl(n);
    let bytes = input.len();
    let chunks = generate_chunks(&input, 4096);

    bencher
        .counter(divan::counter::BytesCount::new(bytes))
        .with_inputs(|| chunks.clone())
        .bench_values(|chunks| {
            let mut lexer = JsonIncrementalLexer::new();
            let mut all_tokens = Vec::new();

            for chunk in chunks {
                all_tokens.extend(lexer.feed(&chunk).unwrap());
            }
            all_tokens.extend(lexer.finish().unwrap());

            black_box(all_tokens)
        });
}

#[divan::bench(
    name = "incremental_parse",
    args = [100, 1000, 10_000, 100_000, 1_000_000],
)]
fn bench_incremental_parse(bencher: Bencher, n: usize) {
    let input = generate_jsonl(n);
    let chunks = generate_chunks(&input, 4096);

    bencher
        .with_inputs(|| chunks.clone())
        .bench_values(|chunks| {
            // First lex all chunks
            let mut lexer = JsonIncrementalLexer::new();
            let mut all_tokens: Vec<Spanned<Token>> = Vec::new();

            for chunk in chunks {
                all_tokens.extend(lexer.feed(&chunk).unwrap());
            }
            all_tokens.extend(lexer.finish().unwrap());

            // Then parse incrementally
            let mut checkpoint = ParseCheckpoint::default();
            let mut lines = Vec::new();

            loop {
                use jsonl_parser::ast::JsonLine;

                match JsonLine::parse_incremental(&all_tokens, &checkpoint) {
                    Ok((Some(line), new_checkpoint)) => {
                        lines.push(line);
                        checkpoint = new_checkpoint;
                    }
                    Ok((None, _)) => break,
                    Err(_) => break,
                }
            }

            black_box(lines)
        });
}

#[divan::bench(
    name = "incremental_full_pipeline",
    args = [100, 1000, 10_000, 100_000, 1_000_000],
)]
fn bench_incremental_full(bencher: Bencher, n: usize) {
    let input = generate_jsonl(n);
    let chunks = generate_chunks(&input, 4096);

    bencher
        .counter(divan::counter::BytesCount::new(input.len()))
        .with_inputs(|| chunks.clone())
        .bench_values(|chunks| {
            let mut lexer = JsonIncrementalLexer::new();
            let mut token_buffer: Vec<Spanned<Token>> = Vec::new();
            let mut checkpoint = ParseCheckpoint::default();
            let mut lines = Vec::new();

            for chunk in chunks {
                // Lex the chunk
                token_buffer.extend(lexer.feed(&chunk).unwrap());

                // Try to parse any complete lines
                loop {
                    use jsonl_parser::ast::JsonLine;

                    if !JsonLine::can_parse(&token_buffer, &checkpoint) {
                        break;
                    }

                    match JsonLine::parse_incremental(&token_buffer, &checkpoint) {
                        Ok((Some(line), new_checkpoint)) => {
                            lines.push(line);
                            checkpoint = new_checkpoint;
                        }
                        Ok((None, _)) => break,
                        Err(_) => break,
                    }
                }

                // Compact buffer if needed
                if checkpoint.tokens_consumed > 1000 {
                    token_buffer.drain(..checkpoint.tokens_consumed);
                    checkpoint.cursor -= checkpoint.tokens_consumed;
                    checkpoint.tokens_consumed = 0;
                }
            }

            // Finish lexing
            token_buffer.extend(lexer.finish().unwrap());

            // Parse remaining
            loop {
                use jsonl_parser::ast::JsonLine;

                match JsonLine::parse_incremental(&token_buffer, &checkpoint) {
                    Ok((Some(line), new_checkpoint)) => {
                        lines.push(line);
                        checkpoint = new_checkpoint;
                    }
                    Ok((None, _)) => break,
                    Err(_) => break,
                }
            }

            black_box(lines)
        });
}
// ANCHOR_END: incremental_benchmarks

// ANCHOR: chunk_size_benchmarks
#[divan::bench(
    name = "chunk_size_impact",
    args = [64, 256, 1024, 4096, 16384, 65536, 131072, 262144, 524288],
)]
fn bench_chunk_sizes(bencher: Bencher, chunk_size: usize) {
    let input = generate_jsonl(10000);
    let chunks = generate_chunks(&input, chunk_size);

    bencher
        .counter(divan::counter::BytesCount::new(input.len()))
        .with_inputs(|| chunks.clone())
        .bench_values(|chunks| {
            let mut lexer = JsonIncrementalLexer::new();
            let mut all_tokens = Vec::new();

            for chunk in chunks {
                all_tokens.extend(lexer.feed(&chunk).unwrap());
            }
            all_tokens.extend(lexer.finish().unwrap());

            black_box(all_tokens)
        });
}
// ANCHOR_END: chunk_size_benchmarks

// ANCHOR: memory_benchmarks
/// Test that we can process many events without accumulating memory
#[divan::bench]
fn bench_memory_stability() {
    let single_line = r#"{"id": 1, "data": "some payload here", "count": 42}"#;
    let input = format!("{}\n", single_line);

    // Simulate processing 100K events
    let mut lexer = JsonIncrementalLexer::new();
    let mut token_buffer: Vec<Spanned<Token>> = Vec::new();
    let mut checkpoint = ParseCheckpoint::default();
    let mut total_parsed = 0usize;

    for _ in 0..100_000 {
        // Feed one line
        token_buffer.extend(lexer.feed(&input).unwrap());

        // Parse and consume
        loop {
            use jsonl_parser::ast::JsonLine;

            match JsonLine::parse_incremental(&token_buffer, &checkpoint) {
                Ok((Some(_line), new_checkpoint)) => {
                    total_parsed += 1;
                    checkpoint = new_checkpoint;
                    // Drop the line immediately (simulating consumer processing)
                }
                Ok((None, _)) => break,
                Err(_) => break,
            }
        }

        // Compact buffer frequently to avoid memory growth
        if checkpoint.tokens_consumed > 100 {
            token_buffer.drain(..checkpoint.tokens_consumed);
            checkpoint.cursor -= checkpoint.tokens_consumed;
            checkpoint.tokens_consumed = 0;
        }
    }

    black_box(total_parsed);

    // Verify we processed all events
    assert_eq!(total_parsed, 100_000);
}
// ANCHOR_END: memory_benchmarks

// ANCHOR: comparison_benchmarks
/// Compare batch vs incremental parsing for the same data
#[divan::bench(name = "comparison_batch_10k")]
fn bench_compare_batch(bencher: Bencher) {
    let input = generate_jsonl(10_000);

    bencher
        .counter(divan::counter::BytesCount::new(input.len()))
        .with_inputs(|| input.clone())
        .bench_values(|input| black_box(parse_jsonl(&input).unwrap()));
}

#[divan::bench(
    name = "comparison_incremental",
    args = [
        (10_000, 1024),
        (10_000, 2048),
        (10_000, 4096),
        (10_000, 8192),
        (100_000, 1024),
        (100_000, 2048),
        (100_000, 4096),
        (100_000, 8192),
        (100_000, 16384),
        (100_000, 32768),

        (1_000_000, 32768),
        (1_000_000, 65536),
        (1_000_000, 131072),
        (1_000_000, 262144),


        (10_000_000, 32768),
        (10_000_000, 65536),
        (10_000_000, 131072),
        (10_000_000, 262144),
    ]
)]
fn bench_compare_incremental(bencher: Bencher, args: (usize, usize)) {
    let input = generate_jsonl(args.0);
    let chunks = generate_chunks(&input, args.1);

    bencher
        .counter(divan::counter::BytesCount::new(input.len()))
        .with_inputs(|| chunks.clone())
        .bench_values(|chunks| {
            let mut lexer = JsonIncrementalLexer::new();
            let mut token_buffer: Vec<Spanned<Token>> = Vec::new();
            let mut checkpoint = ParseCheckpoint::default();
            let mut lines = Vec::new();

            for chunk in chunks {
                token_buffer.extend(lexer.feed(&chunk).unwrap());

                loop {
                    use jsonl_parser::ast::JsonLine;

                    match JsonLine::parse_incremental(&token_buffer, &checkpoint) {
                        Ok((Some(line), new_checkpoint)) => {
                            lines.push(line);
                            checkpoint = new_checkpoint;
                        }
                        Ok((None, _)) => break,
                        Err(_) => break,
                    }
                }
            }

            token_buffer.extend(lexer.finish().unwrap());

            loop {
                use jsonl_parser::ast::JsonLine;

                match JsonLine::parse_incremental(&token_buffer, &checkpoint) {
                    Ok((Some(line), new_checkpoint)) => {
                        lines.push(line);
                        checkpoint = new_checkpoint;
                    }
                    Ok((None, _)) => break,
                    Err(_) => break,
                }
            }

            black_box(lines)
        });
}
// ANCHOR_END: comparison_benchmarks
