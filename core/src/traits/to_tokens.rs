use super::printer::Printer;

/// Trait for converting AST nodes back to text.
///
/// `ToTokens` is the inverse of `Parse` - it converts parsed structures
/// back into their textual representation. This enables:
///
/// - Code formatting / pretty-printing
/// - Source-to-source transformations
/// - Round-trip testing (parse → modify → print)
///
/// # Associated Types
///
/// - [`Printer`]: The printer implementation for formatting output
///
/// # Required Methods
///
/// - `write(&self, printer)`: Write this value to the printer
///
/// # Provided Methods
///
/// - `to_string_formatted()`: Convenience method for getting a String
///
/// # Example
///
/// ```ignore
/// use synkit::{ToTokens, Printer};
///
/// struct BinaryExpr {
///     left: Box<Expr>,
///     op: &'static str,
///     right: Box<Expr>,
/// }
///
/// impl ToTokens for BinaryExpr {
///     type Printer = MyPrinter;
///
///     fn write(&self, p: &mut Self::Printer) {
///         self.left.write(p);
///         p.write(" ");
///         p.write(self.op);
///         p.write(" ");
///         self.right.write(p);
///     }
/// }
/// ```
///
/// # Blanket Implementations
///
/// - `Option<T>`: Writes nothing for `None`, delegates for `Some`
/// - `Box<T>`: Delegates to inner value
/// - `Vec<T>`: Writes each element in sequence
/// - `&T`: Delegates to referenced value
pub trait ToTokens {
    /// The printer type for formatting output.
    type Printer: Printer;

    /// Write this value to the printer.
    ///
    /// # Arguments
    ///
    /// * `printer` - The printer to write to
    fn write(&self, printer: &mut Self::Printer);

    /// Convert to a formatted string.
    ///
    /// Convenience method that creates a default printer, writes to it,
    /// and returns the result as a String.
    ///
    /// # Returns
    ///
    /// The formatted string representation
    fn to_string_formatted(&self) -> String
    where
        Self::Printer: Default,
    {
        let mut printer = Self::Printer::default();
        self.write(&mut printer);
        printer.into_string()
    }
}

impl<T: ToTokens> ToTokens for Option<T> {
    type Printer = T::Printer;

    fn write(&self, p: &mut Self::Printer) {
        if let Some(v) = self {
            v.write(p);
        }
    }
}

impl<T: ToTokens> ToTokens for Box<T> {
    type Printer = T::Printer;

    fn write(&self, p: &mut Self::Printer) {
        self.as_ref().write(p);
    }
}

impl<T: ToTokens> ToTokens for Vec<T> {
    type Printer = T::Printer;

    fn write(&self, p: &mut Self::Printer) {
        for item in self {
            item.write(p);
        }
    }
}

impl<T: ToTokens> ToTokens for &T {
    type Printer = T::Printer;

    fn write(&self, p: &mut Self::Printer) {
        (*self).write(p);
    }
}
