//! Edge case tests for synkit core functionality.
use synkit::{SpanLike, SpannedLike};

/// Minimal span implementation for testing SpanLike trait behavior.
#[derive(Clone, Debug, PartialEq)]
struct TestSpan {
    start: usize,
    end: usize,
}

impl SpanLike for TestSpan {
    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }

    fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    fn call_site() -> Self {
        Self { start: 0, end: 0 }
    }
}

/// Minimal spanned wrapper for testing SpannedLike trait behavior.
#[derive(Clone, Debug)]
struct TestSpanned<T> {
    span: TestSpan,
    value: T,
}

impl<T: Clone> SpannedLike<T> for TestSpanned<T> {
    type Span = TestSpan;

    fn span(&self) -> &Self::Span {
        &self.span
    }

    fn value_ref(&self) -> &T {
        &self.value
    }

    fn value(self) -> T {
        self.value
    }

    fn new(start: usize, end: usize, value: T) -> Self {
        Self {
            span: TestSpan { start, end },
            value,
        }
    }
}

#[test]
fn test_empty_span() {
    let span = TestSpan::new(0, 0);
    assert_eq!(span.len(), 0);
    assert!(span.is_empty());
}

#[test]
fn test_empty_span_call_site() {
    let span = TestSpan::call_site();
    assert_eq!(span.start(), 0);
    assert_eq!(span.end(), 0);
    assert!(span.is_empty());
}

#[test_case::test_case(0, 0; "zero span")]
#[test_case::test_case(100, 100; "same position")]
#[test_case::test_case(usize::MAX, usize::MAX; "max position")]
fn test_empty_spans_various_positions(start: usize, end: usize) {
    let span = TestSpan::new(start, end);
    assert_eq!(span.len(), 0);
    assert!(span.is_empty());
}

#[test_case::test_case(0, 10, 10; "normal span")]
#[test_case::test_case(5, 10, 5; "offset span")]
#[test_case::test_case(10, 5, 0; "inverted span clamps to 0")]
#[test_case::test_case(usize::MAX, 0, 0; "max inverted clamps to 0")]
#[test_case::test_case(0, usize::MAX, usize::MAX; "max length span")]
fn test_span_length_clamping(start: usize, end: usize, expected_len: usize) {
    let span = TestSpan::new(start, end);
    assert_eq!(span.len(), expected_len);
}

#[test]
fn test_inverted_span_is_empty() {
    // When end < start, saturating_sub returns 0
    let span = TestSpan::new(100, 50);
    assert!(span.is_empty());
}

#[test_case::test_case(0, 10, 5, 15, 0, 15; "overlapping spans")]
#[test_case::test_case(0, 5, 10, 15, 0, 15; "disjoint spans")]
#[test_case::test_case(5, 10, 0, 20, 0, 20; "contained span")]
#[test_case::test_case(0, 0, 0, 0, 0, 0; "empty spans")]
fn test_span_join(s1: usize, e1: usize, s2: usize, e2: usize, exp_s: usize, exp_e: usize) {
    let span1 = TestSpan::new(s1, e1);
    let span2 = TestSpan::new(s2, e2);
    let joined = span1.join(&span2);

    assert_eq!(joined.start(), exp_s);
    assert_eq!(joined.end(), exp_e);
}

#[test]
fn test_span_join_max_values() {
    // Test joining spans at extreme positions
    let span1 = TestSpan::new(0, usize::MAX / 2);
    let span2 = TestSpan::new(usize::MAX / 2, usize::MAX);
    let joined = span1.join(&span2);

    assert_eq!(joined.start(), 0);
    assert_eq!(joined.end(), usize::MAX);
}

#[test]
fn test_span_join_near_max() {
    let span1 = TestSpan::new(usize::MAX - 10, usize::MAX - 5);
    let span2 = TestSpan::new(usize::MAX - 3, usize::MAX);
    let joined = span1.join(&span2);

    assert_eq!(joined.start(), usize::MAX - 10);
    assert_eq!(joined.end(), usize::MAX);
}

#[test]
fn test_span_join_inverted_inputs() {
    // Even with inverted input spans, join uses min/max
    let span1 = TestSpan::new(100, 50); // inverted
    let span2 = TestSpan::new(200, 150); // inverted

    let joined = span1.join(&span2);
    // min(100, 200) = 100, max(50, 150) = 150
    assert_eq!(joined.start(), 100);
    assert_eq!(joined.end(), 150);
}

// ============================================
// SpannedLike map tests
// ============================================

#[test]
fn test_spanned_map_preserves_span() {
    let spanned = TestSpanned::new(10, 20, 42i32);
    let mapped = spanned.map(|v| v.to_string());

    assert_eq!(mapped.span().start(), 10);
    assert_eq!(mapped.span().end(), 20);
    assert_eq!(mapped.value_ref(), "42");
}

#[test]
fn test_spanned_map_empty_span() {
    let spanned = TestSpanned::new(0, 0, "test");
    let mapped = spanned.map(|s| s.len());

    assert!(mapped.span().is_empty());
    assert_eq!(mapped.value(), 4);
}

// ============================================
// UTF-8 boundary awareness tests (4.2)
// ============================================

#[test]
fn test_utf8_string_char_boundaries() {
    // "日本語" = 9 bytes (3 chars × 3 bytes each)
    let text = "日本語";
    assert_eq!(text.len(), 9);

    // Valid char boundaries: 0, 3, 6, 9
    assert!(text.is_char_boundary(0));
    assert!(text.is_char_boundary(3));
    assert!(text.is_char_boundary(6));
    assert!(text.is_char_boundary(9));

    // Invalid boundaries
    assert!(!text.is_char_boundary(1));
    assert!(!text.is_char_boundary(2));
    assert!(!text.is_char_boundary(4));
    assert!(!text.is_char_boundary(5));
}

#[test_case::test_case("日本語", 0, 3; "first char")]
#[test_case::test_case("日本語", 3, 6; "second char")]
#[test_case::test_case("日本語", 6, 9; "third char")]
#[test_case::test_case("日本語", 0, 9; "full string")]
fn test_span_at_valid_utf8_boundaries(text: &str, start: usize, end: usize) {
    // Verify the span indices are valid UTF-8 boundaries
    assert!(text.is_char_boundary(start));
    assert!(text.is_char_boundary(end));

    let span = TestSpan::new(start, end);
    let slice = &text[start..end];

    assert_eq!(span.len(), slice.len());
}

#[test]
fn test_span_respects_multibyte_chars() {
    // Mix of ASCII and multibyte
    let text = "a日b"; // 1 + 3 + 1 = 5 bytes
    assert_eq!(text.len(), 5);

    // Valid slicing points
    let span = TestSpan::new(1, 4); // "日"
    assert_eq!(span.len(), 3);
    assert_eq!(&text[1..4], "日");
}

// ============================================
// Integer overflow safety tests
// ============================================

#[test]
fn test_span_arithmetic_no_overflow() {
    // These should not panic due to overflow
    let _ = TestSpan::new(usize::MAX, usize::MAX);
    let _ = TestSpan::new(0, usize::MAX);
    let _ = TestSpan::new(usize::MAX, 0);

    // Length calculation with saturating_sub
    let span = TestSpan::new(usize::MAX, 0);
    assert_eq!(span.len(), 0); // saturating_sub prevents underflow

    let span2 = TestSpan::new(0, usize::MAX);
    assert_eq!(span2.len(), usize::MAX);
}

#[test]
fn test_span_join_no_overflow() {
    // min/max operations don't overflow
    let span1 = TestSpan::new(usize::MAX - 1, usize::MAX);
    let span2 = TestSpan::new(0, 1);
    let joined = span1.join(&span2);

    assert_eq!(joined.start(), 0);
    assert_eq!(joined.end(), usize::MAX);
}

#[test]
fn test_very_large_span() {
    let span = TestSpan::new(usize::MAX / 2, usize::MAX);
    let expected_len = usize::MAX - usize::MAX / 2;
    assert_eq!(span.len(), expected_len);
}

#[test]
fn test_spanned_with_large_positions() {
    let spanned = TestSpanned::new(usize::MAX - 100, usize::MAX, "large");
    assert_eq!(spanned.span().len(), 100);
    assert_eq!(spanned.value(), "large");
}
