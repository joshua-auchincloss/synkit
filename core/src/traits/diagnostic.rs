/// Diagnostic formatting for error messages.
pub trait Diagnostic {
    /// Expected format string, e.g., "`{`" or "identifier".
    fn fmt() -> &'static str;
}
