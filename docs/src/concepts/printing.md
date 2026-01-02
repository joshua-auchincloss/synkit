# Printing

The `ToTokens` trait enables round-trip formatting: parse source, modify AST, print back.

## The `ToTokens` Trait

```rust,ignore
pub trait ToTokens {
    fn write(&self, printer: &mut Printer);
}
```

Implement for each AST node:

```rust,ignore
impl ToTokens for KeyValue {
    fn write(&self, p: &mut Printer) {
        self.key.value.write(p);
        p.space();
        self.eq.value.write(p);
        p.space();
        self.value.value.write(p);
    }
}
```

## Printer Methods

### Basic Output

```rust,ignore
p.word("text");         // Append literal text
p.token(&tok);          // Append token's string form
p.space();              // Single space
p.newline();            // Line break
```

### Indentation

```rust,ignore
p.open_block();         // Increase indent, add newline
p.close_block();        // Decrease indent, add newline
p.indent();             // Just increase indent level
p.dedent();             // Just decrease indent level
```

### Separators

```rust,ignore
// Write items with separator
p.write_separated(&items, ", ");

// Write with custom logic
for (i, item) in items.iter().enumerate() {
    if i > 0 { p.word(", "); }
    item.write(p);
}
```

## Converting to String

```rust,ignore
// Using the trait method
let output = kv.to_string_formatted();

// Manual printer usage
let mut printer = Printer::new();
kv.write(&mut printer);
let output = printer.finish();
```

## Round-trip Example

```rust,ignore
// Parse
let mut stream = TokenStream::lex("key = 42")?;
let kv: Spanned<KeyValue> = stream.parse()?;

// Modify
let mut modified = kv.value.clone();
modified.value = Spanned {
    value: Value::Integer(IntegerToken::new(100)),
    span: Span::CallSite,
};

// Print
let output = modified.to_string_formatted();
assert_eq!(output, "key = 100");
```

## Implementation Patterns

### Token Structs

Token structs need explicit `ToTokens`:

```rust,ignore
impl ToTokens for EqToken {
    fn write(&self, p: &mut Printer) {
        p.token(&self.token());
    }
}

impl ToTokens for BasicStringToken {
    fn write(&self, p: &mut Printer) {
        // Re-add quotes stripped during lexing
        p.word("\"");
        p.word(&self.0);
        p.word("\"");
    }
}
```

### Enum Variants

```rust,ignore
impl ToTokens for Value {
    fn write(&self, p: &mut Printer) {
        match self {
            Value::String(s) => s.write(p),
            Value::Integer(n) => n.write(p),
            Value::True(t) => t.write(p),
            Value::False(f) => f.write(p),
            Value::Array(a) => a.write(p),
            Value::InlineTable(t) => t.write(p),
        }
    }
}
```

### Collections

```rust,ignore
impl ToTokens for Array {
    fn write(&self, p: &mut Printer) {
        self.lbracket.value.write(p);
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 { p.word(", "); }
            item.value.value.write(p);
        }
        self.rbracket.value.write(p);
    }
}
```

### Preserving Trivia

For exact round-trip, preserve comments and whitespace:

```rust,ignore
impl ToTokens for Trivia {
    fn write(&self, p: &mut Printer) {
        match self {
            Trivia::Newline(_) => p.newline(),
            Trivia::Comment(c) => {
                p.token(&c.value.token());
            }
        }
    }
}
```
