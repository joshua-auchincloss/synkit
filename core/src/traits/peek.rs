use super::stream::{SpannedLike, TokenStream};

/// Trait for lookahead without consuming tokens.
///
/// `Peek` enables the parser to inspect the next token to decide which
/// production to use, without consuming it. This is essential for
/// implementing LL(1) and similar parsing strategies.
///
/// # Associated Types
///
/// - `Token`: The token type to peek at (e.g., `MyTok`)
///
/// # Required Methods
///
/// - `is(token)`: Returns `true` if the token matches this type
///
/// # Provided Methods
///
/// - `peek(stream)`: Checks if the next token in the stream matches
///
/// # Example
///
/// ```ignore
/// use synkit::{Peek, TokenStream};
///
/// struct IfKeyword;
///
/// impl Peek for IfKeyword {
///     type Token = MyTok;
///
///     fn is(token: &Self::Token) -> bool {
///         matches!(token, MyTok::If)
///     }
/// }
///
/// // In parser:
/// fn parse_statement(stream: &mut impl TokenStream<Token = MyTok>) {
///     if stream.peek::<IfKeyword>() {
///         parse_if_statement(stream)
///     } else {
///         parse_expression_statement(stream)
///     }
/// }
/// ```
///
/// # Usage Patterns
///
/// ## Simple Token Matching
///
/// ```ignore
/// if stream.peek::<Comma>() {
///     stream.parse::<Comma>()?;
/// }
/// ```
///
/// ## Alternative Productions
///
/// ```ignore
/// if stream.peek::<NumberLiteral>() {
///     Expr::Number(stream.parse()?)
/// } else if stream.peek::<StringLiteral>() {
///     Expr::String(stream.parse()?)
/// } else {
///     return Err(Error::unexpected());
/// }
/// ```
pub trait Peek: Sized {
    /// The token type to peek at.
    type Token: Clone;

    /// Check if a token matches this type.
    ///
    /// # Arguments
    ///
    /// * `token` - The token to check
    ///
    /// # Returns
    ///
    /// `true` if the token matches this type
    fn is(token: &Self::Token) -> bool;

    /// Peek at stream without consuming tokens.
    ///
    /// Default implementation calls `is()` on the next token.
    ///
    /// # Arguments
    ///
    /// * `stream` - The token stream to peek into
    ///
    /// # Returns
    ///
    /// `true` if the next token matches, `false` if no match or stream empty
    #[inline]
    fn peek<S: TokenStream<Token = Self::Token>>(stream: &S) -> bool {
        stream
            .peek_token()
            .map(|t| Self::is(t.value_ref()))
            .unwrap_or(false)
    }
}

impl<T: Peek> Peek for Box<T> {
    type Token = T::Token;

    #[inline]
    fn is(token: &Self::Token) -> bool {
        T::is(token)
    }
}
