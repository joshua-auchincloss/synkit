use super::peek::Peek;
use super::stream::TokenStream;

/// Trait for types that can be parsed from a token stream.
///
/// This is the primary trait for defining how AST nodes are constructed
/// from tokens. Implementations consume tokens from the stream and
/// return either a parsed value or an error.
///
/// # Associated Types
///
/// - `Token`: The token type this parser consumes (e.g., `MyTok`)
/// - `Error`: The error type for parse failures (e.g., `MyParseError`)
///
/// # Implementation Guidelines
///
/// 1. Use `stream.peek::<T>()` to check token types without consuming
/// 2. Use `stream.parse::<T>()` to recursively parse nested structures
/// 3. Use `stream.fork()` for backtracking lookahead
/// 4. Return errors early with `?` operator
///
/// # Example
///
/// ```ignore
/// use synkit::{Parse, Peek, TokenStream};
///
/// struct BinaryExpr {
///     left: Box<Expr>,
///     op: Operator,
///     right: Box<Expr>,
/// }
///
/// impl Parse for BinaryExpr {
///     type Token = MyTok;
///     type Error = ParseError;
///
///     fn parse<S>(stream: &mut S) -> Result<Self, Self::Error>
///     where
///         S: TokenStream<Token = Self::Token>,
///     {
///         let left = stream.parse()?;
///         let op = stream.parse()?;
///         let right = stream.parse()?;
///         Ok(BinaryExpr { left, op, right })
///     }
/// }
/// ```
///
/// # Blanket Implementations
///
/// - `Option<T>`: Parses `Some(T)` if `T::peek()` succeeds, else `None`
/// - `Box<T>`: Delegates to `T::parse()` and boxes the result
pub trait Parse: Sized {
    /// The token type consumed by this parser.
    type Token: Clone;
    /// The error type for parse failures.
    type Error;

    /// Parse a value from the token stream.
    ///
    /// # Arguments
    ///
    /// * `stream` - The token stream to parse from
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - Successfully parsed value
    /// * `Err(Self::Error)` - Parse failure with error details
    fn parse<S>(stream: &mut S) -> Result<Self, Self::Error>
    where
        S: TokenStream<Token = Self::Token>;
}

impl<T> Parse for Option<T>
where
    T: Parse + Peek<Token = <T as Parse>::Token>,
{
    type Token = <T as Parse>::Token;
    type Error = T::Error;

    fn parse<S>(stream: &mut S) -> Result<Self, Self::Error>
    where
        S: TokenStream<Token = Self::Token>,
    {
        if stream.peek::<T>() {
            Ok(Some(T::parse(stream)?))
        } else {
            Ok(None)
        }
    }
}

impl<T: Parse> Parse for Box<T> {
    type Token = T::Token;
    type Error = T::Error;

    fn parse<S>(stream: &mut S) -> Result<Self, Self::Error>
    where
        S: TokenStream<Token = Self::Token>,
    {
        Ok(Box::new(T::parse(stream)?))
    }
}
