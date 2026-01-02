use super::to_tokens::ToTokens;

/// Trait for building formatted text output.
///
/// `Printer` provides a structured way to generate formatted text with
/// support for indentation, whitespace control, and token formatting.
/// It's used by [`ToTokens`] implementations to produce output.
///
/// # Associated Types
///
/// - `Token`: The token type for grammar-specific formatting
///
/// # Required Methods
///
/// - `buf()`: Get current buffer contents
/// - `buf_mut()`: Get mutable buffer for appending
/// - `indent_level()`: Current indentation depth
/// - `set_indent(level)`: Set indentation depth
/// - `into_string()`: Consume and return final output
/// - `token(t)`: Format a token (grammar-specific)
///
/// # Provided Methods
///
/// Basic output:
/// - `word(s)`, `char(c)`: Append text
/// - `space()`, `spaces(n)`, `tab()`, `tabs(n)`: Whitespace
/// - `newline()`: Newline with auto-indent
///
/// Indentation:
/// - `indent()`, `dedent()`: Change indent level
/// - `open_block(token)`, `close_block(token)`: Block delimiters
///
/// Structured output:
/// - `write(value)`: Write a `ToTokens` value
/// - `write_separated(items, sep, ...)`: Write items with separators
///
/// # Example
///
/// ```ignore
/// use synkit::Printer;
///
/// #[derive(Default)]
/// struct MyPrinter {
///     buf: String,
///     indent: usize,
/// }
///
/// impl Printer for MyPrinter {
///     type Token = MyTok;
///
///     fn buf(&self) -> &str { &self.buf }
///     fn buf_mut(&mut self) -> &mut String { &mut self.buf }
///     fn indent_level(&self) -> usize { self.indent }
///     fn set_indent(&mut self, level: usize) { self.indent = level; }
///     fn into_string(self) -> String { self.buf }
///
///     fn token(&mut self, t: &Self::Token) {
///         match t {
///             MyTok::Plus => self.word("+"),
///             MyTok::Minus => self.word("-"),
///             // ...
///         }
///     }
/// }
/// ```
pub trait Printer: Sized {
    /// The token type for grammar-specific formatting.
    type Token;

    /// Get the current buffer contents.
    fn buf(&self) -> &str;
    /// Get a mutable reference to the buffer for appending.
    fn buf_mut(&mut self) -> &mut String;
    /// Get the current indentation level.
    fn indent_level(&self) -> usize;
    /// Set the indentation level.
    fn set_indent(&mut self, level: usize);
    /// Consume the printer and return the final string.
    fn into_string(self) -> String;

    /// Format a token to text.
    ///
    /// This is grammar-specific and should convert tokens to their
    /// textual representation (e.g., `Plus` → `"+"`).
    fn token(&mut self, t: &Self::Token);

    /// Append a string to the buffer.
    fn word(&mut self, s: &str) {
        self.buf_mut().push_str(s);
    }

    /// Append a single character to the buffer.
    fn char(&mut self, c: char) {
        self.buf_mut().push(c);
    }

    /// Append a single space.
    fn space(&mut self) {
        self.char(' ');
    }

    /// Append multiple spaces.
    fn spaces(&mut self, n: usize) {
        self.buf_mut().extend(std::iter::repeat_n(' ', n));
    }

    /// Append a single tab.
    fn tab(&mut self) {
        self.char('\t');
    }

    /// Append multiple tabs.
    fn tabs(&mut self, n: usize) {
        self.buf_mut().extend(std::iter::repeat_n('\t', n));
    }

    /// Append a newline and auto-indent.
    fn newline(&mut self) {
        self.char('\n');
        self.add_indent();
    }

    /// Add indentation at the current level.
    fn add_indent(&mut self) {
        if self.use_tabs() {
            self.tabs(self.indent_level());
        } else {
            self.spaces(self.spaces_width());
        }
    }

    /// Get the number of spaces per indent level.
    ///
    /// Default: 4 spaces
    fn indent_width(&self) -> usize {
        4
    }

    /// Calculate total spaces for current indent level.
    fn spaces_width(&self) -> usize {
        self.indent_level() * self.indent_width()
    }

    /// Whether to use tabs for indentation.
    ///
    /// Default: `true` (tabs)
    fn use_tabs(&self) -> bool {
        true
    }

    /// Increase indentation level by 1.
    fn indent(&mut self) {
        self.set_indent(self.indent_level() + 1);
    }

    /// Decrease indentation level by 1.
    ///
    /// Saturates at 0 (won't go negative).
    fn dedent(&mut self) {
        let level = self.indent_level();
        if level > 0 {
            self.set_indent(level - 1);
        }
    }

    /// Open a block: write token, indent, newline.
    fn open_block(&mut self, open: &Self::Token) {
        self.token(open);
        self.indent();
        self.newline();
    }

    /// Close a block: dedent, newline, write token.
    fn close_block(&mut self, close: &Self::Token) {
        self.dedent();
        self.newline();
        self.token(close);
    }

    /// Write a value implementing `ToTokens`.
    fn write<T: ToTokens<Printer = Self>>(&mut self, value: &T) {
        value.write(self);
    }

    /// Write items separated by a delimiter token.
    ///
    /// # Arguments
    ///
    /// * `items` - Iterator of items to write
    /// * `sep` - Separator token between items
    /// * `trailing` - Whether to add separator after last item
    /// * `newline_after_sep` - Whether to add newline after each separator
    fn write_separated<T, I>(
        &mut self,
        items: I,
        sep: &Self::Token,
        trailing: bool,
        newline_after_sep: bool,
    ) where
        T: ToTokens<Printer = Self>,
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        let iter = items.into_iter();
        let len = iter.len();
        for (idx, item) in iter.enumerate() {
            self.write(&item);
            let is_last = idx == len - 1;
            if !is_last || trailing {
                self.token(sep);
                if newline_after_sep && !is_last {
                    self.newline();
                }
            }
        }
    }

    /// Write items with inline spacing (space after separator, no newlines).
    fn write_separated_inline<T, I>(&mut self, items: I, sep: &Self::Token)
    where
        T: ToTokens<Printer = Self>,
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        let iter = items.into_iter();
        let len = iter.len();
        for (idx, item) in iter.enumerate() {
            self.write(&item);
            if idx < len - 1 {
                self.token(sep);
                self.space();
            }
        }
    }
}
