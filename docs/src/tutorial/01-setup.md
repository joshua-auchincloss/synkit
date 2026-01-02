# Project Setup

## Create the Project

```bash
cargo new toml-parser --lib
cd toml-parser
```

## Dependencies

```toml
[package]
name = "toml-parser"
version = "0.1.0"
edition = "2024"

[dependencies]
synkit = "0.1"
thiserror = "2"
logos = "0.15"
```

## Error Type

Define an error type that implements `Default` (required by logos):

```rust,ignore
{{#include ../../../examples/toml-parser/src/lib.rs:error_type}}
```

Key requirements:
- `#[default]` variant for unknown tokens
- `Expected` variant with `expect` and `found` fields
- `Empty` variant for EOF errors
- `Spanned` variant wrapping errors with location

## parser_kit! Invocation

The macro generates all parsing infrastructure:

```rust,ignore
{{#include ../../../examples/toml-parser/src/lib.rs:token_def}}
```

This generates:
- `span` module with `Span`, `Spanned<T>`
- `tokens` module with `Token` enum and `*Token` structs
- `stream` module with `TokenStream`
- `traits` module with `Parse`, `Peek`, `ToTokens`
- `delimiters` module with `Bracket`, `Brace`

## Error Helpers

Add convenience methods for error creation:

```rust,ignore
{{#include ../../../examples/toml-parser/src/lib.rs:error_impl}}
```

## Module Structure

```rust,ignore
// lib.rs
mod ast;
mod parse;
mod print;
mod visitor;

pub use ast::*;
pub use parse::*;
pub use visitor::*;
```

## Verify Setup

```bash
cargo check
```

The macro should expand without errors. If you see errors about missing traits, ensure your error type has the required variants.
