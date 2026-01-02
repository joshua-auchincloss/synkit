# Synkit vs Kintsu - Feature Gaps Analysis

This document identifies functionality gaps between synkit (the generalized parser toolkit) and kintsu (the original domain-specific parser).

## Architecture Comparison

| Aspect | Synkit | Kintsu | Notes |
|--------|--------|--------|-------|
| **Trait Design** | Generic traits with associated types in `synkit-core`, concrete local traits generated per grammar | Single concrete implementation per trait | Synkit is more flexible/reusable |
| **Error Type** | Configurable via `error:` field | Fixed `LexingError` enum | Synkit allows custom error types |
| **TokenStream** | Trait-based with concrete impl generated | Standalone struct | Synkit allows alternative implementations |
| **Macros** | `Tok!`, `SpannedTok!` (pub crate) | `Token!`, `SpannedToken!` (#[macro_export]) | Different naming, same purpose |

## Current Synkit Features

### Implemented

1. **Core Traits** (in `synkit-core`)
   - `TokenStream` - generic token stream operations
   - `Printer` - code generation/formatting
   - `SpanLike` - span abstraction
   - `SpannedError` - error with span attachment

2. **Generated Code** (via `parser_kit!`)
   - `span` module - `Span`, `RawSpan`, `Spanned<T>`
   - `tokens` module - `Token` enum, `*Token` structs, `Tok!`/`SpannedTok!` macros
   - `stream` module - `TokenStream`, `MutTokenStream`
   - `printer` module - `Printer` implementation
   - `delimiters` module - delimiter structs (e.g., `Bracket`, `Brace`)
   - `traits` module - local `Parse`, `Peek`, `ToTokens`, `Diagnostic` traits

3. **Helper Types** (in `synkit-core`)
   - `Repeated<T, Sep>` - separated sequences with optional trailing
   - `Punctuated<T, P>` - flexible punctuated lists
   - `Terminated<T, P>` - always trailing separator
   - `Separated<T, P>` - never trailing separator
   - `Delimited<T, Span>` - value with delimiter span

4. **Token Generation**
   - Auto-generated `*Token` structs with `new()`, `value()`, `fmt()`
   - **Auto-generated `Parse`, `Peek`, `Diagnostic` implementations for all token structs**
   - Auto-generated `ToTokens` implementations
   - `Tok![=]`, `Tok![struct]`, `Tok![ident]` macro arms

5. **Delimiter Handling**
   - `bracket!`, `brace!` macros for extraction
   - `extract_inner<Open, Close>()` method with nesting support
   - Returns `(TokenStream, Span)` tuple

## Gaps / Missing Features

### 1. Stream Operations

| Feature | Kintsu | Synkit | Priority |
|---------|--------|--------|----------|
| `fork()` with `is_fork` flag | Yes - Prevents mutation logging | Has fork but no flag | Low |
| `peek_unchecked()` | Yes - Raw peek without skip | Has `peek_token_raw()` | Equivalent |
| `collect_source()` | Yes - Gets source span | Missing | Medium |
| Position logging for mutations | Yes - Debug feature | Missing | Low |

### 2. Error Handling

| Feature | Kintsu | Synkit | Priority |
|---------|--------|--------|----------|
| `empty::<T>()` helper | Yes - Type-inferred | Implemented | Done |
| `expected::<T>(found)` helper | Yes - Type-inferred | Implemented | Done |
| `ExpectationFailures` (multiple) | Yes - For union types | Missing | Medium |
| Span attachment chain | Yes - Via `Spanned` variant | Via `with_span()` | Done |

### 3. Parsing Helpers

| Feature | Kintsu | Synkit | Priority |
|---------|--------|--------|----------|
| `parse_spanned()` default impl | Yes - In `Parse` trait | Implemented | Done |
| `Option<T>` blanket impl | Yes - Peeks first | Implemented | Done |
| `Box<T>` blanket impl | Yes - Wraps result | Implemented | Done |
| `Vec<T>` blanket impl | Yes - Parses until fail | Use `Repeated` instead | Alternative |
| `Repeated<T, Sep>` | Yes - Custom type | In synkit-core | Done |

### 4. Macro Completeness

| Feature | Kintsu | Synkit | Priority |
|---------|--------|--------|----------|
| Single-char tokens `[=]` | Yes | Yes | Done |
| Multi-char tokens `[->]` | Yes | Yes (via parse) | Done |
| Keywords `[struct]` | Yes | Yes | Done |
| Regex tokens `[ident]` | Via snake_case | Via snake_case | Done |
| Cross-module usage | `#[macro_export]` | `pub(crate)` only | Design choice |

### 5. Formatting/Printing

| Feature | Kintsu | Synkit | Priority |
|---------|--------|--------|----------|
| `write()` method | In `ToTokens` | In `ToTokens` | Done |
| `display()` convenience | Default impl | `to_string_formatted()` | Done |
| `write_comma_separated()` | Helper method | `write_separated()` | Done |
| `open_block()` / `close_block()` | Indent helpers | Implemented | Done |
| Format config | `FormatConfig` | Builder pattern | Alternative |

### 6. Advanced Features (Not in Synkit)

| Feature | Description | Priority |
|---------|-------------|----------|
| Semantic validation | `is_valid()` checks | Low - app-level |
| Error recovery | Continue parsing after errors | Low - complex |
| Incremental parsing | Reparse only changed parts | Low - complex |
| Source maps | Track original positions through transforms | Low - specialized |

## API Style Differences

### Import Patterns

**Kintsu:**
```rust
use crate::{
    Token, SpannedToken,
    defs::Spanned,
    tokens::{Parse, Peek, ToTokens, ImplDiagnostic, TokenStream, Repeated, brace},
};
```

**Synkit (recommended):**
```rust
use crate::{
    Tok, SpannedTok,  // Note: different macro names
    Spanned, Span,
    Parse, Peek, ToTokens, Diagnostic,
    TokenStream,
    tokens,  // For Token enum and *Token structs
};
use synkit::Repeated;  // Or use crate's re-export
```

### AST Field Patterns

**Kintsu:**
```rust
pub struct Struct {
    pub kw: SpannedToken![struct],
    pub name: SpannedToken![ident],
    pub brace: Brace,
    pub args: Repeated<Arg, Token![,]>,
}
```

**Synkit (equivalent):**
```rust
pub struct Struct {
    pub kw: SpannedTok![struct],
    pub name: SpannedTok![ident],
    pub brace: Brace,
    pub args: synkit::Repeated<Arg, Tok![,], Spanned<Arg>>,
}
```

### Parse Implementation

**Kintsu:**
```rust
impl Parse for Struct {
    fn parse(stream: &mut TokenStream) -> Result<Self, LexingError> {
        let mut braced;
        Ok(Self {
            kw: stream.parse()?,
            name: stream.parse()?,
            brace: brace!(braced in stream),
            args: Repeated::parse(&mut braced)?,
        })
    }
}
```

**Synkit (identical pattern):**
```rust
impl Parse for Struct {
    fn parse(stream: &mut TokenStream) -> Result<Self, TomlError> {
        let mut braced;
        Ok(Self {
            kw: stream.parse()?,
            name: stream.parse()?,
            brace: brace!(braced in stream),
            args: Repeated::parse(&mut braced)?,
        })
    }
}
```

## Recommendations

### High Priority

1. **Re-export macros from crate root** - Add `pub use tokens::{Tok, SpannedTok};` to make imports cleaner
2. **Document the `$crate::` limitation** - Users need to understand why `crate::Tok!` doesn't work in nested modules

### Medium Priority

1. **Add `collect_source()` method** - Useful for extracting source spans
2. **Consider `ExpectationFailures`** - For better error messages on union types

### Low Priority / Design Choices

1. **Macro naming** - `Tok!`/`SpannedTok!` vs `Token!`/`SpannedToken!` - current naming avoids conflict with `Token` enum
2. **`#[macro_export]` vs `pub(crate)`** - Current choice avoids absolute path issues, limits cross-crate usage

## Conclusion

Synkit provides ~95% of kintsu's parsing functionality with a more flexible, trait-based architecture. The main differences are:

1. **Naming**: `Tok!` instead of `Token!` to avoid conflicts
2. **Scope**: `pub(crate)` macros instead of `#[macro_export]`
3. **Generics**: Helper types use generic spans for flexibility

The toml-parser example should be refactored to use kintsu-style patterns where the generated traits and macros work seamlessly.
