#![deny(
    unsafe_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::dbg_macro
)]

//! Procedural macros for the synkit parsing toolkit.
//!
//! This crate provides two main macros:
//!
//! - [`declare_tokens!`]: Generates a token enum compatible with Logos
//! - [`parser_kit!`]: Generates a complete parser infrastructure
//!
//! # Quick Start
//!
//! ```ignore
//! use synkit_macros::parser_kit;
//!
//! parser_kit! {
//!     error: MyParseError,
//!     skip_tokens: [Whitespace, Comment],
//!     tokens: {
//!         Whitespace => r"[ \t\n\r]+",
//!         Number => r"[0-9]+",
//!         Ident => r"[a-zA-Z_][a-zA-Z0-9_]*",
//!     },
//! }
//! ```
use proc_macro::TokenStream;
use syn::parse_macro_input;

mod declare_tokens;
mod parser_kit;

/// Generates a token enum with Logos lexer integration.
///
/// This macro creates a token enum that implements the Logos trait for
/// efficient lexical analysis. Each variant can specify a regex pattern
/// or a literal string.
///
/// # Syntax
///
/// ```ignore
/// declare_tokens! {
///     error: ErrorType,
///     tokens: {
///         // Literal pattern
///         Plus => "+",
///         // Regex pattern
///         Number => r"[0-9]+",
///         // Skip pattern (not emitted as token)
///         #[skip]
///         Whitespace => r"[ \t\n]+",
///     },
/// }
/// ```
///
/// # Generated Code
///
/// The macro generates:
/// - A `Tok` enum with variants for each token
/// - Logos derive implementation for lexing
/// - `Display` implementation for error messages
///
/// # Example
///
/// ```ignore
/// use logos::Logos;
///
/// declare_tokens! {
///     error: LexError,
///     tokens: {
///         #[token("+")]
///         Plus,
///         #[token("-")]
///         Minus,
///         #[regex(r"[0-9]+")]
///         Number,
///     },
/// }
///
/// let mut lexer = Tok::lexer("1 + 2");
/// assert_eq!(lexer.next(), Some(Ok(Tok::Number)));
/// ```
#[proc_macro]
pub fn declare_tokens(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as declare_tokens::DeclareTokensInput);
    declare_tokens::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Generates a complete parser infrastructure from a token specification.
///
/// This is the primary macro for creating a synkit parser. It generates:
/// - Token enum with Logos lexer (`Tok`)
/// - Span and Spanned types (`span::Span`, `span::Spanned<T>`)
/// - Token stream with full synkit trait implementations (`TokenStream`)
/// - Delimiter types for bracket matching
/// - Trait implementations: `Parse`, `Peek`, `ToTokens`, etc.
///
/// # Syntax
///
/// ```ignore
/// parser_kit! {
///     // Required: error type for parse failures
///     error: MyParseError,
///
///     // Optional: tokens to skip during parsing (usually whitespace)
///     skip_tokens: [Whitespace, Comment],
///
///     // Optional: Logos attributes applied to the token enum
///     #[logos(skip r"[ \t]+")]
///
///     // Required: token definitions
///     tokens: {
///         // Literal tokens
///         Plus => "+",
///         Minus => "-",
///
///         // Regex tokens
///         Number => r"[0-9]+",
///         Ident => r"[a-zA-Z_][a-zA-Z0-9_]*",
///     },
///
///     // Optional: delimiter pairs for bracket matching
///     delimiters: {
///         Paren => (LParen, RParen),
///         Bracket => (LBracket, RBracket),
///     },
///
///     // Optional: custom derives for span types
///     span_derives: [serde::Serialize, serde::Deserialize],
///
///     // Optional: custom derives for token types
///     token_derives: [serde::Serialize],
/// }
/// ```
///
/// # Generated Modules and Types
///
/// ## `span` module
///
/// - `RawSpan`: Simple start/end byte offsets
/// - `Span`: Enum with `CallSite` and `Known(RawSpan)` variants
/// - `Spanned<T>`: Value with associated span
///
/// ## `tokens` module
///
/// - `Tok`: Main token enum with Logos derive
/// - `SpannedTok`: Alias for `Spanned<Tok>`
///
/// ## `stream` module
///
/// - `TokenStream`: Main parsing stream implementing `synkit::TokenStream`
///
/// ## `traits` module
///
/// Re-exports of synkit traits for convenience:
/// - `Parse`, `Peek`, `ToTokens`, `Printer`
/// - `SpanLike`, `SpannedLike`, `TokenStream`
///
/// # Token Stream Methods
///
/// The generated `TokenStream` provides:
///
/// - `new(source: &str)` - Create from source string
/// - `peek_token()` / `next()` - Read tokens (skipping configured skip_tokens)
/// - `peek::<T>()` - Check if next token matches type
/// - `parse::<T>()` - Parse a value implementing `Parse`
/// - `fork()` - Create a lookahead copy
/// - `rewind(pos)` - Reset to previous position (clamped to valid range)
/// - `cursor_span()` / `last_span()` - Get current/last token spans
/// - `ensure_consumed()` - Verify no tokens remain
///
/// # Example
///
/// ```ignore
/// parser_kit! {
///     error: CalcError,
///     skip_tokens: [Whitespace],
///     tokens: {
///         Whitespace => r"[ \t\n]+",
///         Number => r"[0-9]+",
///         Plus => "+",
///         Minus => "-",
///     },
/// }
///
/// fn parse_expr(input: &str) -> Result<i64, CalcError> {
///     let mut stream = stream::TokenStream::new(input);
///
///     // Parse first number
///     let num: Number = stream.parse()?;
///     let mut result = num.value.parse::<i64>().unwrap();
///
///     // Parse operations
///     while stream.peek::<Plus>() || stream.peek::<Minus>() {
///         if stream.peek::<Plus>() {
///             let _: Plus = stream.parse()?;
///             let num: Number = stream.parse()?;
///             result += num.value.parse::<i64>().unwrap();
///         } else {
///             let _: Minus = stream.parse()?;
///             let num: Number = stream.parse()?;
///             result -= num.value.parse::<i64>().unwrap();
///         }
///     }
///
///     stream.ensure_consumed()?;
///     Ok(result)
/// }
/// ```
///
/// # Delimiter Matching
///
/// When `delimiters` is specified, the macro generates types that track
/// matched pairs of brackets:
///
/// ```ignore
/// parser_kit! {
///     error: ParseError,
///     tokens: {
///         LParen => "(",
///         RParen => ")",
///     },
///     delimiters: {
///         Paren => (LParen, RParen),
///     },
/// }
///
/// // Use in parser:
/// let (open, inner, close) = stream.parse::<Paren<Expr>>()?;
/// ```
#[proc_macro]
pub fn parser_kit(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as parser_kit::ParserKitInput);
    parser_kit::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
