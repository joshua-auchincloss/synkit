# Concepts Overview

This section covers the core concepts in synkit:

- **[Tokens](tokens.md)** - Token enum, token structs, and the `Tok!` macro
- **[Parsing](parsing.md)** - `Parse` and `Peek` traits, stream operations
- **[Spans & Errors](spans.md)** - Source locations, `Spanned<T>`, error handling
- **[Printing](printing.md)** - `ToTokens` trait and round-trip formatting

## Core Flow

```text
Source → Lexer → TokenStream → Parse → AST → ToTokens → Output
```

1. **Lexer** (logos): Converts source string to token sequence
2. **TokenStream**: Wraps tokens with span tracking and skip logic
3. **Parse**: Trait for converting tokens to AST nodes
4. **AST**: Your domain-specific tree structure
5. **ToTokens**: Trait for converting AST back to formatted output
