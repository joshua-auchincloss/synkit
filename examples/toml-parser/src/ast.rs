//! AST node definitions for TOML
//!
//! These types represent the structure of a TOML document.

use crate::{Spanned, tokens};

// ANCHOR: document
/// The root of a TOML document.
/// Contains a sequence of items (key-value pairs or tables).
#[derive(Debug, Clone)]
pub struct Document {
    pub items: Vec<DocumentItem>,
}

/// A single item in the document: either a top-level key-value or a table section.
#[derive(Debug, Clone)]
pub enum DocumentItem {
    /// A blank line or comment
    Trivia(Trivia),
    /// A key = value pair at the top level
    KeyValue(Spanned<KeyValue>),
    /// A [table] section
    Table(Spanned<Table>),
}
// ANCHOR_END: document

// ANCHOR: trivia
/// Trivia represents non-semantic content: newlines and comments.
#[derive(Debug, Clone)]
pub enum Trivia {
    Newline(Spanned<tokens::NewlineToken>),
    Comment(Spanned<tokens::CommentToken>),
}
// ANCHOR_END: trivia

// ANCHOR: key_value
/// A key-value pair: `key = value`
#[derive(Debug, Clone)]
pub struct KeyValue {
    pub key: Spanned<Key>,
    pub eq: Spanned<tokens::EqToken>,
    pub value: Spanned<Value>,
}
// ANCHOR_END: key_value

// ANCHOR: key
/// A TOML key, which can be bare, quoted, or dotted.
#[derive(Debug, Clone)]
pub enum Key {
    /// Bare key: `foo`
    Bare(tokens::BareKeyToken),
    /// Quoted key: `"foo.bar"`
    Quoted(tokens::BasicStringToken),
    /// Dotted key: `foo.bar.baz`
    Dotted(DottedKey),
}

/// A dotted key like `server.host.name`
#[derive(Debug, Clone)]
pub struct DottedKey {
    pub first: Spanned<SimpleKey>,
    pub rest: Vec<(Spanned<tokens::DotToken>, Spanned<SimpleKey>)>,
}

/// A simple (non-dotted) key
#[derive(Debug, Clone)]
pub enum SimpleKey {
    Bare(tokens::BareKeyToken),
    Quoted(tokens::BasicStringToken),
}
// ANCHOR_END: key

// ANCHOR: value
/// A TOML value.
#[derive(Debug, Clone)]
pub enum Value {
    /// String value
    String(tokens::BasicStringToken),
    /// Integer value
    Integer(tokens::IntegerToken),
    /// Boolean true
    True(tokens::TrueToken),
    /// Boolean false
    False(tokens::FalseToken),
    /// Array value
    Array(Array),
    /// Inline table value
    InlineTable(InlineTable),
}
// ANCHOR_END: value

// ANCHOR: table
/// A table section: `[section]` or `[section.subsection]`
#[derive(Debug, Clone)]
pub struct Table {
    pub lbracket: Spanned<tokens::LBracketToken>,
    pub name: Spanned<Key>,
    pub rbracket: Spanned<tokens::RBracketToken>,
    pub items: Vec<TableItem>,
}

/// An item within a table section.
#[derive(Debug, Clone)]
pub enum TableItem {
    Trivia(Trivia),
    KeyValue(Spanned<KeyValue>),
}
// ANCHOR_END: table

// ANCHOR: array
/// An array: `[1, 2, 3]`
#[derive(Debug, Clone)]
pub struct Array {
    pub lbracket: Spanned<tokens::LBracketToken>,
    pub items: Vec<ArrayItem>,
    pub rbracket: Spanned<tokens::RBracketToken>,
}

/// An item in an array, including trailing trivia.
#[derive(Debug, Clone)]
pub struct ArrayItem {
    pub value: Spanned<Value>,
    pub comma: Option<Spanned<tokens::CommaToken>>,
}
// ANCHOR_END: array

// ANCHOR: inline_table
/// An inline table: `{ key = value, ... }`
#[derive(Debug, Clone)]
pub struct InlineTable {
    pub lbrace: Spanned<tokens::LBraceToken>,
    pub items: Vec<InlineTableItem>,
    pub rbrace: Spanned<tokens::RBraceToken>,
}

/// An item in an inline table.
#[derive(Debug, Clone)]
pub struct InlineTableItem {
    pub kv: Spanned<KeyValue>,
    pub comma: Option<Spanned<tokens::CommaToken>>,
}
// ANCHOR_END: inline_table
