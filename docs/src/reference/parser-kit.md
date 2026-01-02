# parser_kit! Macro

The `parser_kit!` macro generates parsing infrastructure from token definitions.

## Syntax

```rust,ignore
synkit::parser_kit! {
    error: ErrorType,
    skip_tokens: [Token1, Token2],

    #[logos(skip r"...")] // Optional logos-level attributes
    tokens: {
        #[token("=")]
        Eq,

        #[regex(r"[a-z]+", |lex| lex.slice().to_string())]
        #[fmt("identifier")]
        Ident(String),
    },

    delimiters: {
        Bracket => (LBracket, RBracket),
    },

    span_derives: [Debug, Clone, PartialEq],
    token_derives: [Debug, Clone, PartialEq],
    custom_derives: [],
}
```

## Fields

### `error: ErrorType` (required)

Your error type. Must implement `Default`:

```rust,ignore
#[derive(Default)]
pub enum MyError {
    #[default]
    Unknown,
    // ...
}
```

### `skip_tokens: [...]` (required)

Tokens to skip during parsing. Typically whitespace:

```rust,ignore
skip_tokens: [Space, Tab],
```

Skipped tokens don't appear in `stream.next()` but are visible in `stream.next_raw()`.

### `tokens: { ... }` (required)

Token definitions using logos attributes.

#### Unit Tokens

```rust,ignore
#[token("=")]
Eq,
```

Generates `EqToken` with `new()` and `token()` methods.

#### Tokens with Values

```rust,ignore
#[regex(r"[a-z]+", |lex| lex.slice().to_string())]
Ident(String),
```

Generates `IdentToken(String)` implementing `Deref<Target=String>`.

#### Token Attributes

| Attribute | Purpose |
|-----------|---------|
| `#[token("...")]` | Exact string match |
| `#[regex(r"...")]` | Regex pattern |
| `#[regex(r"...", callback)]` | Regex with value extraction |
| `#[fmt("name")]` | Display name for errors |
| `#[derive(...)]` | Additional derives for this token |
| `priority = N` | Logos priority for conflicts |

### `delimiters: { ... }` (optional)

Delimiter pair definitions:

```rust,ignore
delimiters: {
    Bracket => (LBracket, RBracket),
    Brace => (LBrace, RBrace),
    Paren => (LParen, RParen),
},
```

Generates:
- Struct (e.g., `Bracket`) storing spans
- Macro (e.g., `bracket!`) for extraction

### `span_derives: [...]` (optional)

Derives for `Span`, `RawSpan`, `Spanned<T>`:

```rust,ignore
span_derives: [Debug, Clone, PartialEq, Eq, Hash],
```

Default: `Debug, Clone, PartialEq, Eq, Hash`

### `token_derives: [...]` (optional)

Derives for all token structs:

```rust,ignore
token_derives: [Debug, Clone, PartialEq],
```

### `custom_derives: [...]` (optional)

Additional derives for all generated types:

```rust,ignore
custom_derives: [serde::Serialize],
```

## Generated Modules

### `span`

```rust,ignore
pub struct RawSpan { pub start: usize, pub end: usize }
pub enum Span { CallSite, Known(RawSpan) }
pub struct Spanned<T> { pub value: T, pub span: Span }
```

### `tokens`

```rust,ignore
pub enum Token { Eq, Ident(String), ... }
pub struct EqToken;
pub struct IdentToken(pub String);

// Macros
macro_rules! Tok { ... }
macro_rules! SpannedTok { ... }
```

### `stream`

```rust,ignore
pub struct TokenStream { ... }
pub struct MutTokenStream<'a> { ... }

impl TokenStream {
    pub fn lex(source: &str) -> Result<Self, Error>;
    pub fn parse<T: Parse>(&mut self) -> Result<Spanned<T>, Error>;
    pub fn peek<T: Peek>(&self) -> bool;
    pub fn fork(&self) -> Self;
    pub fn advance_to(&mut self, other: &Self);
}
```

### `printer`

```rust,ignore
pub struct Printer { ... }

impl Printer {
    pub fn new() -> Self;
    pub fn finish(self) -> String;
    pub fn word(&mut self, s: &str);
    pub fn token(&mut self, tok: &Token);
    pub fn space(&mut self);
    pub fn newline(&mut self);
    pub fn open_block(&mut self);
    pub fn close_block(&mut self);
}
```

### `delimiters`

For each delimiter definition:

```rust,ignore
pub struct Bracket { pub span: Span }

macro_rules! bracket {
    ($inner:ident in $stream:expr) => { ... }
}
```

### `traits`

```rust,ignore
pub trait Parse: Sized {
    fn parse(stream: &mut TokenStream) -> Result<Self, Error>;
}

pub trait Peek {
    fn is(token: &Token) -> bool;
    fn peek(stream: &TokenStream) -> bool;
}

pub trait ToTokens {
    fn write(&self, printer: &mut Printer);
    fn to_string_formatted(&self) -> String;
}

pub trait Diagnostic {
    fn fmt() -> &'static str;
}
```

## Expansion Example

Input:
```rust,ignore
synkit::parser_kit! {
    error: E,
    skip_tokens: [],
    tokens: {
        #[token("=")]
        Eq,
    },
    delimiters: {},
    span_derives: [Debug],
    token_derives: [Debug],
}
```

Expands to ~500 lines including all modules, traits, and implementations.
