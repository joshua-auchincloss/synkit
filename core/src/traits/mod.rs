//! Core traits for the synkit parsing framework.
//!
//! This module defines the fundamental traits that enable synkit's parsing
//! capabilities. Implementations of these traits work together to provide
//! a flexible, type-safe parsing infrastructure.
//!
//! # Trait Hierarchy
//!
//! ```text
//! TokenStream (stream interface)
//!     ├── parse::<T>() where T: Parse
//!     └── peek::<T>() where T: Peek
//!
//! SpanLike (position tracking)
//!     └── SpannedLike<T> (value + span)
//!
//! ToTokens (code generation)
//!     └── Printer (formatting)
//!
//! Diagnostic (error reporting)
//!     └── SpannedError (error + span)
//! ```
//!
//! # Usage Patterns
//!
//! ## Parsing
//!
//! ```ignore
//! use synkit::{TokenStream, Parse};
//!
//! fn parse_expression(stream: &mut impl TokenStream) -> Result<Expr, Error> {
//!     // Peek to decide which production to use
//!     if stream.peek::<NumberToken>() {
//!         let num = stream.parse::<NumberLiteral>()?;
//!         Ok(Expr::Number(num))
//!     } else if stream.peek::<IdentToken>() {
//!         let ident = stream.parse::<Identifier>()?;
//!         Ok(Expr::Ident(ident))
//!     } else {
//!         Err(Error::unexpected_token())
//!     }
//! }
//! ```
//!
//! ## Code Generation
//!
//! ```ignore
//! use synkit::{ToTokens, Printer};
//!
//! impl ToTokens for MyExpr {
//!     fn to_tokens(&self, printer: &mut impl Printer) {
//!         match self {
//!             MyExpr::Number(n) => printer.write(&n.to_string()),
//!             MyExpr::Ident(i) => printer.write(&i.name),
//!         }
//!     }
//! }
//! ```
//!
//! # Feature Flags
//!
//! - `std`: Enables `std::error::Error` implementations
//! - `serde`: Enables serialization for span types

mod diagnostic;
mod error;
mod parse;
mod peek;
mod printer;
mod stream;
mod to_tokens;

pub use diagnostic::Diagnostic;
pub use error::SpannedError;
pub use parse::Parse;
pub use peek::Peek;
pub use printer::Printer;
pub use stream::{SpanLike, SpannedLike, TokenStream};
pub use to_tokens::ToTokens;
