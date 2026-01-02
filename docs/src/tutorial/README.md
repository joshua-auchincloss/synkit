# Tutorial: TOML Parser

Build a complete TOML parser with round-trip printing using synkit.

## Source Code

> **📦 Complete source**: [examples/toml-parser](https://github.com/joshua-auchincloss/synkit/tree/main/examples/toml-parser)

## What You'll Build

A parser for a TOML subset supporting:

```toml
# Comment
key = "value"
number = 42
flag = true

[section]
nested = "data"

[section.subsection]
array = [1, 2, 3]
inline = { a = 1, b = 2 }
```

## Source Code

The complete example lives in `examples/toml-parser/`. Each chapter references the actual code.

## Chapters

1. **[Project Setup](01-setup.md)** - Dependencies, error type, `parser_kit!` invocation
2. **[Defining Tokens](02-tokens.md)** - Token patterns and attributes
3. **[AST Design](03-ast.md)** - Node types with `Spanned<T>`
4. **[Parse Implementations](04-parse.md)** - Converting tokens to AST
5. **[Round-trip Printing](05-printing.md)** - `ToTokens` for output
6. **[Visitors](06-visitors.md)** - Traversing the AST
7. **[Testing](07-testing.md)** - Parse and round-trip tests
