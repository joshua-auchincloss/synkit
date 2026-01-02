//! Visitor pattern for traversing TOML AST

use crate::ast::*;

// ANCHOR: visitor_trait
/// Visitor trait for traversing TOML AST nodes.
///
/// Implement the `visit_*` methods you care about. Default implementations
/// call the corresponding `walk_*` methods to traverse children.
pub trait TomlVisitor {
    fn visit_document(&mut self, doc: &Document) {
        self.walk_document(doc);
    }

    fn visit_document_item(&mut self, item: &DocumentItem) {
        self.walk_document_item(item);
    }

    fn visit_key_value(&mut self, kv: &KeyValue) {
        self.walk_key_value(kv);
    }

    fn visit_key(&mut self, key: &Key) {
        self.walk_key(key);
    }

    fn visit_simple_key(&mut self, key: &SimpleKey) {
        let _ = key; // leaf node
    }

    fn visit_value(&mut self, value: &Value) {
        self.walk_value(value);
    }

    fn visit_table(&mut self, table: &Table) {
        self.walk_table(table);
    }

    fn visit_array(&mut self, array: &Array) {
        self.walk_array(array);
    }

    fn visit_inline_table(&mut self, table: &InlineTable) {
        self.walk_inline_table(table);
    }

    // Walk methods traverse child nodes

    fn walk_document(&mut self, doc: &Document) {
        for item in &doc.items {
            self.visit_document_item(item);
        }
    }

    fn walk_document_item(&mut self, item: &DocumentItem) {
        match item {
            DocumentItem::Trivia(_) => {}
            DocumentItem::KeyValue(kv) => self.visit_key_value(&kv.value),
            DocumentItem::Table(table) => self.visit_table(&table.value),
        }
    }

    fn walk_key_value(&mut self, kv: &KeyValue) {
        self.visit_key(&kv.key.value);
        self.visit_value(&kv.value.value);
    }

    fn walk_key(&mut self, key: &Key) {
        match key {
            Key::Bare(tok) => self.visit_simple_key(&SimpleKey::Bare(tok.clone())),
            Key::Quoted(tok) => self.visit_simple_key(&SimpleKey::Quoted(tok.clone())),
            Key::Dotted(dotted) => {
                self.visit_simple_key(&dotted.first.value);
                for (_, k) in &dotted.rest {
                    self.visit_simple_key(&k.value);
                }
            }
        }
    }

    fn walk_value(&mut self, value: &Value) {
        match value {
            Value::Array(arr) => self.visit_array(arr),
            Value::InlineTable(tbl) => self.visit_inline_table(tbl),
            _ => {}
        }
    }

    fn walk_table(&mut self, table: &Table) {
        self.visit_key(&table.name.value);
        for item in &table.items {
            match item {
                TableItem::Trivia(_) => {}
                TableItem::KeyValue(kv) => self.visit_key_value(&kv.value),
            }
        }
    }

    fn walk_array(&mut self, array: &Array) {
        for item in &array.items {
            self.visit_value(&item.value.value);
        }
    }

    fn walk_inline_table(&mut self, table: &InlineTable) {
        for item in &table.items {
            self.visit_key_value(&item.kv.value);
        }
    }
}
// ANCHOR_END: visitor_trait

// ANCHOR: key_collector
/// Example visitor: collects all keys in the document.
pub struct KeyCollector {
    pub keys: Vec<String>,
}

impl KeyCollector {
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }

    pub fn collect(doc: &Document) -> Vec<String> {
        let mut collector = Self::new();
        collector.visit_document(doc);
        collector.keys
    }
}

impl Default for KeyCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl TomlVisitor for KeyCollector {
    fn visit_simple_key(&mut self, key: &SimpleKey) {
        let name = match key {
            SimpleKey::Bare(tok) => tok.0.clone(),
            SimpleKey::Quoted(tok) => tok.0.clone(),
        };
        self.keys.push(name);
    }
}
// ANCHOR_END: key_collector

// ANCHOR: value_counter
/// Example visitor: counts values by type.
#[derive(Default, Debug)]
pub struct ValueCounter {
    pub strings: usize,
    pub integers: usize,
    pub booleans: usize,
    pub arrays: usize,
    pub inline_tables: usize,
}

impl ValueCounter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn count(doc: &Document) -> Self {
        let mut counter = Self::new();
        counter.visit_document(doc);
        counter
    }
}

impl TomlVisitor for ValueCounter {
    fn visit_value(&mut self, value: &Value) {
        match value {
            Value::String(_) => self.strings += 1,
            Value::Integer(_) => self.integers += 1,
            Value::True(_) | Value::False(_) => self.booleans += 1,
            Value::Array(arr) => {
                self.arrays += 1;
                self.visit_array(arr);
            }
            Value::InlineTable(tbl) => {
                self.inline_tables += 1;
                self.visit_inline_table(tbl);
            }
        }
    }
}
// ANCHOR_END: value_counter

// ANCHOR: table_finder
/// Example visitor: finds all table names.
pub struct TableFinder {
    pub tables: Vec<String>,
}

impl TableFinder {
    pub fn new() -> Self {
        Self { tables: Vec::new() }
    }

    pub fn find(doc: &Document) -> Vec<String> {
        let mut finder = Self::new();
        finder.visit_document(doc);
        finder.tables
    }

    fn key_to_string(key: &Key) -> String {
        match key {
            Key::Bare(tok) => tok.0.clone(),
            Key::Quoted(tok) => format!("\"{}\"", tok.0),
            Key::Dotted(dotted) => {
                let mut parts = vec![Self::simple_key_to_string(&dotted.first.value)];
                for (_, k) in &dotted.rest {
                    parts.push(Self::simple_key_to_string(&k.value));
                }
                parts.join(".")
            }
        }
    }

    fn simple_key_to_string(key: &SimpleKey) -> String {
        match key {
            SimpleKey::Bare(tok) => tok.0.clone(),
            SimpleKey::Quoted(tok) => format!("\"{}\"", tok.0),
        }
    }
}

impl Default for TableFinder {
    fn default() -> Self {
        Self::new()
    }
}

impl TomlVisitor for TableFinder {
    fn visit_table(&mut self, table: &Table) {
        self.tables.push(Self::key_to_string(&table.name.value));
        self.walk_table(table);
    }
}
// ANCHOR_END: table_finder

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, TokenStream};

    fn parse_doc(input: &str) -> Document {
        let mut stream = TokenStream::lex(input).unwrap();
        stream.parse::<Document>().unwrap().value
    }

    #[test]
    fn test_key_collector() {
        let doc = parse_doc(
            r#"
name = "test"
version = "1.0"

[package]
author = "me"
"#,
        );

        let keys = KeyCollector::collect(&doc);
        assert!(keys.contains(&"name".to_string()));
        assert!(keys.contains(&"version".to_string()));
        assert!(keys.contains(&"package".to_string()));
        assert!(keys.contains(&"author".to_string()));
    }

    #[test]
    fn test_value_counter() {
        let doc = parse_doc(
            r#"
name = "test"
count = 42
enabled = true
tags = [1, 2, 3]
"#,
        );

        let counts = ValueCounter::count(&doc);
        assert_eq!(counts.strings, 1);
        assert_eq!(counts.integers, 4); // 42 + 1 + 2 + 3
        assert_eq!(counts.booleans, 1);
        assert_eq!(counts.arrays, 1);
    }

    #[test]
    fn test_table_finder() {
        let doc = parse_doc(
            r#"
[package]
name = "test"

[dependencies]
foo = "1.0"

[dev.nested]
bar = "2.0"
"#,
        );

        let tables = TableFinder::find(&doc);
        assert!(tables.contains(&"package".to_string()));
        assert!(tables.contains(&"dependencies".to_string()));
        assert!(tables.contains(&"dev.nested".to_string()));
    }
}
