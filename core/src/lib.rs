#![deny(
    unsafe_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::dbg_macro
)]

pub mod config;
mod delimited;
mod error;
mod punctuated;
mod repeated;
pub mod traits;

#[cfg(any(feature = "tokio", feature = "futures"))]
pub mod async_stream;

pub use config::{ParseConfig, RecursionGuard};
pub use delimited::Delimited;
pub use error::Error;
pub use punctuated::{Punctuated, PunctuatedInner, Separated, Terminated, TrailingPolicy};
pub use repeated::{Repeated, RepeatedItem};
pub use traits::{
    Diagnostic, Parse, Peek, Printer, SpanLike, SpannedError, SpannedLike, ToTokens, TokenStream,
};
