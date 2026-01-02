# Chunk Boundaries

> **📦 Source**: [examples/jsonl-parser/src/incremental.rs](https://github.com/joshua-auchincloss/synkit/blob/main/examples/jsonl-parser/src/incremental.rs)

The `ChunkBoundary` trait defines where token streams can be safely split for incremental parsing.

## The Problem

When processing streaming input, we need to know when we have enough tokens to parse a complete unit. For JSONL, a "complete unit" is a single JSON line ending with a newline.

But we can't just split on any newline - consider:

```json
{"message": "hello\nworld"}
```

The `\n` inside the string is NOT a record boundary.

## ChunkBoundary Trait

```rust,ignore
pub trait ChunkBoundary {
    type Token;

    /// Is this token a potential boundary?
    fn is_boundary_token(token: &Self::Token) -> bool;

    /// Depth change: +1 for openers, -1 for closers
    fn depth_delta(token: &Self::Token) -> i32 { 0 }

    /// Should this token be skipped when scanning?
    fn is_ignorable(token: &Self::Token) -> bool { false }

    /// Find next boundary at depth 0
    fn find_boundary<S: AsRef<Self::Token>>(
        tokens: &[S],
        start: usize
    ) -> Option<usize>;
}
```

## JSONL Implementation

```rust,ignore
impl ChunkBoundary for JsonLine {
    type Token = Token;

    #[inline]
    fn is_boundary_token(token: &Token) -> bool {
        matches!(token, Token::Newline)
    }

    #[inline]
    fn depth_delta(token: &Token) -> i32 {
        match token {
            Token::LBrace | Token::LBracket => 1,  // Opens nesting
            Token::RBrace | Token::RBracket => -1, // Closes nesting
            _ => 0,
        }
    }

    #[inline]
    fn is_ignorable(token: &Token) -> bool {
        matches!(token, Token::Space | Token::Tab)
    }
}
```

## How It Works

The default `find_boundary` implementation:

1. Starts at `depth = 0`
2. For each token:
   - Adds `depth_delta()` to depth
   - If `depth == 0` and `is_boundary_token()`: return position + 1
3. Returns `None` if no boundary found

### Example Token Stream

```text
Tokens: { "a" : 1 } \n { "b" : 2 } \n
Depth:  1       1 0 0  1       1 0 0
        ^             ^             ^
        open          boundary      boundary
```

The first `\n` at index 5 (after `}`) is a valid boundary because depth is 0.

## Finding Boundaries

Use `find_boundary` to locate complete chunks:

```rust,ignore
let tokens: Vec<Spanned<Token>> = /* from lexer */;
let mut start = 0;

while let Some(end) = JsonLine::find_boundary(&tokens, start) {
    let chunk = &tokens[start..end];
    let line = parse_json_line(chunk)?;
    process(line);
    start = end;
}
// tokens[start..] contains incomplete data - wait for more
```

## Design Considerations

### Delimiter Matching

For formats with paired delimiters (JSON, TOML, XML), track nesting depth. A boundary is only valid when all delimiters are balanced.

### String Literals

Newlines inside strings don't affect depth because the lexer treats the entire string as one token. The `ChunkBoundary` operates on tokens, not characters.

### Multiple Boundary Types

Some formats have multiple boundary types. For TOML:

```rust,ignore
fn is_boundary_token(token: &Token) -> bool {
    matches!(token, Token::Newline | Token::TableHeader)
}
```

## Next

[Chapter 3: Incremental Lexer →](03-lexer.md)
