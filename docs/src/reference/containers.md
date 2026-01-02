# Container Types

synkit provides container types for common parsing patterns.

## Punctuated Sequence Types

Three wrapper types for punctuated sequences with different trailing policies:

| Type | Trailing Separator | Use Case |
|------|-------------------|----------|
| `Punctuated<T, P>` | Optional | Array literals: `[1, 2, 3]` or `[1, 2, 3,]` |
| `Separated<T, P>` | Forbidden | Function args: `f(a, b, c)` |
| `Terminated<T, P>` | Required | Statements: `use foo; use bar;` |

### Punctuated

```rust,ignore
use synkit::Punctuated;

// Optional trailing comma
let items: Punctuated<Value, CommaToken> = parse_punctuated(&mut stream)?;

for value in items.iter() {
    process(value);
}

// Check if trailing comma present
if items.trailing_punct() {
    // ...
}
```

### Separated

```rust,ignore
use synkit::Separated;

// Trailing separator is an error
let args: Separated<Arg, CommaToken> = parse_separated(&mut stream)?;
```

### Terminated

```rust,ignore
use synkit::Terminated;

// Each statement must end with separator
let stmts: Terminated<Stmt, SemiToken> = parse_terminated(&mut stream)?;
```

### Common Methods

All three types share these methods via `PunctuatedInner`:

```rust,ignore
fn new() -> Self;
fn with_capacity(capacity: usize) -> Self;
fn push_value(&mut self, value: T);
fn push_punct(&mut self, punct: P);
fn len(&self) -> usize;
fn is_empty(&self) -> bool;
fn iter(&self) -> impl Iterator<Item = &T>;
fn pairs(&self) -> impl Iterator<Item = (&T, Option<&P>)>;
fn first(&self) -> Option<&T>;
fn last(&self) -> Option<&T>;
fn trailing_punct(&self) -> bool;
```

## Repeated

Alternative sequence type preserving separator tokens:

```rust,ignore
use synkit::Repeated;

pub struct Repeated<T, Sep, Spanned> {
    pub values: Vec<RepeatedItem<T, Sep, Spanned>>,
}

pub struct RepeatedItem<T, Sep, Spanned> {
    pub value: Spanned,
    pub sep: Option<Spanned>,
}
```

Use `Repeated` when you need to preserve separator token information (e.g., for source-accurate reprinting).

### Methods

```rust,ignore
fn empty() -> Self;
fn with_capacity(capacity: usize) -> Self;
fn len(&self) -> usize;
fn is_empty(&self) -> bool;
fn iter(&self) -> impl Iterator<Item = &RepeatedItem<...>>;
fn push(&mut self, item: RepeatedItem<...>);
```

## Delimited

Value enclosed by delimiters:

```rust,ignore
use synkit::Delimited;

pub struct Delimited<T, Span> {
    pub span: Span,   // Span covering "[...]" or "{...}"
    pub inner: T,     // The content
}
```

Created automatically by delimiter macros:

```rust,ignore
let mut inner;
let bracket = bracket!(inner in stream);
// bracket.span covers "[" through "]"
// inner is a TokenStream of the contents
```

## Usage Patterns

### Comma-Separated Arguments

```rust,ignore
pub struct FnCall {
    pub name: Spanned<IdentToken>,
    pub paren: Paren,
    pub args: Separated<Expr, CommaToken>,  // No trailing comma
}
```

### Array with Optional Trailing

```rust,ignore
pub struct Array {
    pub bracket: Bracket,
    pub items: Punctuated<Expr, CommaToken>,  // Optional trailing
}
```

### Statement Block

```rust,ignore
pub struct Block {
    pub brace: Brace,
    pub stmts: Terminated<Stmt, SemiToken>,  // Required trailing
}

// Parse arms manually for control
let mut arms = Vec::new();
while Pattern::peek(stream) {
    arms.push(stream.parse::<Spanned<MatchArm>>()?);
}
```

## Printing Containers

```rust,ignore
impl<T: ToTokens, Sep: ToTokens> ToTokens for Repeated<T, Sep, Spanned<T>> {
    fn write(&self, p: &mut Printer) {
        for (i, item) in self.iter().enumerate() {
            if i > 0 {
                // Separator between items
                p.word(", ");
            }
            item.value.write(p);
        }
        // Handle trailing separator if present
        if self.has_trailing() {
            p.word(",");
        }
    }
}
```
