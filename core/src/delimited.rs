use crate::traits::SpanLike;

/// A value enclosed by delimiters (e.g., brackets, braces, parentheses).
///
/// Stores the combined span of the delimiters and the inner content.
/// Use this to represent constructs like `[items]`, `{block}`, or `(expr)`.
///
/// # Type Parameters
///
/// - `T`: The inner content type
/// - `Span`: The span type for source positions
///
/// # Example
///
/// ```ignore
/// // Parse: [1, 2, 3]
/// let bracket = bracket!(inner in stream);
/// // `inner` is a TokenStream of "1, 2, 3"
/// // `bracket` is Delimited with span covering "[...]"
/// let items: Punctuated<Expr, Comma> = inner.parse()?;
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct Delimited<T, Span> {
    /// The span covering the entire delimited region (including delimiters).
    pub span: Span,
    /// The inner content between the delimiters.
    pub inner: T,
}

impl<T, Span: SpanLike> Delimited<T, Span> {
    #[inline]
    pub fn new(span: Span, inner: T) -> Self {
        Self { span, inner }
    }

    #[inline]
    pub fn call_site(inner: T) -> Self {
        Self {
            span: Span::call_site(),
            inner,
        }
    }

    #[inline]
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Delimited<U, Span> {
        Delimited {
            span: self.span,
            inner: f(self.inner),
        }
    }
}

impl<T, Span> std::ops::Deref for Delimited<T, Span> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T, Span> std::ops::DerefMut for Delimited<T, Span> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
