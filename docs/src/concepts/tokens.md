# Tokens

synkit generates two representations for each token: an **enum variant** and a **struct**.

## Token Enum

The `Token` enum contains all token variants, used by the lexer:

```rust,ignore
#[derive(Logos, Debug, Clone, PartialEq)]
pub enum Token {
    #[token("=")]
    Eq,

    #[regex(r"[a-z]+", |lex| lex.slice().to_string())]
    Ident(String),

    #[regex(r"[0-9]+", |lex| lex.slice().parse().ok())]
    Number(i64),
}
```

## Token Structs

For each variant, synkit generates a corresponding struct:

```rust,ignore
// Unit token (no value)
pub struct EqToken;

impl EqToken {
    pub fn new() -> Self { Self }
    pub fn token(&self) -> Token { Token::Eq }
}

// Token with value
pub struct IdentToken(pub String);

impl IdentToken {
    pub fn new(value: String) -> Self { Self(value) }
    pub fn token(&self) -> Token { Token::Ident(self.0.clone()) }
}

impl std::ops::Deref for IdentToken {
    type Target = String;
    fn deref(&self) -> &Self::Target { &self.0 }
}
```

## Token Attributes

### `#[token(...)]` and `#[regex(...)]`

Standard logos attributes for matching:

```rust,ignore
#[token("=")]           // Exact match
#[regex(r"[a-z]+")]     // Regex pattern
#[regex(r"[0-9]+", |lex| lex.slice().parse().ok())]  // With callback
```

### `#[fmt(...)]`

Custom display name for error messages:

```rust,ignore
#[regex(r"[a-z]+", |lex| lex.slice().to_string())]
#[fmt("identifier")]  // Error: "expected identifier, found ..."
Ident(String),
```

Without `#[fmt]`, uses the variant name in snake_case.

### `#[derive(...)]` on tokens

Additional derives for a specific token struct:

```rust,ignore
#[regex(r"[A-Za-z_]+", |lex| lex.slice().to_string())]
#[derive(Hash, Eq)]  // Only for IdentToken
Ident(String),
```

### `priority`

Logos priority for overlapping patterns:

```rust,ignore
#[token("true", priority = 2)]  // Higher priority than bare keys
True,

#[regex(r"[A-Za-z]+", priority = 1)]
BareKey(String),
```

## The `Tok!` Macro

Access token types by their pattern:

```rust,ignore
// Punctuation - use the literal
Tok![=]     // → EqToken
Tok![.]     // → DotToken
Tok![,]     // → CommaToken

// Keywords - use the keyword
Tok![true]  // → TrueToken
Tok![false] // → FalseToken

// Regex tokens - use snake_case name
Tok![ident]   // → IdentToken
Tok![number]  // → NumberToken
```

## `SpannedTok!`

Shorthand for `Spanned<Tok![...]>`:

```rust,ignore
SpannedTok![=]      // → Spanned<EqToken>
SpannedTok![ident]  // → Spanned<IdentToken>
```

## Auto-generated Trait Implementations

Each token struct automatically implements:

| Trait | Purpose |
|-------|---------|
| `Parse` | Parse from TokenStream |
| `Peek` | Check if token matches without consuming |
| `Diagnostic` | Format name for error messages |
| `Display` | Human-readable output |
