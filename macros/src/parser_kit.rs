use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Ident, Path, Token, braced, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

use crate::declare_tokens::{DeclareTokensInput, TokenDef};

pub struct ParserKitInput {
    pub error_type: Ident,
    pub skip_tokens: Vec<Ident>,
    pub logos_attrs: Vec<Attribute>,
    pub tokens: Vec<TokenDef>,
    pub delimiters: Vec<DelimiterDef>,
    pub span_derives: Vec<Path>,
    pub token_derives: Vec<Path>,
    pub custom_derives: Vec<Path>,
}

pub struct DelimiterDef {
    pub name: Ident,
    pub open: Ident,
    pub close: Ident,
}

impl Parse for ParserKitInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut error_type = None;
        let mut skip_tokens = Vec::new();
        let mut logos_attrs = Vec::new();
        let mut tokens = Vec::new();
        let mut delimiters = Vec::new();
        let mut span_derives = Vec::new();
        let mut token_derives = Vec::new();
        let mut custom_derives = Vec::new();

        while !input.is_empty() {
            if input.peek(Token![#]) {
                let attr = input.call(Attribute::parse_outer)?;
                logos_attrs.extend(attr);
                continue;
            }

            let ident: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            match ident.to_string().as_str() {
                "error" => {
                    error_type = Some(input.parse()?);
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                }
                "skip_tokens" => {
                    let content;
                    bracketed!(content in input);
                    skip_tokens = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?
                        .into_iter()
                        .collect();
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                }
                "tokens" => {
                    let content;
                    braced!(content in input);
                    while !content.is_empty() {
                        tokens.push(content.parse()?);
                        if content.peek(Token![,]) {
                            content.parse::<Token![,]>()?;
                        }
                    }
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                }
                "delimiters" => {
                    let content;
                    braced!(content in input);
                    while !content.is_empty() {
                        let name: Ident = content.parse()?;
                        content.parse::<Token![=>]>()?;
                        let inner;
                        syn::parenthesized!(inner in content);
                        let open: Ident = inner.parse()?;
                        inner.parse::<Token![,]>()?;
                        let close: Ident = inner.parse()?;
                        delimiters.push(DelimiterDef { name, open, close });
                        if content.peek(Token![,]) {
                            content.parse::<Token![,]>()?;
                        }
                    }
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                }
                "span_derives" => {
                    let content;
                    bracketed!(content in input);
                    span_derives = Punctuated::<Path, Token![,]>::parse_terminated(&content)?
                        .into_iter()
                        .collect();
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                }
                "token_derives" => {
                    let content;
                    bracketed!(content in input);
                    token_derives = Punctuated::<Path, Token![,]>::parse_terminated(&content)?
                        .into_iter()
                        .collect();
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                }
                "custom_derives" => {
                    let content;
                    bracketed!(content in input);
                    custom_derives = Punctuated::<Path, Token![,]>::parse_terminated(&content)?
                        .into_iter()
                        .collect();
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown field: {}", other),
                    ));
                }
            }
        }

        let error_type =
            error_type.ok_or_else(|| syn::Error::new(input.span(), "missing `error` field"))?;

        Ok(Self {
            error_type,
            skip_tokens,
            logos_attrs,
            tokens,
            delimiters,
            span_derives,
            token_derives,
            custom_derives,
        })
    }
}

pub fn expand(input: ParserKitInput) -> syn::Result<TokenStream> {
    let ParserKitInput {
        error_type,
        skip_tokens,
        logos_attrs,
        tokens,
        delimiters,
        span_derives,
        token_derives,
        custom_derives,
    } = input;

    let span_derives_tokens = if span_derives.is_empty() {
        quote! { Debug, Clone, PartialEq, Eq, Hash, Copy }
    } else {
        quote! { #(#span_derives),* }
    };

    let custom_derives_attr = if custom_derives.is_empty() {
        quote! {}
    } else {
        quote! { #[derive(#(#custom_derives),*)] }
    };

    let span_module = quote! {
        pub mod span {
            /// Raw byte span with start and end offsets.
            ///
            /// Layout: 16 bytes on 64-bit (2 × usize), 8-byte aligned.
            #[derive(#span_derives_tokens)]
            #custom_derives_attr
            #[repr(C)]
            pub struct RawSpan {
                pub start: usize,
                pub end: usize,
            }

            /// Source location span, either known or synthetic (call-site).
            ///
            /// Layout: 24 bytes on 64-bit (8-byte discriminant region + 16 bytes data).
            /// Uses `usize::MAX` sentinel in start position for CallSite to enable
            /// future niche optimization if needed.
            #[derive(#span_derives_tokens)]
            #custom_derives_attr
            pub enum Span {
                CallSite,
                Known(RawSpan),
            }

            impl Span {
                #[inline]
                pub fn new(start: usize, end: usize) -> Self {
                    Self::Known(RawSpan { start, end })
                }

                #[inline]
                pub fn call_site() -> Self {
                    Self::CallSite
                }

                #[inline]
                pub fn len(&self) -> usize {
                    match self {
                        Self::Known(s) => s.end.saturating_sub(s.start),
                        Self::CallSite => 0,
                    }
                }

                #[inline]
                pub fn is_empty(&self) -> bool {
                    self.len() == 0
                }

                #[inline]
                pub fn raw(&self) -> RawSpan {
                    match self {
                        Self::Known(s) => *s,
                        Self::CallSite => RawSpan { start: 0, end: 0 },
                    }
                }

                #[inline]
                pub fn join(&self, other: &Self) -> Self {
                    match (self, other) {
                        (Self::Known(a), Self::Known(b)) => {
                            Self::new(a.start.min(b.start), a.end.max(b.end))
                        }
                        (Self::Known(s), _) | (_, Self::Known(s)) => Self::Known(*s),
                        _ => Self::CallSite,
                    }
                }
            }

            impl synkit::SpanLike for Span {
                #[inline]
                fn start(&self) -> usize {
                    self.raw().start
                }

                #[inline]
                fn end(&self) -> usize {
                    self.raw().end
                }

                #[inline]
                fn new(start: usize, end: usize) -> Self {
                    Self::new(start, end)
                }

                #[inline]
                fn call_site() -> Self {
                    Self::CallSite
                }
            }

            /// A value with associated source span.
            ///
            /// Field order optimized: span first (8-byte aligned) ensures T
            /// starts at optimal offset regardless of T's alignment.
            #[derive(Debug, Clone)]
            #custom_derives_attr
            #[repr(C)]
            pub struct Spanned<T> {
                pub span: Span,
                pub value: T,
            }

            impl<T> Spanned<T> {
                #[inline]
                pub fn new(start: usize, end: usize, value: T) -> Self {
                    Self {
                        span: Span::new(start, end),
                        value,
                    }
                }

                #[inline]
                pub fn call_site(value: T) -> Self {
                    Self {
                        span: Span::CallSite,
                        value,
                    }
                }

                #[inline]
                pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
                    Spanned {
                        span: self.span,
                        value: f(self.value),
                    }
                }

                #[inline]
                pub fn as_ref(&self) -> Spanned<&T> {
                    Spanned {
                        span: self.span.clone(),
                        value: &self.value,
                    }
                }
            }

            impl<T> std::ops::Deref for Spanned<T> {
                type Target = T;
                fn deref(&self) -> &Self::Target {
                    &self.value
                }
            }

            impl<T: Clone> synkit::SpannedLike<T> for Spanned<T> {
                type Span = Span;

                fn span(&self) -> &Span {
                    &self.span
                }

                fn value_ref(&self) -> &T {
                    &self.value
                }

                fn value(self) -> T {
                    self.value
                }

                fn new(start: usize, end: usize, value: T) -> Self {
                    Self::new(start, end, value)
                }
            }

            // Compile-time layout assertions for 64-bit platforms
            #[cfg(target_pointer_width = "64")]
            const _: () = {
                use core::mem::{size_of, align_of};

                // RawSpan: 16 bytes, 8-byte aligned (2 × usize)
                const _RAW_SPAN_SIZE: () = assert!(size_of::<RawSpan>() == 16);
                const _RAW_SPAN_ALIGN: () = assert!(align_of::<RawSpan>() == 8);

                // Span: 24 bytes (8 discriminant + 16 data), 8-byte aligned
                const _SPAN_SIZE: () = assert!(size_of::<Span>() == 24);
                const _SPAN_ALIGN: () = assert!(align_of::<Span>() == 8);

                // Spanned<u8>: 32 bytes (24 span + 1 value + 7 padding)
                const _SPANNED_U8_SIZE: () = assert!(size_of::<Spanned<u8>>() == 32);

                // Spanned<usize>: 32 bytes (24 span + 8 value)
                const _SPANNED_USIZE_SIZE: () = assert!(size_of::<Spanned<usize>>() == 32);
            };
        }
    };

    let declare_tokens_input = DeclareTokensInput {
        span_mod: None,
        error_type: error_type.clone(),
        derives: token_derives.clone(),
        struct_derives: token_derives.clone(),
        logos_attrs,
        tokens: tokens.clone(),
    };

    let tokens_expanded = crate::declare_tokens::expand(declare_tokens_input)?;

    let tokens_module = quote! {
        pub mod tokens {
            #tokens_expanded
        }
    };

    let skip_patterns: Vec<_> = skip_tokens
        .iter()
        .map(|name| {
            quote! { super::tokens::Token::#name }
        })
        .collect();

    let skip_match = if skip_patterns.is_empty() {
        quote! { false }
    } else {
        quote! { matches!(tok.value, #(#skip_patterns)|*) }
    };

    let stream_module = quote! {
        pub mod stream {
            use std::sync::Arc;
            use std::path::Path;
            use super::span::{Span, Spanned};
            use super::tokens::{Token, SpannedToken};

            pub struct TokenStream {
                source: Arc<str>,
                source_path: Option<Arc<Path>>,
                tokens: Arc<Vec<SpannedToken>>,
                cursor: usize,
                range_start: usize,
                range_end: usize,
                last_cursor: usize,
            }

            impl TokenStream {
                pub fn lex(source: &str) -> Result<Self, super::#error_type> {
                    Self::lex_with_path(source, None::<&Path>)
                }

                pub fn lex_with_path(
                    source: &str,
                    path: Option<impl AsRef<Path>>,
                ) -> Result<Self, super::#error_type> {
                    use logos::Logos;
                    let source: Arc<str> = Arc::from(source);
                    let mut lex = Token::lexer(&source);
                    let mut tokens = Vec::new();

                    while let Some(tok) = lex.next() {
                        let span = lex.span();
                        let tok = tok?;
                        tokens.push(Spanned::new(span.start, span.end, tok));
                    }

                    let len = tokens.len();
                    Ok(Self {
                        source,
                        source_path: path.map(|p| Arc::from(p.as_ref())),
                        tokens: Arc::new(tokens),
                        cursor: 0,
                        range_start: 0,
                        range_end: len,
                        last_cursor: 0,
                    })
                }

                /// Create a TokenStream from pre-lexed tokens.
                ///
                /// This is the zero-copy path for incremental parsing: tokens are
                /// borrowed via `Arc` without re-lexing.
                ///
                /// # Arguments
                /// * `source` - The original source text (for span slicing)
                /// * `tokens` - Pre-lexed tokens to parse
                ///
                /// # Example
                /// ```ignore
                /// let tokens = Arc::new(lexed_tokens);
                /// let source: Arc<str> = Arc::from(source_text);
                /// let stream = TokenStream::from_tokens(source, tokens);
                /// let value: MyAst = stream.parse()?;
                /// ```
                pub fn from_tokens(source: Arc<str>, tokens: Arc<Vec<SpannedToken>>) -> Self {
                    let len = tokens.len();
                    Self {
                        source,
                        source_path: None,
                        tokens,
                        cursor: 0,
                        range_start: 0,
                        range_end: len,
                        last_cursor: 0,
                    }
                }

                /// Create a TokenStream from a range of pre-lexed tokens.
                ///
                /// This allows parsing a subset of tokens without copying.
                pub fn from_tokens_range(
                    source: Arc<str>,
                    tokens: Arc<Vec<SpannedToken>>,
                    range: std::ops::Range<usize>,
                ) -> Self {
                    Self {
                        source,
                        source_path: None,
                        tokens,
                        cursor: range.start,
                        range_start: range.start,
                        range_end: range.end,
                        last_cursor: range.start,
                    }
                }

                pub fn source(&self) -> &str {
                    &self.source
                }

                pub fn source_path(&self) -> Option<&Path> {
                    self.source_path.as_deref()
                }

                pub fn slice(&self, span: &Span) -> &str {
                    use synkit::SpanLike;
                    &self.source[span.start()..span.end()]
                }

                pub fn all(&self) -> &[SpannedToken] {
                    &self.tokens[self.range_start..self.range_end]
                }

                fn is_skip_token(tok: &SpannedToken) -> bool {
                    #skip_match
                }

                /// Parse a value from the stream and wrap it with span information.
                /// This is the primary parsing method users should use.
                pub fn parse<T: super::traits::Parse>(&mut self) -> Result<Spanned<T>, super::#error_type> {
                    T::parse_spanned(self)
                }

                /// Peek without consuming to check if the next token matches type T.
                pub fn peek<T: super::traits::Peek>(&self) -> bool {
                    T::peek(self)
                }

                /// Check if the stream has reached EOF (no more non-skip tokens).
                pub fn is_empty(&self) -> bool {
                    use synkit::TokenStream as _;
                    self.peek_token().is_none()
                }

                /// Get the span of the current cursor position.
                pub fn current_span(&self) -> &Span {
                    self.tokens.get(self.cursor)
                        .map(|t| &t.span)
                        .unwrap_or(&Span::CallSite)
                }

                /// Extract tokens between matching delimiters (e.g., brackets, braces, parens).
                ///
                /// Returns a new TokenStream containing only the inner tokens (excluding delimiters)
                /// and the span covering the entire delimited region.
                ///
                /// # Type Parameters
                /// * `Open` - The opening delimiter token type (must impl Parse + Peek)
                /// * `Close` - The closing delimiter token type (must impl Parse + Peek)
                ///
                /// # Example
                /// ```ignore
                /// // For input: [1, 2, 3]
                /// let (inner, span) = stream.extract_inner::<LBracketToken, RBracketToken>()?;
                /// // inner now contains tokens for: 1, 2, 3
                /// ```
                pub fn extract_inner<
                    Open: super::traits::Parse + super::traits::Peek + super::traits::Diagnostic,
                    Close: super::traits::Parse + super::traits::Peek + super::traits::Diagnostic,
                >(&mut self) -> Result<(TokenStream, Span), super::#error_type> {
                    use synkit::TokenStream as _;
                    use synkit::SpanLike;

                    // Consume and validate opening delimiter
                    let first_span = match self.next() {
                        Some(tok) if Open::is(&tok.value) => tok.span.clone(),
                        Some(tok) => {
                            return Err(super::#error_type::Expected {
                                expect: Open::fmt(),
                                found: format!("{}", tok.value),
                            });
                        }
                        None => {
                            return Err(super::#error_type::Empty {
                                expect: Open::fmt(),
                            });
                        }
                    };

                    let open_index = self.cursor - 1;
                    let mut depth = 1usize;
                    let mut end_pos = None;

                    // Find matching close delimiter, tracking nesting
                    while let Some(tok) = self.next_raw() {
                        if Open::is(&tok.value) {
                            depth += 1;
                        } else if Close::is(&tok.value) {
                            depth -= 1;
                            if depth == 0 {
                                end_pos = Some(self.cursor);
                                break;
                            }
                        }
                    }

                    if let Some(end) = end_pos {
                        let close_index = end - 1;
                        let inner_start = open_index + 1;
                        let inner_end = close_index;

                        let close_span = self.tokens.get(close_index)
                            .map(|t| &t.span)
                            .unwrap_or(&Span::CallSite);

                        let combined_span = Span::new(first_span.start(), close_span.end());

                        Ok((
                            TokenStream {
                                source: Arc::clone(&self.source),
                                source_path: self.source_path.as_ref().map(Arc::clone),
                                tokens: Arc::clone(&self.tokens),
                                cursor: inner_start,
                                range_start: inner_start,
                                range_end: inner_end,
                                last_cursor: inner_start,
                            },
                            combined_span,
                        ))
                    } else {
                        Err(super::#error_type::Empty {
                            expect: Close::fmt(),
                        })
                    }
                }
            }

            impl synkit::TokenStream for TokenStream {
                type Token = Token;
                type Span = Span;
                type Spanned<T: Clone> = Spanned<T>;

                fn peek_token_raw(&self) -> Option<&SpannedToken> {
                    self.tokens
                        .get(self.cursor)
                        .filter(|_| self.cursor < self.range_end)
                }

                fn next_raw(&mut self) -> Option<SpannedToken> {
                    if self.cursor >= self.range_end {
                        return None;
                    }
                    let tok = self.tokens.get(self.cursor).cloned();
                    if tok.is_some() {
                        self.last_cursor = self.cursor;
                        self.cursor += 1;
                    }
                    tok
                }

                fn next(&mut self) -> Option<SpannedToken> {
                    loop {
                        let tok = self.next_raw()?;
                        if !Self::is_skip_token(&tok) {
                            return Some(tok);
                        }
                    }
                }

                fn peek_token(&self) -> Option<&SpannedToken> {
                    let mut cursor = self.cursor;
                    while cursor < self.range_end {
                        if let Some(tok) = self.tokens.get(cursor) {
                            if !Self::is_skip_token(tok) {
                                return Some(tok);
                            }
                            cursor += 1;
                        } else {
                            break;
                        }
                    }
                    None
                }

                fn cursor(&self) -> usize {
                    self.cursor
                }

                fn rewind(&mut self, pos: usize) {
                    self.cursor = pos.clamp(self.range_start, self.range_end);
                }

                fn fork(&self) -> Self {
                    Self {
                        source: Arc::clone(&self.source),
                        source_path: self.source_path.as_ref().map(Arc::clone),
                        tokens: Arc::clone(&self.tokens),
                        cursor: self.cursor,
                        range_start: self.range_start,
                        range_end: self.range_end,
                        last_cursor: self.last_cursor,
                    }
                }

                fn cursor_span(&self) -> Option<Span> {
                    self.tokens.get(self.cursor).map(|t| t.span.clone())
                }

                fn last_span(&self) -> Option<Span> {
                    self.tokens.get(self.last_cursor).map(|t| t.span.clone())
                }

                fn span_at(&self, pos: usize) -> Option<Span> {
                    self.tokens.get(pos).map(|t| t.span.clone())
                }
            }

            // Compile-time assertions for TokenStream
            const _: () = {
                const fn assert_send<T: Send>() {}
                const fn assert_sync<T: Sync>() {}
                assert_send::<TokenStream>();
                assert_sync::<TokenStream>();
            };

            #[cfg(target_pointer_width = "64")]
            const _: () = {
                use core::mem::{size_of, align_of};

                // TokenStream layout on 64-bit:
                // - source: Arc<str> = 16 bytes (DST: ptr + len)
                // - source_path: Option<Arc<Path>> = 16 bytes (DST: ptr + len)
                // - tokens: Arc<Vec<SpannedToken>> = 8 bytes (thin ptr)
                // - cursor: usize = 8 bytes
                // - range_start: usize = 8 bytes
                // - range_end: usize = 8 bytes
                // - last_cursor: usize = 8 bytes
                // Total: 72 bytes, 8-byte aligned
                const _STREAM_SIZE: () = assert!(size_of::<TokenStream>() == 72);
                const _STREAM_ALIGN: () = assert!(align_of::<TokenStream>() == 8);
            };

            #[derive(Default, Debug, Clone)]
            pub struct MutTokenStream {
                tokens: Vec<SpannedToken>,
            }

            impl MutTokenStream {
                pub fn new() -> Self {
                    Self::default()
                }

                pub fn push(&mut self, token: SpannedToken) {
                    self.tokens.push(token);
                }

                pub fn extend<I: IntoIterator<Item = SpannedToken>>(&mut self, iter: I) {
                    self.tokens.extend(iter);
                }

                pub fn all_tokens(&self) -> &[SpannedToken] {
                    &self.tokens
                }

                pub fn into_vec(self) -> Vec<SpannedToken> {
                    self.tokens
                }
            }
        }
    };

    let printer_module = quote! {
        pub mod printer {
            use super::tokens::Token;

            pub struct Printer {
                pub buf: String,
                pub indent_level: usize,
                indent_width: usize,
                use_tabs: bool,
            }

            impl Default for Printer {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl Printer {
                pub fn new() -> Self {
                    Self {
                        buf: String::with_capacity(1024),
                        indent_level: 0,
                        indent_width: 4,
                        use_tabs: false,
                    }
                }

                pub fn with_capacity(cap: usize) -> Self {
                    Self {
                        buf: String::with_capacity(cap),
                        ..Self::default()
                    }
                }

                pub fn with_indent_width(mut self, width: usize) -> Self {
                    self.indent_width = width;
                    self
                }

                pub fn with_tabs(mut self) -> Self {
                    self.use_tabs = true;
                    self
                }
            }

            impl synkit::Printer for Printer {
                type Token = Token;

                fn buf(&self) -> &str {
                    &self.buf
                }

                fn buf_mut(&mut self) -> &mut String {
                    &mut self.buf
                }

                fn indent_level(&self) -> usize {
                    self.indent_level
                }

                fn set_indent(&mut self, level: usize) {
                    self.indent_level = level;
                }

                fn into_string(self) -> String {
                    self.buf
                }

                fn indent_width(&self) -> usize {
                    self.indent_width
                }

                fn use_tabs(&self) -> bool {
                    self.use_tabs
                }

                fn token(&mut self, t: &Token) {
                    use std::fmt::Write;
                    let _ = write!(self.buf, "{}", t);
                }
            }
        }
    };

    // Generate delimiter structs (inside delimiters module)
    let delimiter_structs: Vec<_> = delimiters
        .iter()
        .map(|d| {
            let DelimiterDef { name, open, close } = d;

            quote! {
                #[derive(Debug, Clone)]
                pub struct #name {
                    span: super::span::Span,
                }

                impl #name {
                    pub fn new(span: super::span::Span) -> Self {
                        Self { span }
                    }

                    pub fn call_site() -> Self {
                        Self {
                            span: super::span::Span::CallSite,
                        }
                    }

                    pub fn span(&self) -> &super::span::Span {
                        &self.span
                    }

                    pub fn write_with<F>(&self, printer: &mut super::printer::Printer, inner: F)
                    where
                        F: FnOnce(&mut super::printer::Printer),
                    {
                        use synkit::Printer as _;
                        printer.token(&super::tokens::Token::#open);
                        inner(printer);
                        printer.token(&super::tokens::Token::#close);
                    }
                }
            }
        })
        .collect();

    // Generate delimiter macros at crate level (not inside module, for proper re-export)
    let delimiter_macros: Vec<_> = delimiters
        .iter()
        .map(|d| {
            let DelimiterDef { name, open, close } = d;
            let open_token = format_ident!("{}Token", open);
            let close_token = format_ident!("{}Token", close);
            let macro_name = format_ident!("{}", name.to_string().to_lowercase());

            quote! {
                /// Extract tokens within matching delimiters.
                ///
                /// # Usage
                /// ```ignore
                /// let delim = #macro_name!(inner in stream);
                /// // `inner` is now a TokenStream of the contents
                /// // `delim` holds the span information
                /// ```
                #[allow(non_snake_case)]
                #[macro_export]
                macro_rules! #macro_name {
                    ($tokens:ident in $input:ident) => {
                        match $input.extract_inner::<
                            $crate::tokens::#open_token,
                            $crate::tokens::#close_token
                        >() {
                            Ok((tokens, span)) => {
                                $tokens = tokens;
                                $crate::delimiters::#name::new(span)
                            }
                            Err(e) => return Err(e),
                        }
                    };
                    ($tokens:ident in $input:ident; $err:expr) => {
                        match $input.extract_inner::<
                            $crate::tokens::#open_token,
                            $crate::tokens::#close_token
                        >() {
                            Ok((tokens, span)) => {
                                $tokens = tokens;
                                $crate::delimiters::#name::new(span)
                            }
                            Err(..) => return $err,
                        }
                    };
                }
            }
        })
        .collect();

    let delimiters_module = quote! {
        pub mod delimiters {
            #(#delimiter_structs)*
        }
    };

    // Generate Diagnostic, Peek, and Parse impls for token structs
    let token_trait_impls: Vec<_> = tokens
        .iter()
        .map(|t| {
            let name = &t.name;
            let struct_name = format_ident!("{}Token", t.name);
            let has_inner = t.inner_type.is_some();

            let parse_impl = if has_inner {
                quote! {
                    impl Parse for super::tokens::#struct_name {
                        fn parse(stream: &mut TokenStream) -> Result<Self, super::#error_type> {
                            use synkit::TokenStream as _;
                            match stream.next() {
                                Some(tok) => match tok.value {
                                    super::tokens::Token::#name(v) => Ok(super::tokens::#struct_name::new(v)),
                                    ref other => Err(super::#error_type::Expected {
                                        expect: super::tokens::#struct_name::fmt(),
                                        found: format!("{}", other),
                                    }),
                                },
                                None => Err(super::#error_type::Empty {
                                    expect: super::tokens::#struct_name::fmt(),
                                }),
                            }
                        }
                    }
                }
            } else {
                quote! {
                    impl Parse for super::tokens::#struct_name {
                        fn parse(stream: &mut TokenStream) -> Result<Self, super::#error_type> {
                            use synkit::TokenStream as _;
                            match stream.next() {
                                Some(tok) => match &tok.value {
                                    super::tokens::Token::#name => Ok(super::tokens::#struct_name::new()),
                                    other => Err(super::#error_type::Expected {
                                        expect: super::tokens::#struct_name::fmt(),
                                        found: format!("{}", other),
                                    }),
                                },
                                None => Err(super::#error_type::Empty {
                                    expect: super::tokens::#struct_name::fmt(),
                                }),
                            }
                        }
                    }
                }
            };

            quote! {
                impl Diagnostic for super::tokens::#struct_name {
                    fn fmt() -> &'static str {
                        super::tokens::#struct_name::fmt()
                    }
                }

                impl Peek for super::tokens::#struct_name {
                    fn is(token: &Token) -> bool {
                        <super::tokens::#struct_name as synkit::Peek>::is(token)
                    }
                }
                #parse_impl
            }
        })
        .collect();

    #[cfg(any(feature = "tokio", feature = "futures"))]
    let async_traits = quote! {

            /// Simplified IncrementalParse trait for streaming/chunked parsing.
            ///
            /// Implement this trait to enable incremental parsing of AST nodes
            /// from a token buffer with checkpoint-based state management.
            ///
            /// ```ignore
            /// impl IncrementalParse for MyNode {
            ///     fn parse_incremental(
            ///         tokens: &[Token],
            ///         checkpoint: &synkit::async_stream::ParseCheckpoint,
            ///     ) -> Result<(Option<Self>, synkit::async_stream::ParseCheckpoint), LexError> {
            ///         // ...
            ///     }
            ///
            ///     fn can_parse(tokens: &[Token], checkpoint: &synkit::async_stream::ParseCheckpoint) -> bool {
            ///         checkpoint.cursor < tokens.len()
            ///     }
            /// }
            /// ```
            pub trait IncrementalParse: Sized {
                /// Attempt to parse from the given tokens starting at the checkpoint.
                ///
                /// Returns:
                /// - `Ok((Some(node), new_checkpoint))` if a complete node was parsed
                /// - `Ok((None, checkpoint))` if more tokens are needed
                /// - `Err(error)` if an unrecoverable error occurred
                fn parse_incremental(
                    tokens: &[Token],
                    checkpoint: &synkit::async_stream::ParseCheckpoint,
                ) -> Result<(Option<Self>, synkit::async_stream::ParseCheckpoint), super::#error_type>;

                /// Check if parsing can produce a result with the current tokens.
                ///
                /// This is used for early return when more input is clearly needed.
                fn can_parse(tokens: &[Token], checkpoint: &synkit::async_stream::ParseCheckpoint) -> bool;
            }
    };
    #[cfg(not(any(feature = "tokio", feature = "futures")))]
    let async_traits = quote! {};

    // Generate user-friendly local trait aliases
    let traits_module = quote! {
        /// User-friendly traits using concrete types.
        ///
        /// These traits use concrete types (TokenStream, Token, Error) so users don't need
        /// to specify associated types when implementing them.
        pub mod traits {
            use super::span::{Span, Spanned};
            use super::tokens::Token;
            use super::stream::TokenStream;
            use super::printer::Printer;

            /// Simplified Parse trait using concrete types.
            ///
            /// Implement this trait for your AST nodes:
            /// ```ignore
            /// impl Parse for MyNode {
            ///     fn parse(stream: &mut TokenStream) -> Result<Self, LexError> {
            ///         // ...
            ///     }
            /// }
            /// ```
            pub trait Parse: Sized {
                fn parse(stream: &mut TokenStream) -> Result<Self, super::#error_type>;

                /// Parse and wrap the result with span information.
                ///
                /// The span starts from the first non-skip token (not from whitespace).
                fn parse_spanned(stream: &mut TokenStream) -> Result<Spanned<Self>, super::#error_type> {
                    use synkit::TokenStream as _;
                    // Get span of first non-skip token (peek_token skips whitespace)
                    let start = stream.peek_token()
                        .map(|t| synkit::SpanLike::start(&t.span))
                        .unwrap_or(0);

                    let value = Self::parse(stream)?;

                    let end = stream.last_span()
                        .map(|s| synkit::SpanLike::end(&s))
                        .unwrap_or(start);

                    Ok(Spanned::new(start, end, value))
                }
            }

            /// Simplified Peek trait using concrete Token type.
            ///
            /// Implement this trait to enable lookahead for your AST nodes:
            /// ```ignore
            /// impl Peek for MyNode {
            ///     fn is(token: &Token) -> bool {
            ///         matches!(token, Token::MyKeyword)
            ///     }
            /// }
            /// ```
            pub trait Peek: Sized {
                /// Check if a token matches this type.
                fn is(token: &Token) -> bool;

                /// Peek at stream without consuming (default impl uses `is()`).
                fn peek(stream: &TokenStream) -> bool {
                    use synkit::TokenStream as _;
                    stream
                        .peek_token()
                        .map(|t| Self::is(&t.value))
                        .unwrap_or(false)
                }
            }

            /// Simplified ToTokens trait using concrete Printer type.
            ///
            /// Implement this trait for round-trip formatting:
            /// ```ignore
            /// impl ToTokens for MyNode {
            ///     fn write(&self, printer: &mut Printer) {
            ///         printer.token(&self.keyword.token());
            ///         // ...
            ///     }
            /// }
            /// ```
            pub trait ToTokens {
                fn write(&self, printer: &mut Printer);

                fn to_string_formatted(&self) -> String {
                    let mut printer = Printer::new();
                    self.write(&mut printer);
                    synkit::Printer::into_string(printer)
                }
            }

            /// Simplified Diagnostic trait for error messages.
            pub trait Diagnostic {
                fn fmt() -> &'static str;
            }


            // Blanket impls for Option, Box, etc. using local traits
            impl<T: Parse + Peek> Parse for Option<T> {
                fn parse(stream: &mut TokenStream) -> Result<Self, super::#error_type> {
                    if T::peek(stream) {
                        Ok(Some(T::parse(stream)?))
                    } else {
                        Ok(None)
                    }
                }
            }

            impl<T: Parse> Parse for Box<T> {
                fn parse(stream: &mut TokenStream) -> Result<Self, super::#error_type> {
                    Ok(Box::new(T::parse(stream)?))
                }
            }

            impl<T: Peek> Peek for Box<T> {
                fn is(token: &Token) -> bool {
                    T::is(token)
                }
            }

            impl<T: ToTokens> ToTokens for Option<T> {
                fn write(&self, p: &mut Printer) {
                    if let Some(v) = self {
                        v.write(p);
                    }
                }
            }

            impl<T: ToTokens> ToTokens for Box<T> {
                fn write(&self, p: &mut Printer) {
                    self.as_ref().write(p);
                }
            }

            impl<T: ToTokens> ToTokens for Vec<T> {
                fn write(&self, p: &mut Printer) {
                    for item in self {
                        item.write(p);
                    }
                }
            }

            impl<T: ToTokens> ToTokens for &T {
                fn write(&self, p: &mut Printer) {
                    (*self).write(p);
                }
            }

            #async_traits

            // Implement local traits for generated token structs
            #(#token_trait_impls)*
        }
    };

    #[cfg(any(feature = "tokio", feature = "futures"))]
    let async_exports = quote! {
        pub use traits::IncrementalParse;
    };

    #[cfg(not(any(feature = "tokio", feature = "futures")))]
    let async_exports = quote! {};

    let reexports = quote! {
        pub use span::{Span, RawSpan, Spanned};
        pub use tokens::{Token, SpannedToken};
        pub use stream::{TokenStream, MutTokenStream};
        pub use printer::Printer;
        pub use traits::{Parse, Peek, ToTokens, Diagnostic};

        #async_exports
    };

    let delimiter_reexports: Vec<_> = delimiters.iter().map(|d| &d.name).collect();
    let delimiter_reexport = if delimiter_reexports.is_empty() {
        quote! {}
    } else {
        quote! { pub use delimiters::{#(#delimiter_reexports),*}; }
    };

    let output = quote! {
        #[allow(unused)]
        #span_module
        #[allow(unused)]
        #tokens_module
        #[allow(unused)]
        #stream_module
        #[allow(unused)]
        #printer_module
        #[allow(unused)]
        #delimiters_module
        #[allow(unused)]
        #traits_module

        #[allow(unused)]
        pub mod prelude {
            use super::*;
            #reexports
        }
        pub use prelude::*;

        #delimiter_reexport

        // Delimiter extraction macros
        #(#delimiter_macros)*
    };

    Ok(output)
}
