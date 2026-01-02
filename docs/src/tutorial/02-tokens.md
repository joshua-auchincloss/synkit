# Defining Tokens

The `tokens:` block in `parser_kit!` defines your grammar's lexical elements.

## Token Categories

### Whitespace Tokens

Skipped during parsing but tracked for round-trip:

```rust,ignore
// Skipped automatically
#[token(" ", priority = 0)]
Space,

#[token("\t", priority = 0)]
Tab,

// Not skipped - we track these for formatting
#[regex(r"\r?\n")]
#[fmt("newline")]
Newline,
```

Use `skip_tokens: [Space, Tab]` to mark tokens to skip.

### Punctuation

Simple exact-match tokens:

```rust,ignore
#[token("=")]
Eq,

#[token(".")]
Dot,

#[token(",")]
Comma,

#[token("[")]
LBracket,

#[token("]")]
RBracket,

#[token("{")]
LBrace,

#[token("}")]
RBrace,
```

### Keywords

Keywords need higher priority than identifiers:

```rust,ignore
#[token("true")]
True,

#[token("false")]
False,
```

### Value Tokens

Tokens with captured data use callbacks:

```rust,ignore
// Bare keys: alphanumeric, underscores, dashes
#[regex(r"[A-Za-z0-9_-]+", |lex| lex.slice().to_string(), priority = 1)]
#[fmt("bare key")]
#[derive(PartialOrd, Ord, Hash, Eq)]
BareKey(String),

// Basic strings (double-quoted)
#[regex(r#""([^"\\]|\\.)*""#, |lex| {
    let s = lex.slice();
    s[1..s.len()-1].to_string()  // Strip quotes
})]
#[fmt("string")]
BasicString(String),

// Integers
#[regex(r"-?[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
#[fmt("integer")]
Integer(i64),
```

### Comments

Track but don't interpret:

```rust,ignore
#[regex(r"#[^\n]*")]
#[fmt("comment")]
Comment,
```

## Generated Types

For each token, synkit generates:

| Token | Struct | Macro |
|-------|--------|-------|
| `Eq` | `EqToken` | `Tok![=]` |
| `Dot` | `DotToken` | `Tok![.]` |
| `BareKey(String)` | `BareKeyToken(String)` | `Tok![bare_key]` |
| `BasicString(String)` | `BasicStringToken(String)` | `Tok![basic_string]` |
| `Integer(i64)` | `IntegerToken(i64)` | `Tok![integer]` |

## Delimiters

Define delimiter pairs for extraction:

```rust,ignore
delimiters: {
    Bracket => (LBracket, RBracket),
    Brace => (LBrace, RBrace),
},
```

Generates `Bracket` and `Brace` structs with span information, plus `bracket!` and `brace!` macros.

## Priority Handling

When patterns overlap, use `priority`:

```rust,ignore
#[token("true", priority = 2)]   // Higher wins
True,

#[regex(r"[A-Za-z]+", priority = 1)]
BareKey(String),
```

Input `"true"` matches `True`, not `BareKey("true")`.

## Derives

Control derives at different levels:

```rust,ignore
// For all tokens
token_derives: [Clone, PartialEq, Debug],

// For specific token
#[derive(Hash, Eq)]  // Additional derives for BareKeyToken only
BareKey(String),

// For span types
span_derives: [Debug, Clone, PartialEq, Eq, Hash],
```
