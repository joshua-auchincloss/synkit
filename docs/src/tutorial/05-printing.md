# Round-trip Printing

Implement `ToTokens` to convert AST back to formatted output.

## Basic Pattern

```rust,ignore
impl ToTokens for MyNode {
    fn write(&self, p: &mut Printer) {
        self.child1.value.write(p);
        p.space();
        self.child2.value.write(p);
    }
}
```

## Token Printing

```rust,ignore
{{#include ../../../examples/toml-parser/src/print.rs:print_tokens}}
```

Note: `BasicStringToken` strips quotes during lexing, so we re-add them for output.

## Trivia

Preserve newlines and comments:

```rust,ignore
{{#include ../../../examples/toml-parser/src/print.rs:print_trivia}}
```

## Key-Value Pairs

```rust,ignore
{{#include ../../../examples/toml-parser/src/print.rs:print_key_value}}
```

Spacing around `=` is a style choice—adjust as needed.

## Arrays

Handle items with optional trailing commas:

```rust,ignore
{{#include ../../../examples/toml-parser/src/print.rs:print_array}}
```

## Tables

```rust,ignore
{{#include ../../../examples/toml-parser/src/print.rs:print_table}}
```

## Documents

```rust,ignore
{{#include ../../../examples/toml-parser/src/print.rs:print_document}}
```

## Using the Output

```rust,ignore
// Parse
let mut stream = TokenStream::lex(input)?;
let doc: Spanned<Document> = stream.parse()?;

// Print using trait method
let output = doc.value.to_string_formatted();

// Or manual printer
let mut printer = Printer::new();
doc.value.write(&mut printer);
let output = printer.finish();
```

## Printer Methods Reference

| Method | Effect |
|--------|--------|
| `word(s)` | Append string |
| `token(&tok)` | Append token's display |
| `space()` | Single space |
| `newline()` | Line break |
| `open_block()` | Indent + newline |
| `close_block()` | Dedent + newline |
| `indent()` | Increase indent |
| `dedent()` | Decrease indent |
| `write_separated(&items, sep)` | Items with separator |

## Formatting Choices

The `ToTokens` implementation defines your output format:

```rust,ignore
// Compact: key=value
self.key.value.write(p);
self.eq.value.write(p);
self.value.value.write(p);

// Spaced: key = value
self.key.value.write(p);
p.space();
self.eq.value.write(p);
p.space();
self.value.value.write(p);
```

For exact round-trip, store original spacing as trivia. For normalized output, apply consistent rules in `write()`.
