# AST Design

Design AST nodes that preserve all information for round-trip formatting.

## Design Principles

1. **Use `Spanned<T>` for all children** - Enables error locations and source mapping
2. **Include punctuation tokens** - Needed for exact round-trip
3. **Track trivia** - Comments and newlines for formatting

## Document Structure

```rust,ignore
{{#include ../../../examples/toml-parser/src/ast.rs:document}}
```

- `Document` is the root containing all items
- `DocumentItem` distinguishes top-level elements
- `Trivia` captures non-semantic content

## Keys

```rust,ignore
{{#include ../../../examples/toml-parser/src/ast.rs:key}}
```

- `Key` enum handles all key forms
- `DottedKey` preserves dot tokens for round-trip
- `SimpleKey` is the base case (bare or quoted)

## Values

```rust,ignore
{{#include ../../../examples/toml-parser/src/ast.rs:value}}
```

Each variant stores its token type directly, preserving the original representation.

## Key-Value Pairs

```rust,ignore
{{#include ../../../examples/toml-parser/src/ast.rs:key_value}}
```

Note how `eq` stores the equals token—this enables formatting choices like `key=value` vs `key = value`.

## Tables

```rust,ignore
{{#include ../../../examples/toml-parser/src/ast.rs:table}}
```

- Brackets stored explicitly for round-trip
- Items include trivia for blank lines/comments within table

## Arrays

```rust,ignore
{{#include ../../../examples/toml-parser/src/ast.rs:array}}
```

`ArrayItem` includes optional trailing comma—essential for preserving:
```toml
[1, 2, 3]     # No trailing comma
[1, 2, 3,]    # With trailing comma
```

## Inline Tables

```rust,ignore
{{#include ../../../examples/toml-parser/src/ast.rs:inline_table}}
```

Similar structure to arrays, with key-value pairs instead of values.

## Why This Design?

### Span Preservation

Every `Spanned<T>` carries source location:

```rust,ignore
let kv: Spanned<KeyValue> = stream.parse()?;
let key_span = &kv.value.key.span;  // Location of key
let eq_span = &kv.value.eq.span;    // Location of '='
let val_span = &kv.value.value.span; // Location of value
```

### Round-trip Fidelity

Storing tokens enables exact reconstruction:

```rust,ignore
// Original: key = "value"
// After parse → print:
//   key = "value"  (identical)
```

### Trivia Handling

Without trivia tracking:
```toml
# Comment lost!
key = value
```

With trivia in AST:
```toml
# Comment preserved
key = value
```
