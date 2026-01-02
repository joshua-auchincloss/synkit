use super::stream::SpanLike;

/// Error that can have a span attached.
pub trait SpannedError: Sized {
    type Span: SpanLike;

    /// Wrap with span information.
    fn with_span(self, span: Self::Span) -> Self;

    /// Get span if present.
    fn span(&self) -> Option<&Self::Span>;
}
