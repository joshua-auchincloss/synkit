//! Async streaming parser support.
//!
//! This module provides traits and utilities for incremental, asynchronous parsing
//! of token streams. It enables parsing of data that arrives in chunks (e.g., over
//! a network connection) without blocking.
//!
//! # Architecture
//!
//! The async parsing system uses a streaming pipeline:
//! - **Source** feeds chunks to **AsyncTokenStream** (lexer + buffer)
//! - **AsyncTokenStream** emits tokens to **AstStream** (parser + AST buffer)
//! - **AstStream** emits AST nodes to **Consumer**
//!
#![cfg_attr(feature = "docs", doc = simple_mermaid::mermaid!("../docs/diagrams/async_stream.mmd"))]
//!
//! # Features
//!
//! - **Incremental lexing**: Source chunks are lexed as they arrive
//! - **Incremental parsing**: AST nodes are emitted as soon as parseable
//! - **Backpressure**: Channels provide natural flow control
//! - **Cancellation**: Streams can be cancelled via channel closure
//!
//! # Example
//!
//! ```ignore
//! use synkit::async_stream::{AsyncTokenStream, AstStream};
//! use tokio_stream::StreamExt;
//!
//! async fn parse_network_data(mut rx: tokio::sync::mpsc::Receiver<String>) {
//!     let (token_tx, token_rx) = tokio::sync::mpsc::channel(32);
//!     let (ast_tx, mut ast_rx) = tokio::sync::mpsc::channel(16);
//!
//!     // Spawn lexer task
//!     tokio::spawn(async move {
//!         let mut lexer = AsyncTokenStream::new(token_tx);
//!         while let Some(chunk) = rx.recv().await {
//!             lexer.feed(&chunk).await?;
//!         }
//!         lexer.finish().await?;
//!     });
//!
//!     // Spawn parser task
//!     tokio::spawn(async move {
//!         let mut parser = AstStream::<Document>::new(token_rx, ast_tx);
//!         parser.run().await?;
//!     });
//!
//!     // Consume AST nodes
//!     while let Some(node) = ast_rx.recv().await {
//!         process_node(node);
//!     }
//! }
//! ```

use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

/// State of an incremental parse operation.
///
/// Represents the three possible outcomes when parsing from an incremental
/// token stream: need more input, complete, or error.
///
#[cfg_attr(doc, doc = simple_mermaid::mermaid!("docs/diagrams/parse_state.mmd"))]
///
/// # Example
///
/// ```ignore
/// match parser.try_parse(&tokens, checkpoint)? {
///     ParseState::NeedMore => {
///         // Store checkpoint, wait for more tokens
///     }
///     ParseState::Complete => {
///         // Emit AST node, advance cursor
///     }
///     ParseState::Error => {
///         // Report error, possibly attempt recovery
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseState {
    /// More input is needed to continue parsing.
    ///
    /// The parser has consumed all available tokens but cannot determine
    /// if the parse is complete. The caller should feed more tokens and
    /// retry from the stored checkpoint.
    NeedMore,
    /// Parsing is complete with no errors.
    ///
    /// A complete AST node (or other parse result) is available. The
    /// checkpoint indicates how many tokens were consumed.
    Complete,
    /// An error occurred during parsing.
    ///
    /// The input was invalid. Depending on error recovery strategy, the
    /// parser may attempt to resync at a known boundary token.
    Error,
}

/// A checkpoint for incremental parsing.
///
/// Stores the parser state at a "resync point" where parsing can resume
/// after receiving more input. Checkpoints enable the parser to avoid
/// re-processing tokens when more input arrives.
///
/// # Usage Pattern
///
/// 1. Parser attempts to parse from current checkpoint
/// 2. If `ParseState::NeedMore`, store the checkpoint
/// 3. When more tokens arrive, resume from stored checkpoint
/// 4. If `ParseState::Complete`, advance checkpoint past consumed tokens
///
/// # Fields
///
/// - `cursor`: Absolute position in the logical token stream
/// - `tokens_consumed`: Tokens used in the current parse attempt (may be partial)
/// - `state`: Opaque parser state for complex grammars (e.g., LR parser state stack)
///
/// # Example
///
/// ```ignore
/// let mut checkpoint = ParseCheckpoint::default();
///
/// loop {
///     match parser.try_parse(&tokens, &mut checkpoint) {
///         ParseState::Complete => {
///             emit_ast_node(parser.take_result());
///             checkpoint = ParseCheckpoint {
///                 cursor: checkpoint.cursor + checkpoint.tokens_consumed,
///                 tokens_consumed: 0,
///                 state: 0,
///             };
///         }
///         ParseState::NeedMore => break, // Wait for more tokens
///         ParseState::Error => return Err(parse_error()),
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct ParseCheckpoint {
    /// Cursor position in the token stream.
    ///
    /// This is the absolute index into the logical stream of all tokens
    /// seen so far. When tokens are drained from the buffer, this value
    /// may need adjustment.
    pub cursor: usize,
    /// Number of tokens consumed from the current chunk.
    ///
    /// Reset to 0 after each successful parse. Used to track progress
    /// within a single parse attempt.
    pub tokens_consumed: usize,
    /// Parser-specific state (e.g., nesting depth, current production).
    ///
    /// For simple parsers, this may be unused (0). For stateful parsers
    /// like LR parsers, this encodes the state stack or production being
    /// reduced. The interpretation is parser-specific.
    pub state: u64,
}

/// Error type for async streaming operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamError {
    /// Channel was closed unexpectedly.
    ChannelClosed,
    /// Lexer encountered an error.
    LexError(String),
    /// Parser encountered an error.
    ParseError(String),
    /// Incomplete input at end of stream.
    IncompleteInput,
    /// Input chunk exceeded maximum allowed size.
    ChunkTooLarge {
        /// Size of the chunk that was rejected.
        size: usize,
        /// Maximum allowed chunk size.
        max: usize,
    },
    /// Token buffer exceeded maximum capacity.
    BufferOverflow {
        /// Current buffer size.
        current: usize,
        /// Maximum allowed buffer size.
        max: usize,
    },
    /// Timeout waiting for more input.
    Timeout,
    /// Resource limit exceeded (e.g., max tokens, max depth).
    ResourceLimit {
        /// Name of the resource that was exhausted.
        resource: &'static str,
        /// Current value.
        current: usize,
        /// Maximum allowed value.
        max: usize,
    },
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamError::ChannelClosed => write!(f, "channel closed unexpectedly"),
            StreamError::LexError(msg) => write!(f, "lex error: {}", msg),
            StreamError::ParseError(msg) => write!(f, "parse error: {}", msg),
            StreamError::IncompleteInput => write!(f, "incomplete input at end of stream"),
            StreamError::ChunkTooLarge { size, max } => {
                write!(f, "chunk size {} exceeds maximum {}", size, max)
            }
            StreamError::BufferOverflow { current, max } => {
                write!(f, "buffer size {} exceeds maximum {}", current, max)
            }
            StreamError::Timeout => write!(f, "timeout waiting for input"),
            StreamError::ResourceLimit {
                resource,
                current,
                max,
            } => {
                write!(f, "{} limit exceeded: {} > {}", resource, current, max)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for StreamError {}

/// Hints for pre-allocating buffers in incremental lexers.
///
/// Providing accurate hints can significantly reduce allocations during parsing.
#[derive(Debug, Clone, Copy)]
pub struct LexerCapacityHint {
    /// Expected buffer size for source text accumulation.
    /// Default: 4096 bytes
    pub buffer_capacity: usize,
    /// Expected number of tokens per chunk.
    /// Default: 256 tokens
    pub tokens_per_chunk: usize,
}

impl Default for LexerCapacityHint {
    fn default() -> Self {
        Self {
            buffer_capacity: 4096,
            tokens_per_chunk: 256,
        }
    }
}

impl LexerCapacityHint {
    /// Create hints optimized for small inputs (<1KB).
    pub const fn small() -> Self {
        Self {
            buffer_capacity: 256,
            tokens_per_chunk: 32,
        }
    }

    /// Create hints optimized for medium inputs (1KB-64KB).
    pub const fn medium() -> Self {
        Self {
            buffer_capacity: 4096,
            tokens_per_chunk: 256,
        }
    }

    /// Create hints optimized for large inputs (>64KB).
    pub const fn large() -> Self {
        Self {
            buffer_capacity: 65536,
            tokens_per_chunk: 2048,
        }
    }

    /// Create custom hints from expected chunk size.
    ///
    /// Estimates tokens as ~1 token per 4 bytes (conservative).
    pub const fn from_chunk_size(chunk_size: usize) -> Self {
        Self {
            buffer_capacity: chunk_size,
            tokens_per_chunk: chunk_size / 4,
        }
    }
}

/// Trait for types that can be incrementally lexed.
///
/// This trait extends a synchronous lexer with the ability to process
/// input in chunks.
pub trait IncrementalLexer: Sized {
    /// The token type produced by the lexer.
    type Token: Clone;
    /// The span type for token positions.
    type Span: Clone;
    /// The spanned token type.
    type Spanned: Clone;
    /// The error type for lexing failures.
    type Error: fmt::Display;

    /// Create a new incremental lexer with default capacity.
    fn new() -> Self;

    /// Create a new incremental lexer with capacity hints.
    ///
    /// Implementations should use these hints to pre-allocate buffers,
    /// reducing allocations during parsing.
    ///
    /// # Example
    /// ```ignore
    /// let hint = LexerCapacityHint::from_chunk_size(4096);
    /// let lexer = MyLexer::with_capacity_hint(hint);
    /// ```
    fn with_capacity_hint(_hint: LexerCapacityHint) -> Self {
        // Default implementation ignores the hint
        Self::new()
    }

    /// Feed a chunk of source text to the lexer.
    ///
    /// Returns the tokens that can be produced from the accumulated input.
    /// Some tokens may be held back if they span a chunk boundary.
    fn feed(&mut self, chunk: &str) -> Result<Vec<Self::Spanned>, Self::Error>;

    /// Feed a chunk and store tokens in the provided buffer.
    ///
    /// This avoids allocating a new Vec for each chunk. The buffer is NOT
    /// cleared before appending - call `buffer.clear()` first if needed.
    ///
    /// Returns the number of tokens added.
    fn feed_into(
        &mut self,
        chunk: &str,
        buffer: &mut Vec<Self::Spanned>,
    ) -> Result<usize, Self::Error> {
        let tokens = self.feed(chunk)?;
        let count = tokens.len();
        buffer.extend(tokens);
        Ok(count)
    }

    /// Signal that no more input will arrive.
    ///
    /// Returns any remaining tokens and validates that the input is complete.
    fn finish(self) -> Result<Vec<Self::Spanned>, Self::Error>;

    /// Finish and store remaining tokens in the provided buffer.
    ///
    /// Returns the number of tokens added.
    fn finish_into(self, buffer: &mut Vec<Self::Spanned>) -> Result<usize, Self::Error> {
        let tokens = self.finish()?;
        let count = tokens.len();
        buffer.extend(tokens);
        Ok(count)
    }

    /// Get the current byte offset in the source.
    fn offset(&self) -> usize;
}

// =============================================================================
// Chunk Boundary Detection
// =============================================================================

/// Describes how to detect chunk boundaries for incremental parsing.
///
/// This trait allows parsers to declaratively specify what constitutes a
/// complete parseable unit without re-implementing depth tracking logic.
///
/// # Example
/// ```ignore
/// impl ChunkBoundary for JsonLine {
///     type Token = Token;
///
///     fn is_boundary_token(token: &Token) -> bool {
///         matches!(token, Token::Newline)
///     }
///
///     fn depth_delta(token: &Token) -> i32 {
///         match token {
///             Token::LBrace | Token::LBracket => 1,
///             Token::RBrace | Token::RBracket => -1,
///             _ => 0,
///         }
///     }
/// }
/// ```
pub trait ChunkBoundary {
    /// The token type for boundary detection.
    type Token;

    /// Returns true if this token could be a chunk boundary.
    ///
    /// A boundary is only valid when depth is 0 (balanced delimiters).
    fn is_boundary_token(token: &Self::Token) -> bool;

    /// Returns the depth change caused by this token.
    ///
    /// - Positive: opens a nested structure (e.g., `{`, `[`)
    /// - Negative: closes a nested structure (e.g., `}`, `]`)
    /// - Zero: no effect on depth
    #[inline]
    fn depth_delta(token: &Self::Token) -> i32 {
        let _ = token;
        0
    }

    /// Returns true if this token should be skipped when looking for boundaries.
    ///
    /// Useful for ignoring whitespace tokens.
    #[inline]
    fn is_ignorable(token: &Self::Token) -> bool {
        let _ = token;
        false
    }

    /// Find the next chunk boundary in the token slice.
    ///
    /// Returns `Some(end_pos)` where `end_pos` is the index AFTER the boundary token,
    /// or `None` if no complete chunk is available.
    fn find_boundary<S: AsRef<Self::Token>>(tokens: &[S], start: usize) -> Option<usize> {
        let mut depth: i32 = 0;

        for (i, tok) in tokens.iter().enumerate().skip(start) {
            let token = tok.as_ref();
            depth += Self::depth_delta(token);

            if depth == 0 && Self::is_boundary_token(token) {
                return Some(i + 1); // Past the boundary token
            }
        }

        None
    }

    /// Check if a complete chunk is available starting at the given position.
    #[inline]
    fn has_complete_chunk<S: AsRef<Self::Token>>(tokens: &[S], start: usize) -> bool {
        Self::find_boundary(tokens, start).is_some()
    }
}

// =============================================================================
// Incremental Token Buffer
// =============================================================================

/// A reusable buffer for incremental token processing.
///
/// This buffer manages tokens efficiently by:
/// - Reusing allocated capacity across parse operations
/// - Supporting compaction to remove consumed tokens
/// - Tracking cursor position for incremental parsing
///
/// # Example
/// ```ignore
/// let mut buffer = IncrementalBuffer::with_capacity(1024);
///
/// // Feed tokens from lexer
/// buffer.extend(lexer.feed(chunk)?);
///
/// // Parse all available chunks
/// while let Some(boundary) = JsonLine::find_boundary(buffer.remaining(), 0) {
///     let chunk_tokens = &buffer.remaining()[..boundary];
///     let ast = parse_chunk(chunk_tokens)?;
///     buffer.consume(boundary);
///     results.push(ast);
/// }
///
/// // Compact to release consumed memory
/// buffer.compact();
/// ```
#[derive(Debug, Clone)]
pub struct IncrementalBuffer<T> {
    tokens: Vec<T>,
    cursor: usize,
}

impl<T> IncrementalBuffer<T> {
    /// Create an empty buffer.
    #[inline]
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            cursor: 0,
        }
    }

    /// Create a buffer with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            tokens: Vec::with_capacity(capacity),
            cursor: 0,
        }
    }

    /// Returns the number of unconsumed tokens.
    #[inline]
    pub fn len(&self) -> usize {
        self.tokens.len() - self.cursor
    }

    /// Returns true if there are no unconsumed tokens.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    /// Returns the total capacity of the buffer.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.tokens.capacity()
    }

    /// Returns unconsumed tokens as a slice.
    #[inline]
    pub fn remaining(&self) -> &[T] {
        &self.tokens[self.cursor..]
    }

    /// Returns unconsumed tokens as a mutable slice.
    #[inline]
    pub fn remaining_mut(&mut self) -> &mut [T] {
        &mut self.tokens[self.cursor..]
    }

    /// Append tokens to the buffer.
    #[inline]
    pub fn extend(&mut self, tokens: impl IntoIterator<Item = T>) {
        self.tokens.extend(tokens);
    }

    /// Push a single token.
    #[inline]
    pub fn push(&mut self, token: T) {
        self.tokens.push(token);
    }

    /// Mark `n` tokens as consumed.
    ///
    /// These tokens will be removed on the next `compact()` call.
    ///
    /// # Clamping Behavior
    ///
    /// If `n` exceeds the number of remaining tokens, the cursor is clamped
    /// to the buffer length rather than exceeding it. This prevents out-of-bounds
    /// access and allows callers to safely consume "all remaining" by passing
    /// a large value like `usize::MAX`.
    #[inline]
    pub fn consume(&mut self, n: usize) {
        self.cursor = (self.cursor + n).min(self.tokens.len());
    }

    /// Get the current cursor position.
    #[inline]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Compact the buffer by removing consumed tokens.
    ///
    /// This shifts remaining tokens to the front and resets the cursor.
    /// Call this periodically to release memory.
    pub fn compact(&mut self) {
        if self.cursor > 0 {
            self.tokens.drain(..self.cursor);
            self.cursor = 0;
        }
    }

    /// Clear all tokens and reset cursor.
    #[inline]
    pub fn clear(&mut self) {
        self.tokens.clear();
        self.cursor = 0;
    }

    /// Shrink capacity to fit current contents.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.compact();
        self.tokens.shrink_to_fit();
    }

    /// Reserve capacity for additional tokens.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.tokens.reserve(additional);
    }

    /// Get total tokens (including consumed).
    #[inline]
    pub fn total_tokens(&self) -> usize {
        self.tokens.len()
    }

    /// Get number of consumed tokens pending compaction.
    #[inline]
    pub fn consumed_pending(&self) -> usize {
        self.cursor
    }

    /// Get mutable access to the underlying token storage.
    ///
    /// **Warning**: This bypasses the cursor-based consumption tracking.
    /// Only use for appending tokens (e.g., from `IncrementalLexer::feed_into`).
    /// Do not remove or reorder tokens.
    #[inline]
    pub fn tokens_mut(&mut self) -> &mut Vec<T> {
        &mut self.tokens
    }
}

impl<T> Default for IncrementalBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> AsRef<[T]> for IncrementalBuffer<T> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.remaining()
    }
}

// =============================================================================
// Incremental Parse Trait
// =============================================================================

/// Trait for types that can be incrementally parsed.
///
/// This trait enables parsing of AST nodes as tokens become available,
/// without waiting for the complete input.
pub trait IncrementalParse: Sized {
    /// The token type consumed by the parser.
    type Token: Clone;
    /// The error type for parsing failures.
    type Error: fmt::Display;

    /// Attempt to parse from the given tokens starting at the checkpoint.
    ///
    /// Returns:
    /// - `Ok((Some(node), new_checkpoint))` if a complete node was parsed
    /// - `Ok((None, checkpoint))` if more tokens are needed
    /// - `Err(error)` if an unrecoverable error occurred
    fn parse_incremental<S>(
        tokens: &[S],
        checkpoint: &ParseCheckpoint,
    ) -> Result<(Option<Self>, ParseCheckpoint), Self::Error>
    where
        S: AsRef<Self::Token>;

    /// Check if parsing can produce a result with the current tokens.
    ///
    /// This is used for early return when more input is clearly needed.
    fn can_parse<S>(tokens: &[S], checkpoint: &ParseCheckpoint) -> bool
    where
        S: AsRef<Self::Token>;
}

/// Helper to parse all available chunks from a buffer.
///
/// This function repeatedly calls the parser until no more complete chunks
/// are available, collecting results into a Vec.
///
/// # Type Parameters
/// - `T`: The AST node type (must implement `ChunkBoundary`)
/// - `Tok`: The token type
/// - `S`: The token wrapper type (implements `AsRef<Tok>`)
/// - `F`: A function that parses a slice of tokens into an AST node
///
/// # Example
/// ```ignore
/// let results = parse_available_chunks::<JsonLine, Token, _, _, _>(
///     &mut buffer,
///     |tokens| {
///         let mut stream = TokenStream::from_tokens(tokens);
///         JsonLine::parse(&mut stream)
///     },
/// )?;
/// ```
pub fn parse_available_chunks<T, Tok, S, E, F>(
    buffer: &mut IncrementalBuffer<S>,
    mut parse_fn: F,
) -> Result<Vec<T>, E>
where
    T: ChunkBoundary<Token = Tok>,
    S: AsRef<Tok> + Clone,
    F: FnMut(&[S]) -> Result<T, E>,
{
    let mut results = Vec::new();

    loop {
        let remaining = buffer.remaining();
        match T::find_boundary(remaining, 0) {
            Some(boundary) => {
                let chunk = &remaining[..boundary];
                let ast = parse_fn(chunk)?;
                buffer.consume(boundary);
                results.push(ast);
            }
            None => break,
        }
    }

    Ok(results)
}

/// A future that resolves when more tokens are available or the stream ends.
pub struct TokenFuture<'a, T> {
    tokens: &'a mut Vec<T>,
    min_count: usize,
}

impl<'a, T> Future for TokenFuture<'a, T> {
    type Output = bool;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.tokens.len() >= self.min_count {
            Poll::Ready(true)
        } else {
            Poll::Pending
        }
    }
}

/// Configuration for async stream processing.
///
/// Controls buffer sizes, capacity hints, and resource limits for the
/// streaming parse pipeline. Proper configuration can significantly
/// impact performance and memory usage.
///
/// # Presets
///
/// Use the preset constructors for common scenarios:
/// - [`StreamConfig::small()`]: Low memory, small inputs (<1KB)
/// - [`StreamConfig::medium()`]: Balanced (default), typical use (1KB-64KB)
/// - [`StreamConfig::large()`]: High throughput, large inputs (>64KB)
/// - [`StreamConfig::from_chunk_size()`]: Auto-tune from expected chunk size
///
/// # Performance Considerations
///
/// - `token_buffer_size`: Larger buffers reduce channel contention but use more memory
/// - `ast_buffer_size`: Should match expected AST nodes per parse batch
/// - `max_chunk_size`: Protects against memory exhaustion from large inputs
/// - `lexer_hint`: Pre-allocation reduces allocations during hot paths
///
/// # Example
///
/// ```ignore
/// // For processing 8KB network packets
/// let config = StreamConfig::from_chunk_size(8192);
/// let stream = AsyncTokenStream::with_config(tx, config);
///
/// // Or customize for specific needs
/// let config = StreamConfig {
///     token_buffer_size: 2048,
///     ast_buffer_size: 128,
///     max_chunk_size: 128 * 1024,
///     lexer_hint: LexerCapacityHint::large(),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Size of the token buffer (number of tokens).
    ///
    /// Used for channel capacity and pre-allocation hints. Larger values
    /// reduce backpressure but increase memory usage. Default: 1024.
    pub token_buffer_size: usize,
    /// Size of the AST buffer (number of nodes).
    ///
    /// Controls how many AST nodes can be buffered before the parser
    /// blocks waiting for the consumer. Default: 64.
    pub ast_buffer_size: usize,
    /// Maximum source chunk size to buffer.
    ///
    /// Chunks larger than this will trigger [`StreamError::ChunkTooLarge`].
    /// Protects against memory exhaustion. Default: 64KB.
    pub max_chunk_size: usize,
    /// Capacity hints for the incremental lexer.
    ///
    /// Passed to [`IncrementalLexer::with_capacity_hint`] to pre-allocate
    /// internal buffers. Default: [`LexerCapacityHint::medium()`].
    pub lexer_hint: LexerCapacityHint,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            token_buffer_size: 1024,
            ast_buffer_size: 64,
            max_chunk_size: 64 * 1024, // 64KB
            lexer_hint: LexerCapacityHint::medium(),
        }
    }
}

impl StreamConfig {
    /// Configuration optimized for small inputs (<1KB).
    pub const fn small() -> Self {
        Self {
            token_buffer_size: 128,
            ast_buffer_size: 16,
            max_chunk_size: 4 * 1024,
            lexer_hint: LexerCapacityHint::small(),
        }
    }

    /// Configuration optimized for medium inputs (1KB-64KB).
    pub const fn medium() -> Self {
        Self {
            token_buffer_size: 1024,
            ast_buffer_size: 64,
            max_chunk_size: 64 * 1024,
            lexer_hint: LexerCapacityHint::medium(),
        }
    }

    /// Configuration optimized for large inputs (>64KB).
    pub const fn large() -> Self {
        Self {
            token_buffer_size: 8192,
            ast_buffer_size: 512,
            max_chunk_size: 256 * 1024,
            lexer_hint: LexerCapacityHint::large(),
        }
    }

    /// Create configuration from expected chunk size.
    pub const fn from_chunk_size(chunk_size: usize) -> Self {
        let tokens_estimate = chunk_size / 4;
        Self {
            token_buffer_size: tokens_estimate,
            ast_buffer_size: tokens_estimate / 16,
            max_chunk_size: chunk_size * 2,
            lexer_hint: LexerCapacityHint::from_chunk_size(chunk_size),
        }
    }
}

#[cfg(feature = "tokio")]
pub mod tokio_impl {
    //! Tokio-based async stream implementation.

    use super::*;
    use ::tokio::sync::mpsc;

    /// Async token stream that receives source chunks and emits tokens.
    pub struct AsyncTokenStream<L: IncrementalLexer> {
        lexer: L,
        token_tx: mpsc::Sender<L::Spanned>,
        config: StreamConfig,
    }

    impl<L: IncrementalLexer> AsyncTokenStream<L> {
        /// Create a new async token stream with default configuration.
        pub fn new(token_tx: mpsc::Sender<L::Spanned>) -> Self {
            Self::with_config(token_tx, StreamConfig::default())
        }

        /// Create a new async token stream with custom configuration.
        ///
        /// The lexer is created with capacity hints from the config.
        pub fn with_config(token_tx: mpsc::Sender<L::Spanned>, config: StreamConfig) -> Self {
            Self {
                lexer: L::with_capacity_hint(config.lexer_hint),
                token_tx,
                config,
            }
        }

        /// Feed a chunk of source text to the lexer.
        pub async fn feed(&mut self, chunk: &str) -> Result<(), StreamError> {
            // Validate chunk size
            if chunk.len() > self.config.max_chunk_size {
                return Err(StreamError::ChunkTooLarge {
                    size: chunk.len(),
                    max: self.config.max_chunk_size,
                });
            }

            // Lex the chunk
            let tokens = self
                .lexer
                .feed(chunk)
                .map_err(|e| StreamError::LexError(e.to_string()))?;

            // Send tokens to the parser
            for token in tokens {
                self.token_tx
                    .send(token)
                    .await
                    .map_err(|_| StreamError::ChannelClosed)?;
            }

            Ok(())
        }

        /// Signal that no more input will arrive.
        pub async fn finish(self) -> Result<(), StreamError> {
            let tokens = self
                .lexer
                .finish()
                .map_err(|e| StreamError::LexError(e.to_string()))?;

            for token in tokens {
                self.token_tx
                    .send(token)
                    .await
                    .map_err(|_| StreamError::ChannelClosed)?;
            }

            Ok(())
        }
    }

    /// Async AST stream that receives tokens and emits parsed nodes.
    pub struct AstStream<T, Tok>
    where
        T: IncrementalParse<Token = Tok>,
        Tok: Clone,
    {
        token_rx: mpsc::Receiver<Tok>,
        ast_tx: mpsc::Sender<T>,
        token_buffer: Vec<Tok>,
        checkpoint: ParseCheckpoint,
        config: StreamConfig,
    }

    impl<T, Tok> AstStream<T, Tok>
    where
        T: IncrementalParse<Token = Tok>,
        Tok: Clone + AsRef<Tok>,
    {
        /// Create a new AST stream.
        pub fn new(token_rx: mpsc::Receiver<Tok>, ast_tx: mpsc::Sender<T>) -> Self {
            Self::with_config(token_rx, ast_tx, StreamConfig::default())
        }

        /// Create a new AST stream with custom configuration.
        pub fn with_config(
            token_rx: mpsc::Receiver<Tok>,
            ast_tx: mpsc::Sender<T>,
            config: StreamConfig,
        ) -> Self {
            Self {
                token_rx,
                ast_tx,
                token_buffer: Vec::with_capacity(config.token_buffer_size),
                checkpoint: ParseCheckpoint::default(),
                config,
            }
        }

        /// Run the parser until the token stream is exhausted.
        pub async fn run(&mut self) -> Result<(), StreamError> {
            loop {
                // Try to receive more tokens
                match self.token_rx.recv().await {
                    Some(token) => {
                        // Check buffer capacity before adding
                        if self.token_buffer.len() >= self.config.token_buffer_size * 2 {
                            return Err(StreamError::BufferOverflow {
                                current: self.token_buffer.len(),
                                max: self.config.token_buffer_size * 2,
                            });
                        }

                        self.token_buffer.push(token);

                        // Try to parse if we have enough tokens
                        if T::can_parse(&self.token_buffer, &self.checkpoint) {
                            self.try_parse().await?;
                        }
                    }
                    None => {
                        // Channel closed - try final parse
                        self.try_parse().await?;

                        // Check for incomplete input
                        if !self.token_buffer.is_empty()
                            && self.checkpoint.cursor < self.token_buffer.len()
                        {
                            return Err(StreamError::IncompleteInput);
                        }

                        return Ok(());
                    }
                }
            }
        }

        async fn try_parse(&mut self) -> Result<(), StreamError> {
            loop {
                match T::parse_incremental(&self.token_buffer, &self.checkpoint) {
                    Ok((Some(node), new_checkpoint)) => {
                        self.checkpoint = new_checkpoint;
                        self.ast_tx
                            .send(node)
                            .await
                            .map_err(|_| StreamError::ChannelClosed)?;
                    }
                    Ok((None, _)) => {
                        // Need more tokens
                        break;
                    }
                    Err(e) => {
                        return Err(StreamError::ParseError(e.to_string()));
                    }
                }
            }

            // Compact the buffer if we've consumed many tokens
            if self.checkpoint.tokens_consumed > self.config.token_buffer_size / 2 {
                self.compact_buffer();
            }

            Ok(())
        }

        fn compact_buffer(&mut self) {
            let consumed = self.checkpoint.tokens_consumed;
            if consumed > 0 {
                self.token_buffer.drain(..consumed);
                self.checkpoint.cursor -= consumed;
                self.checkpoint.tokens_consumed = 0;
            }
        }
    }
}

#[cfg(feature = "futures")]
pub mod futures_impl {
    //! Futures-based async stream implementation (runtime-agnostic).

    use super::*;
    use core::pin::Pin;
    use futures_core::Stream;

    /// A stream adapter that yields parsed AST nodes.
    pub struct ParseStream<S, T, Tok>
    where
        S: Stream<Item = Tok>,
        T: IncrementalParse<Token = Tok>,
        Tok: Clone,
    {
        inner: S,
        token_buffer: Vec<Tok>,
        checkpoint: ParseCheckpoint,
        pending_node: Option<T>,
        _marker: core::marker::PhantomData<T>,
    }

    impl<S, T, Tok> ParseStream<S, T, Tok>
    where
        S: Stream<Item = Tok>,
        T: IncrementalParse<Token = Tok>,
        Tok: Clone,
    {
        /// Create a new parse stream wrapping a token stream.
        pub fn new(inner: S) -> Self {
            Self::with_capacity(inner, 256)
        }

        /// Create a new parse stream with pre-allocated token buffer.
        ///
        /// Use this when you have an estimate of how many tokens will be buffered.
        pub fn with_capacity(inner: S, token_buffer_capacity: usize) -> Self {
            Self {
                inner,
                token_buffer: Vec::with_capacity(token_buffer_capacity),
                checkpoint: ParseCheckpoint::default(),
                pending_node: None,
                _marker: core::marker::PhantomData,
            }
        }
    }

    impl<S, T, Tok> Stream for ParseStream<S, T, Tok>
    where
        S: Stream<Item = Tok> + Unpin,
        T: IncrementalParse<Token = Tok> + Unpin,
        Tok: Clone + AsRef<Tok> + Unpin,
    {
        type Item = Result<T, StreamError>;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            // Return pending node if we have one
            if let Some(node) = self.pending_node.take() {
                return Poll::Ready(Some(Ok(node)));
            }

            // Try to get more tokens
            let this = self.get_mut();
            loop {
                match Pin::new(&mut this.inner).poll_next(cx) {
                    Poll::Ready(Some(token)) => {
                        this.token_buffer.push(token);

                        // Try to parse
                        if T::can_parse(&this.token_buffer, &this.checkpoint) {
                            match T::parse_incremental(&this.token_buffer, &this.checkpoint) {
                                Ok((Some(node), new_checkpoint)) => {
                                    this.checkpoint = new_checkpoint;
                                    return Poll::Ready(Some(Ok(node)));
                                }
                                Ok((None, _)) => {
                                    // Need more tokens
                                    continue;
                                }
                                Err(e) => {
                                    return Poll::Ready(Some(Err(StreamError::ParseError(
                                        e.to_string(),
                                    ))));
                                }
                            }
                        }
                    }
                    Poll::Ready(None) => {
                        // Stream ended - try final parse
                        if this.checkpoint.cursor < this.token_buffer.len() {
                            match T::parse_incremental(&this.token_buffer, &this.checkpoint) {
                                Ok((Some(node), new_checkpoint)) => {
                                    this.checkpoint = new_checkpoint;
                                    return Poll::Ready(Some(Ok(node)));
                                }
                                Ok((None, _)) if this.token_buffer.is_empty() => {
                                    return Poll::Ready(None);
                                }
                                Ok((None, _)) => {
                                    return Poll::Ready(Some(Err(StreamError::IncompleteInput)));
                                }
                                Err(e) => {
                                    return Poll::Ready(Some(Err(StreamError::ParseError(
                                        e.to_string(),
                                    ))));
                                }
                            }
                        }
                        return Poll::Ready(None);
                    }
                    Poll::Pending => {
                        return Poll::Pending;
                    }
                }
            }
        }
    }
}
