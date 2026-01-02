//! JSON Abstract Syntax Tree
//!
//! This module defines the AST types for JSON values.

use crate::Span;
use std::collections::HashMap;

// ANCHOR: ast_types
/// A JSON value with its span information
#[derive(Debug, Clone, PartialEq)]
pub struct JsonValue {
    pub kind: JsonValueKind,
    pub span: Span,
}

/// The kind of JSON value
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValueKind {
    /// `null`
    Null,
    /// `true` or `false`
    Bool(bool),
    /// A number (stored as string to preserve precision)
    Number(String),
    /// A string value
    String(String),
    /// An array `[...]`
    Array(Vec<JsonValue>),
    /// An object `{...}`
    Object(JsonObject),
}

/// A JSON object with ordered keys
#[derive(Debug, Clone, PartialEq, Default)]
pub struct JsonObject {
    /// Key-value pairs in insertion order
    pub entries: Vec<(String, JsonValue)>,
}

/// A single line in JSONL format
#[derive(Debug, Clone, PartialEq)]
pub struct JsonLine {
    pub value: JsonValue,
    pub span: Span,
}

/// A JSONL document (sequence of JSON values)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct JsonLines {
    pub lines: Vec<JsonLine>,
}
// ANCHOR_END: ast_types

impl JsonValue {
    pub fn new(kind: JsonValueKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn null(span: Span) -> Self {
        Self::new(JsonValueKind::Null, span)
    }

    pub fn bool(value: bool, span: Span) -> Self {
        Self::new(JsonValueKind::Bool(value), span)
    }

    pub fn number(value: String, span: Span) -> Self {
        Self::new(JsonValueKind::Number(value), span)
    }

    pub fn string(value: String, span: Span) -> Self {
        Self::new(JsonValueKind::String(value), span)
    }

    pub fn array(values: Vec<JsonValue>, span: Span) -> Self {
        Self::new(JsonValueKind::Array(values), span)
    }

    pub fn object(obj: JsonObject, span: Span) -> Self {
        Self::new(JsonValueKind::Object(obj), span)
    }

    /// Convert to a standard HashMap (loses key ordering)
    pub fn as_object(&self) -> Option<HashMap<&str, &JsonValue>> {
        match &self.kind {
            JsonValueKind::Object(obj) => {
                Some(obj.entries.iter().map(|(k, v)| (k.as_str(), v)).collect())
            }
            _ => None,
        }
    }

    /// Get a value from an object by key
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match &self.kind {
            JsonValueKind::Object(obj) => {
                obj.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
            }
            _ => None,
        }
    }

    /// Get array length
    pub fn len(&self) -> Option<usize> {
        match &self.kind {
            JsonValueKind::Array(arr) => Some(arr.len()),
            JsonValueKind::Object(obj) => Some(obj.entries.len()),
            _ => None,
        }
    }
}

impl JsonObject {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: String, value: JsonValue) {
        self.entries.push((key, value));
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl JsonLines {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    pub fn push(&mut self, line: JsonLine) {
        self.lines.push(line);
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &JsonLine> {
        self.lines.iter()
    }
}

impl IntoIterator for JsonLines {
    type Item = JsonLine;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.lines.into_iter()
    }
}

// Memory size estimation for monitoring
impl JsonValue {
    /// Estimate the memory size of this value in bytes (rough approximation)
    pub fn estimated_size(&self) -> usize {
        let base = std::mem::size_of::<Self>();
        let content = match &self.kind {
            JsonValueKind::Null => 0,
            JsonValueKind::Bool(_) => 0,
            JsonValueKind::Number(s) => s.capacity(),
            JsonValueKind::String(s) => s.capacity(),
            JsonValueKind::Array(arr) => {
                arr.capacity() * std::mem::size_of::<JsonValue>()
                    + arr.iter().map(|v| v.estimated_size()).sum::<usize>()
            }
            JsonValueKind::Object(obj) => {
                obj.entries.capacity() * std::mem::size_of::<(String, JsonValue)>()
                    + obj
                        .entries
                        .iter()
                        .map(|(k, v)| k.capacity() + v.estimated_size())
                        .sum::<usize>()
            }
        };
        base + content
    }
}
