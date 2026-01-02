//! Parser configuration for resource limits and behavior tuning.
//!
//! This module provides [`ParseConfig`] for controlling parser behavior,
//! including recursion limits to prevent stack overflow attacks.
//!
//! # Recursion Limits
//!
//! Following the pattern established by `serde_json`, parsers should enforce
//! a maximum recursion depth to prevent malicious or malformed input from
//! causing stack overflows. The default limit of 128 balances security with
//! practical use cases.
//!
//! # Example
//!
//! ```ignore
//! use synkit_core::config::ParseConfig;
//!
//! // Use default limits (recursion depth: 128)
//! let config = ParseConfig::default();
//!
//! // Increase limit for deeply nested data
//! let config = ParseConfig::new()
//!     .with_max_recursion_depth(256);
//!
//! // Disable recursion limit (use with caution!)
//! let config = ParseConfig::new()
//!     .with_max_recursion_depth(usize::MAX);
//! ```

use crate::Error;

/// Configuration for parser behavior and resource limits.
///
/// Controls limits on recursion depth, token count, and other resources
/// to prevent denial-of-service attacks via malformed input.
///
/// # Default Values
///
/// | Setting | Default | Rationale |
/// |---------|---------|-----------|
/// | `max_recursion_depth` | 128 | Matches serde_json default |
/// | `max_tokens` | `usize::MAX` | No limit by default |
///
/// # Security Considerations
///
/// Without recursion limits, deeply nested input like `[[[[[[...]]]]]]` can
/// cause stack overflow. The default limit of 128 prevents most attacks while
/// allowing reasonable nesting for typical use cases.
///
/// # Example
///
/// ```ignore
/// let config = ParseConfig::default();
/// let mut parser = TokenStream::with_config(tokens, config);
///
/// // In recursive parse implementation:
/// fn parse_nested(stream: &mut TokenStream) -> Result<Nested, Error> {
///     stream.enter_nested()?; // Increments depth, checks limit
///     let inner = stream.parse()?;
///     stream.exit_nested(); // Decrements depth
///     Ok(Nested { inner })
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseConfig {
    /// Maximum allowed recursion depth.
    ///
    /// When parsing nested structures, each level of nesting increments a
    /// depth counter. If the counter exceeds this limit, parsing fails with
    /// [`Error::RecursionLimitExceeded`].
    ///
    /// Default: 128 (matching serde_json)
    pub max_recursion_depth: usize,

    /// Maximum number of tokens to process.
    ///
    /// If the parser consumes more than this many tokens, parsing fails.
    /// This can prevent resource exhaustion from extremely long inputs.
    ///
    /// Default: `usize::MAX` (no limit)
    pub max_tokens: usize,
}

impl Default for ParseConfig {
    /// Returns the default configuration.
    ///
    /// - `max_recursion_depth`: 128
    /// - `max_tokens`: `usize::MAX`
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl ParseConfig {
    /// Default configuration, usable in const contexts.
    ///
    /// Equivalent to `ParseConfig::default()` but available at compile time.
    pub const DEFAULT: Self = Self {
        max_recursion_depth: 128,
        max_tokens: usize::MAX,
    };

    /// Creates a new configuration with default values.
    #[inline]
    pub const fn new() -> Self {
        Self::DEFAULT
    }

    /// Sets the maximum recursion depth.
    ///
    /// # Arguments
    ///
    /// * `depth` - Maximum nesting level. Use `usize::MAX` to disable the limit.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = ParseConfig::new()
    ///     .with_max_recursion_depth(256);
    /// ```
    #[inline]
    pub const fn with_max_recursion_depth(mut self, depth: usize) -> Self {
        self.max_recursion_depth = depth;
        self
    }

    /// Sets the maximum token count.
    ///
    /// # Arguments
    ///
    /// * `count` - Maximum tokens to process. Use `usize::MAX` to disable.
    #[inline]
    pub const fn with_max_tokens(mut self, count: usize) -> Self {
        self.max_tokens = count;
        self
    }

    /// Disables the recursion limit.
    ///
    /// # Warning
    ///
    /// Only use this when parsing trusted input! Untrusted deeply-nested
    /// input can cause stack overflow.
    #[inline]
    pub const fn disable_recursion_limit(self) -> Self {
        self.with_max_recursion_depth(usize::MAX)
    }
}

/// Tracks recursion depth during parsing.
///
/// This is a lightweight wrapper that parsers use to track and enforce
/// recursion limits. It pairs with [`ParseConfig`] to provide the limit.
///
/// # Example
///
/// ```ignore
/// struct MyParser {
///     depth: RecursionGuard,
///     config: ParseConfig,
/// }
///
/// impl MyParser {
///     fn parse_nested(&mut self) -> Result<(), Error> {
///         self.depth.enter(self.config.max_recursion_depth)?;
///         // ... parse nested content ...
///         self.depth.exit();
///         Ok(())
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct RecursionGuard {
    /// Current recursion depth.
    depth: usize,
}

impl RecursionGuard {
    /// Creates a new guard with depth 0.
    #[inline]
    pub const fn new() -> Self {
        Self { depth: 0 }
    }

    /// Current recursion depth.
    #[inline]
    pub const fn depth(&self) -> usize {
        self.depth
    }

    /// Enter a nested context, incrementing depth.
    ///
    /// Returns `Err(Error::RecursionLimitExceeded)` if the new depth would
    /// exceed the limit.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum allowed depth (from `ParseConfig::max_recursion_depth`)
    #[inline]
    pub fn enter(&mut self, limit: usize) -> Result<(), Error> {
        self.depth = self.depth.saturating_add(1);
        if self.depth > limit {
            Err(Error::RecursionLimitExceeded {
                depth: self.depth,
                limit,
            })
        } else {
            Ok(())
        }
    }

    /// Exit a nested context, decrementing depth.
    ///
    /// Uses saturating subtraction so extra `exit()` calls don't underflow.
    #[inline]
    pub fn exit(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Reset depth to zero.
    ///
    /// Useful when reusing a parser for multiple inputs.
    #[inline]
    pub fn reset(&mut self) {
        self.depth = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config_defaults() {
        let config = ParseConfig::default();
        assert_eq!(config.max_recursion_depth, 128);
        assert_eq!(config.max_tokens, usize::MAX);
    }

    #[test]
    fn test_parse_config_builder() {
        let config = ParseConfig::new()
            .with_max_recursion_depth(256)
            .with_max_tokens(10000);

        assert_eq!(config.max_recursion_depth, 256);
        assert_eq!(config.max_tokens, 10000);
    }

    #[test]
    fn test_parse_config_disable_recursion() {
        let config = ParseConfig::new().disable_recursion_limit();
        assert_eq!(config.max_recursion_depth, usize::MAX);
    }

    #[test]
    fn test_recursion_guard_basic() {
        let mut guard = RecursionGuard::new();
        assert_eq!(guard.depth(), 0);

        guard.enter(128).unwrap();
        assert_eq!(guard.depth(), 1);

        guard.enter(128).unwrap();
        assert_eq!(guard.depth(), 2);

        guard.exit();
        assert_eq!(guard.depth(), 1);

        guard.exit();
        assert_eq!(guard.depth(), 0);
    }

    #[test]
    fn test_recursion_guard_limit_exceeded() {
        let mut guard = RecursionGuard::new();

        // Fill to limit
        for _ in 0..3 {
            guard.enter(3).unwrap();
        }
        assert_eq!(guard.depth(), 3);

        // Next should fail
        let result = guard.enter(3);
        assert!(matches!(
            result,
            Err(Error::RecursionLimitExceeded { depth: 4, limit: 3 })
        ));
    }

    #[test]
    fn test_recursion_guard_exit_saturates() {
        let mut guard = RecursionGuard::new();

        // Extra exits don't underflow
        guard.exit();
        guard.exit();
        assert_eq!(guard.depth(), 0);
    }

    #[test]
    fn test_recursion_guard_reset() {
        let mut guard = RecursionGuard::new();
        guard.enter(128).unwrap();
        guard.enter(128).unwrap();
        assert_eq!(guard.depth(), 2);

        guard.reset();
        assert_eq!(guard.depth(), 0);
    }
}
