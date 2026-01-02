//! Core error types for synkit.
//!
//! User-defined error types should implement `From<synkit::Error>` to integrate
//! with synkit's built-in error handling.

use core::fmt;

/// Core synkit error type.
///
/// This enum captures errors that originate from synkit's internal operations.
/// User-defined parsers should define their own error types and implement
/// `From<Error>` to convert synkit errors into their domain-specific errors.
///
/// # Example
///
/// ```ignore
/// use thiserror::Error;
///
/// #[derive(Error, Debug)]
/// pub enum MyParseError {
///     #[error("stream not fully consumed: {remaining} tokens remaining")]
///     StreamNotConsumed { remaining: usize },
///
///     #[error("expected {expect}, found {found}")]
///     Expected { expect: &'static str, found: String },
///
///     // ... other variants
/// }
///
/// impl From<synkit::Error> for MyParseError {
///     fn from(err: synkit::Error) -> Self {
///         match err {
///             synkit::Error::StreamNotConsumed { remaining } => {
///                 MyParseError::StreamNotConsumed { remaining }
///             }
///         }
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The token stream was not fully consumed after parsing.
    ///
    /// This error is returned by `ensure_consumed()` when there are
    /// remaining tokens (excluding whitespace) in the stream.
    StreamNotConsumed {
        /// Number of remaining tokens (excluding whitespace).
        remaining: usize,
    },

    /// Recursion limit exceeded during parsing.
    ///
    /// This error is returned when nested parsing exceeds the configured
    /// maximum recursion depth. This limit exists to prevent stack overflow
    /// from deeply nested malicious input.
    ///
    /// # Example
    ///
    /// Input like `[[[[[[...]]]]]]` with thousands of nesting levels would
    /// trigger this error with the default limit of 128.
    RecursionLimitExceeded {
        /// Current recursion depth when limit was exceeded.
        depth: usize,
        /// Maximum allowed recursion depth.
        limit: usize,
    },

    /// Token limit exceeded during parsing.
    ///
    /// This error is returned when the parser has consumed more tokens than
    /// the configured maximum. This limit exists to prevent resource exhaustion
    /// from extremely long inputs.
    TokenLimitExceeded {
        /// Number of tokens consumed when limit was exceeded.
        consumed: usize,
        /// Maximum allowed token count.
        limit: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::StreamNotConsumed { remaining } => {
                write!(
                    f,
                    "stream not fully consumed: {} tokens remaining",
                    remaining
                )
            }
            Error::RecursionLimitExceeded { depth, limit } => {
                write!(
                    f,
                    "recursion limit exceeded: depth {} > limit {}",
                    depth, limit
                )
            }
            Error::TokenLimitExceeded { consumed, limit } => {
                write!(
                    f,
                    "token limit exceeded: consumed {} > limit {}",
                    consumed, limit
                )
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
