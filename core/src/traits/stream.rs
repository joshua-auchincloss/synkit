use super::parse::Parse;
use super::peek::Peek;
use crate::Error;

/// A span representing a source location range.
///
/// Spans track the byte offsets of tokens and AST nodes within source text.
/// All implementations must be `Clone` to support parser backtracking.
pub trait SpanLike: Clone {
    /// Returns the start byte offset.
    fn start(&self) -> usize;

    /// Returns the end byte offset (exclusive).
    fn end(&self) -> usize;

    /// Creates a new span from start and end offsets.
    fn new(start: usize, end: usize) -> Self;

    /// Returns a synthetic span for generated code.
    fn call_site() -> Self;

    /// Returns the length of this span.
    ///
    /// # Clamping Behavior
    ///
    /// Uses saturating subtraction to compute `end - start`. If `end < start`
    /// (an inverted span), this returns `0` rather than panicking or wrapping.
    /// This ensures safe handling of malformed or sentinel span values.
    #[inline]
    fn len(&self) -> usize {
        self.end().saturating_sub(self.start())
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Join two spans into one covering both regions.
    ///
    /// # Clamping Behavior
    ///
    /// Uses `min()` for start and `max()` for end positions. No overflow checking
    /// is performed since `min`/`max` operations cannot overflow. The result
    /// spans from the earliest start to the latest end, regardless of whether
    /// the input spans are inverted or disjoint.
    #[inline]
    fn join(&self, other: &Self) -> Self {
        Self::new(self.start().min(other.start()), self.end().max(other.end()))
    }
}

/// A value paired with its source location span.
///
/// Wraps any value `T` with span information for error reporting and
/// source mapping. Implementations must be `Clone` to support backtracking.
pub trait SpannedLike<T> {
    /// The span type used to track source locations.
    type Span: SpanLike + Copy;

    /// Returns a reference to the span.
    fn span(&self) -> &Self::Span;

    /// Returns a reference to the wrapped value.
    fn value_ref(&self) -> &T;

    /// Consumes self and returns the wrapped value.
    fn value(self) -> T;

    /// Creates a new spanned value from offsets and a value.
    fn new(start: usize, end: usize, value: T) -> Self;

    /// Maps the inner value while preserving the span.
    #[inline]
    fn map<U: Clone, F: FnOnce(T) -> U>(self, f: F) -> impl SpannedLike<U, Span = Self::Span>
    where
        Self: Sized,
    {
        let span = *self.span();
        MappedSpanned {
            span,
            value: f(self.value()),
        }
    }
}

#[derive(Clone)]
struct MappedSpanned<T, S> {
    span: S,
    value: T,
}

impl<T: Clone, S: SpanLike + Copy> SpannedLike<T> for MappedSpanned<T, S> {
    type Span = S;

    #[inline]
    fn span(&self) -> &Self::Span {
        &self.span
    }

    #[inline]
    fn value_ref(&self) -> &T {
        &self.value
    }

    #[inline]
    fn value(self) -> T {
        self.value
    }

    #[inline]
    fn new(start: usize, end: usize, value: T) -> Self {
        Self {
            span: S::new(start, end),
            value,
        }
    }
}

/// A stream of tokens for parsing.
///
/// Provides the core interface for lexer output consumption. Token streams
/// support peeking, consumption, forking for lookahead, and rewinding.
pub trait TokenStream: Sized {
    /// The token type produced by the lexer.
    type Token: Clone;

    /// The span type for tracking source locations.
    type Span: SpanLike;

    /// A spanned wrapper type for associating values with spans.
    type Spanned<T: Clone>: SpannedLike<T, Span = Self::Span>;

    /// Peeks at the next token without consuming (includes whitespace).
    fn peek_token_raw(&self) -> Option<&Self::Spanned<Self::Token>>;

    /// Consumes and returns the next token (includes whitespace).
    fn next_raw(&mut self) -> Option<Self::Spanned<Self::Token>>;

    /// Returns the current cursor position.
    fn cursor(&self) -> usize;

    /// Rewinds to a previous cursor position.
    fn rewind(&mut self, pos: usize);

    /// Creates a fork for lookahead without consuming tokens.
    fn fork(&self) -> Self;

    /// Returns the span at the current cursor position.
    fn cursor_span(&self) -> Option<Self::Span>;

    /// Returns the span of the last consumed token.
    fn last_span(&self) -> Option<Self::Span>;

    /// Peeks at the next significant token (skips whitespace by default).
    #[inline]
    fn peek_token(&self) -> Option<&Self::Spanned<Self::Token>> {
        self.peek_token_raw()
    }

    /// Consumes and returns the next significant token.
    #[inline]
    fn next(&mut self) -> Option<Self::Spanned<Self::Token>> {
        self.next_raw()
    }

    /// Checks if the next token matches type `T` without consuming.
    #[inline]
    fn peek<T: Peek<Token = Self::Token>>(&self) -> bool {
        T::peek(self)
    }

    /// Parses a value of type `T` from the stream.
    #[inline]
    fn parse<T: Parse<Token = Self::Token>>(&mut self) -> Result<T, T::Error> {
        T::parse(self)
    }

    /// Parses a value and wraps it with its source span.
    fn parse_spanned<T: Parse<Token = Self::Token> + Clone>(
        &mut self,
    ) -> Result<Self::Spanned<T>, T::Error> {
        let start = self.cursor_span().unwrap_or_else(Self::Span::call_site);
        let value = T::parse(self)?;
        let end = self.last_span().unwrap_or_else(Self::Span::call_site);
        Ok(Self::Spanned::new(start.start(), end.end(), value))
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.peek_token().is_none()
    }

    /// Returns the number of remaining significant tokens (excluding whitespace).
    ///
    /// The default implementation counts tokens via `peek_token()` and `next()`,
    /// which may be inefficient. Implementations may override this for better performance.
    fn remaining(&self) -> usize {
        let mut count = 0;
        let mut fork = self.fork();
        while fork.next().is_some() {
            count += 1;
        }
        count
    }

    /// Ensures the stream has been fully consumed.
    ///
    /// Returns `Ok(())` if no significant tokens remain (whitespace is ignored).
    /// Returns `Err(Error::StreamNotConsumed)` if tokens remain.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let doc: Document = stream.parse()?;
    /// stream.ensure_consumed()?; // Error if trailing garbage
    /// ```
    #[inline]
    fn ensure_consumed(&self) -> Result<(), Error> {
        let remaining = self.remaining();
        if remaining > 0 {
            Err(Error::StreamNotConsumed { remaining })
        } else {
            Ok(())
        }
    }

    /// Create a span covering a range of cursor positions.
    ///
    /// This is useful for tracking the span of a parsed AST node that
    /// spans multiple tokens.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let start = stream.cursor();
    /// // ... parse multiple tokens ...
    /// let end = stream.cursor();
    /// let span = stream.span_range(start..end);
    /// ```
    #[inline]
    fn span_range(&self, range: core::ops::Range<usize>) -> Self::Span {
        Self::Span::new(range.start, range.end)
    }

    /// Get the span of a token at a specific cursor position.
    ///
    /// Returns `None` if the position is out of bounds.
    fn span_at(&self, pos: usize) -> Option<Self::Span>;
}
