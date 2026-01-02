# Visitors

The visitor pattern traverses AST nodes without modifying them.

## Visitor Trait

```rust,ignore
{{#include ../../../examples/toml-parser/src/visitor.rs:visitor_trait}}
```

Two method types:
- **`visit_*`**: Override to handle specific nodes, calls `walk_*` by default
- **`walk_*`**: Traverses children, typically not overridden

## Example: Collecting Keys

```rust,ignore
{{#include ../../../examples/toml-parser/src/visitor.rs:key_collector}}
```

Usage:

```rust,ignore
let mut collector = KeyCollector::new();
collector.visit_document(&doc.value);
// collector.keys now contains all key names
```

## Example: Counting Values

```rust,ignore
{{#include ../../../examples/toml-parser/src/visitor.rs:value_counter}}
```

## Example: Finding Tables

```rust,ignore
{{#include ../../../examples/toml-parser/src/visitor.rs:table_finder}}
```

## Visitor vs Direct Traversal

**Visitor pattern** when:
- Multiple traversal operations needed
- Want to separate traversal from logic
- Building analysis tools

**Direct recursion** when:
- One-off transformation
- Simple structure
- Need mutation

## Transforming Visitors

For mutation, use a mutable visitor or return new nodes:

```rust,ignore
pub trait TomlTransform {
    fn transform_value(&mut self, value: Value) -> Value {
        self.walk_value(value)
    }

    fn walk_value(&mut self, value: Value) -> Value {
        match value {
            Value::Array(arr) => Value::Array(self.transform_array(arr)),
            other => other,
        }
    }
    // ...
}
```

## Visitor Tips

### Selective Traversal

Override `visit_*` to stop descent:

```rust,ignore
fn visit_inline_table(&mut self, _table: &InlineTable) {
    // Don't call walk_inline_table - skip inline table contents
}
```

### Accumulating Results

Use struct fields:

```rust,ignore
struct Stats {
    tables: usize,
    keys: usize,
    values: usize,
}

impl TomlVisitor for Stats {
    fn visit_table(&mut self, table: &Table) {
        self.tables += 1;
        self.walk_table(table);
    }
    // ...
}
```

### Context Tracking

Track path during traversal:

```rust,ignore
struct PathTracker {
    path: Vec<String>,
    paths: Vec<String>,
}

impl TomlVisitor for PathTracker {
    fn visit_table(&mut self, table: &Table) {
        self.path.push(table_name(table));
        self.paths.push(self.path.join("."));
        self.walk_table(table);
        self.path.pop();
    }
}
```
