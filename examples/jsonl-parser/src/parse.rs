//! JSON Parsing Implementation
//!
//! This module provides parsers for JSON values following the synkit patterns.

use crate::stream::TokenStream;
use crate::tokens::Token;
use crate::{
    Diagnostic, JsonError, Parse, Peek,
    ast::{JsonLine, JsonLines, JsonObject, JsonValue, JsonValueKind},
};
use synkit::TokenStream as _;

// Helper to peek at raw tokens (including newlines)
fn peek_raw(stream: &TokenStream) -> Option<&Token> {
    stream.peek_token_raw().map(|t| &t.value)
}

// ANCHOR: parse_impl
impl Peek for JsonValue {
    fn is(token: &Token) -> bool {
        matches!(
            token,
            Token::Null
                | Token::True
                | Token::False
                | Token::Number(_)
                | Token::String(_)
                | Token::LBracket
                | Token::LBrace
        )
    }
}

impl Diagnostic for JsonValue {
    fn fmt() -> &'static str {
        "JSON value"
    }
}

impl Parse for JsonValue {
    fn parse(stream: &mut TokenStream) -> Result<Self, JsonError> {
        let start = stream.cursor();

        let Some(tok) = stream.peek_token() else {
            return Err(JsonError::empty::<Self>());
        };

        let kind = match &tok.value {
            Token::Null => {
                stream.next();
                JsonValueKind::Null
            }
            Token::True => {
                stream.next();
                JsonValueKind::Bool(true)
            }
            Token::False => {
                stream.next();
                JsonValueKind::Bool(false)
            }
            Token::Number(n) => {
                let n = n.clone();
                stream.next();
                JsonValueKind::Number(n)
            }
            Token::String(s) => {
                let s = s.clone();
                stream.next();
                JsonValueKind::String(s)
            }
            Token::LBracket => {
                let arr = parse_array(stream)?;
                JsonValueKind::Array(arr)
            }
            Token::LBrace => {
                let obj = parse_object(stream)?;
                JsonValueKind::Object(obj)
            }
            _ => {
                return Err(JsonError::expected::<Self>(&tok.value));
            }
        };

        let end = stream.cursor();
        let span = stream.span_range(start..end);

        Ok(JsonValue::new(kind, span))
    }
}

fn parse_array(stream: &mut TokenStream) -> Result<Vec<JsonValue>, JsonError> {
    // Consume opening bracket
    let Some(tok) = stream.peek_token() else {
        return Err(JsonError::Empty { expect: "'['" });
    };
    if !matches!(tok.value, Token::LBracket) {
        return Err(JsonError::Expected {
            expect: "'['",
            found: format!("{}", tok.value),
        });
    }
    stream.next();

    let mut values = Vec::new();

    // Check for empty array
    if let Some(tok) = stream.peek_token()
        && matches!(tok.value, Token::RBracket)
    {
        stream.next();
        return Ok(values);
    }

    // Parse first element
    values.push(JsonValue::parse(stream)?);

    // Parse remaining elements
    loop {
        let Some(tok) = stream.peek_token() else {
            return Err(JsonError::Empty {
                expect: "']' or ','",
            });
        };

        match &tok.value {
            Token::RBracket => {
                stream.next();
                break;
            }
            Token::Comma => {
                stream.next();
                values.push(JsonValue::parse(stream)?);
            }
            _ => {
                return Err(JsonError::Expected {
                    expect: "']' or ','",
                    found: format!("{}", tok.value),
                });
            }
        }
    }

    Ok(values)
}

fn parse_object(stream: &mut TokenStream) -> Result<JsonObject, JsonError> {
    // Consume opening brace
    let Some(tok) = stream.peek_token() else {
        return Err(JsonError::Empty { expect: "'{'" });
    };
    if !matches!(tok.value, Token::LBrace) {
        return Err(JsonError::Expected {
            expect: "'{'",
            found: format!("{}", tok.value),
        });
    }
    stream.next();

    let mut obj = JsonObject::new();

    // Check for empty object
    if let Some(tok) = stream.peek_token()
        && matches!(tok.value, Token::RBrace)
    {
        stream.next();
        return Ok(obj);
    }

    // Parse first key-value pair
    let (key, value) = parse_key_value(stream)?;
    obj.insert(key, value);

    // Parse remaining pairs
    loop {
        let Some(tok) = stream.peek_token() else {
            return Err(JsonError::Empty {
                expect: "'}' or ','",
            });
        };

        match &tok.value {
            Token::RBrace => {
                stream.next();
                break;
            }
            Token::Comma => {
                stream.next();
                let (key, value) = parse_key_value(stream)?;
                obj.insert(key, value);
            }
            _ => {
                return Err(JsonError::Expected {
                    expect: "'}' or ','",
                    found: format!("{}", tok.value),
                });
            }
        }
    }

    Ok(obj)
}

fn parse_key_value(stream: &mut TokenStream) -> Result<(String, JsonValue), JsonError> {
    // Parse key (must be string)
    let Some(tok) = stream.peek_token() else {
        return Err(JsonError::Empty {
            expect: "string key",
        });
    };

    let key = match &tok.value {
        Token::String(s) => {
            let s = s.clone();
            stream.next();
            s
        }
        _ => {
            return Err(JsonError::Expected {
                expect: "string key",
                found: format!("{}", tok.value),
            });
        }
    };

    // Parse colon
    let Some(tok) = stream.peek_token() else {
        return Err(JsonError::Empty { expect: "':'" });
    };
    if !matches!(tok.value, Token::Colon) {
        return Err(JsonError::Expected {
            expect: "':'",
            found: format!("{}", tok.value),
        });
    }
    stream.next();

    // Parse value
    let value = JsonValue::parse(stream)?;

    Ok((key, value))
}
// ANCHOR_END: parse_impl

// ANCHOR: jsonl_parse
impl Parse for JsonLine {
    fn parse(stream: &mut TokenStream) -> Result<Self, JsonError> {
        let start = stream.cursor();
        let value = JsonValue::parse(stream)?;
        let end = stream.cursor();
        let span = stream.span_range(start..end);

        // Consume trailing newline if present
        if let Some(tok) = peek_raw(stream)
            && matches!(tok, Token::Newline)
        {
            stream.next_raw();
        }

        Ok(JsonLine { value, span })
    }
}

impl Parse for JsonLines {
    fn parse(stream: &mut TokenStream) -> Result<Self, JsonError> {
        let mut lines = JsonLines::new();

        // Skip leading newlines
        while let Some(tok) = peek_raw(stream) {
            if matches!(tok, Token::Newline) {
                stream.next_raw();
            } else {
                break;
            }
        }

        while stream.peek_token().is_some() {
            let line = JsonLine::parse(stream)?;
            lines.push(line);

            // Skip any extra newlines between lines
            while let Some(tok) = peek_raw(stream) {
                if matches!(tok, Token::Newline) {
                    stream.next_raw();
                } else {
                    break;
                }
            }
        }

        Ok(lines)
    }
}
// ANCHOR_END: jsonl_parse

/// Parse a single JSON value from a string
pub fn parse_json(input: &str) -> Result<JsonValue, JsonError> {
    let mut stream = TokenStream::lex(input).map_err(|_| JsonError::Unknown)?;
    JsonValue::parse(&mut stream)
}

/// Parse a JSONL document from a string
pub fn parse_jsonl(input: &str) -> Result<JsonLines, JsonError> {
    let mut stream = TokenStream::lex(input).map_err(|_| JsonError::Unknown)?;
    JsonLines::parse(&mut stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_null() {
        let result = parse_json("null").unwrap();
        assert!(matches!(result.kind, JsonValueKind::Null));
    }

    #[test]
    fn test_parse_bool() {
        let t = parse_json("true").unwrap();
        let f = parse_json("false").unwrap();
        assert!(matches!(t.kind, JsonValueKind::Bool(true)));
        assert!(matches!(f.kind, JsonValueKind::Bool(false)));
    }

    #[test]
    fn test_parse_number() {
        let n = parse_json("42").unwrap();
        assert!(matches!(n.kind, JsonValueKind::Number(ref s) if s == "42"));

        let f = parse_json("-3.14e10").unwrap();
        assert!(matches!(f.kind, JsonValueKind::Number(ref s) if s == "-3.14e10"));
    }

    #[test]
    fn test_parse_string() {
        let s = parse_json(r#""hello world""#).unwrap();
        assert!(matches!(s.kind, JsonValueKind::String(ref v) if v == "hello world"));
    }

    #[test]
    fn test_parse_empty_array() {
        let arr = parse_json("[]").unwrap();
        assert!(matches!(arr.kind, JsonValueKind::Array(ref v) if v.is_empty()));
    }

    #[test]
    fn test_parse_array() {
        let arr = parse_json("[1, 2, 3]").unwrap();
        if let JsonValueKind::Array(v) = arr.kind {
            assert_eq!(v.len(), 3);
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn test_parse_nested_array() {
        let arr = parse_json("[[1, 2], [3, 4]]").unwrap();
        if let JsonValueKind::Array(v) = arr.kind {
            assert_eq!(v.len(), 2);
            assert!(matches!(&v[0].kind, JsonValueKind::Array(_)));
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn test_parse_empty_object() {
        let obj = parse_json("{}").unwrap();
        assert!(matches!(obj.kind, JsonValueKind::Object(ref o) if o.is_empty()));
    }

    #[test]
    fn test_parse_object() {
        let obj = parse_json(r#"{"name": "Alice", "age": 30}"#).unwrap();
        if let JsonValueKind::Object(o) = obj.kind {
            assert_eq!(o.len(), 2);
            assert!(o.get("name").is_some());
            assert!(o.get("age").is_some());
        } else {
            panic!("expected object");
        }
    }

    #[test]
    fn test_parse_nested_object() {
        let obj = parse_json(r#"{"user": {"name": "Bob", "active": true}}"#).unwrap();
        let user = obj.get("user").unwrap();
        assert!(matches!(&user.kind, JsonValueKind::Object(_)));
    }

    #[test]
    fn test_parse_jsonl() {
        let input = r#"{"id": 1}
{"id": 2}
{"id": 3}"#;
        let lines = parse_jsonl(input).unwrap();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_parse_jsonl_with_empty_lines() {
        let input = r#"{"id": 1}

{"id": 2}

{"id": 3}"#;
        let lines = parse_jsonl(input).unwrap();
        assert_eq!(lines.len(), 3);
    }
}
