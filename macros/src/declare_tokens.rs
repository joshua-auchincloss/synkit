use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Ident, LitStr, Path, Token, Type, braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

/// Convert PascalCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

pub struct DeclareTokensInput {
    pub span_mod: Option<Path>,
    pub error_type: Ident,
    pub derives: Vec<Path>,
    pub struct_derives: Vec<Path>,
    pub logos_attrs: Vec<Attribute>,
    pub tokens: Vec<TokenDef>,
}

pub struct TokenDef {
    pub attrs: Vec<Attribute>,
    pub fmt_str: Option<LitStr>,
    pub extra_derives: Vec<Path>,
    pub no_to_tokens: bool,
    pub name: Ident,
    pub inner_type: Option<Type>,
}

impl Clone for TokenDef {
    fn clone(&self) -> Self {
        Self {
            attrs: self.attrs.clone(),
            fmt_str: self.fmt_str.clone(),
            extra_derives: self.extra_derives.clone(),
            no_to_tokens: self.no_to_tokens,
            name: self.name.clone(),
            inner_type: self.inner_type.clone(),
        }
    }
}

impl Parse for DeclareTokensInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut span_mod = None;
        let mut error_type = None;
        let mut derives = Vec::new();
        let mut struct_derives = Vec::new();
        let mut logos_attrs = Vec::new();
        let mut tokens = Vec::new();

        while !input.is_empty() {
            if input.peek(Token![#]) {
                let attr = input.call(Attribute::parse_outer)?;
                logos_attrs.extend(attr);
                continue;
            }

            let ident: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            match ident.to_string().as_str() {
                "span_mod" => {
                    span_mod = Some(input.parse()?);
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                }
                "error" => {
                    error_type = Some(input.parse()?);
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                }
                "derives" => {
                    let content;
                    bracketed!(content in input);
                    derives = Punctuated::<Path, Token![,]>::parse_terminated(&content)?
                        .into_iter()
                        .collect();
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                }
                "struct_derives" => {
                    let content;
                    bracketed!(content in input);
                    struct_derives = Punctuated::<Path, Token![,]>::parse_terminated(&content)?
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
            span_mod,
            error_type,
            derives,
            struct_derives,
            logos_attrs,
            tokens,
        })
    }
}

impl Parse for TokenDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = Vec::new();
        let mut fmt_str = None;
        let mut extra_derives = Vec::new();
        let mut no_to_tokens = false;

        while input.peek(Token![#]) {
            let attr_list = input.call(Attribute::parse_outer)?;
            for attr in attr_list {
                if attr.path().is_ident("fmt") {
                    fmt_str = Some(attr.parse_args()?);
                } else if attr.path().is_ident("derive") {
                    attr.parse_nested_meta(|meta| {
                        extra_derives.push(meta.path);
                        Ok(())
                    })?;
                } else if attr.path().is_ident("no_to_tokens") {
                    no_to_tokens = true;
                } else {
                    attrs.push(attr);
                }
            }
        }

        let name: Ident = input.parse()?;

        let inner_type = if input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            Some(content.parse()?)
        } else {
            None
        };

        Ok(Self {
            attrs,
            fmt_str,
            extra_derives,
            no_to_tokens,
            name,
            inner_type,
        })
    }
}

pub fn expand(input: DeclareTokensInput) -> syn::Result<TokenStream> {
    let DeclareTokensInput {
        span_mod,
        error_type,
        derives,
        struct_derives,
        logos_attrs,
        tokens,
    } = input;

    let span_import = if let Some(ref path) = span_mod {
        quote! { use #path::{Span, Spanned}; }
    } else {
        quote! { use super::span::{Span, Spanned}; }
    };

    let error_ref = quote! { super::#error_type };

    let derives_tokens = if derives.is_empty() {
        quote! { Clone, PartialEq, Debug }
    } else {
        quote! { #(#derives),* }
    };

    let struct_derives_tokens = if struct_derives.is_empty() {
        quote! { Clone, PartialEq, Debug }
    } else {
        quote! { #(#struct_derives),* }
    };

    let token_variants: Vec<_> = tokens
        .iter()
        .map(|t| {
            let TokenDef {
                attrs,
                name,
                inner_type,
                ..
            } = t;
            if let Some(ty) = inner_type {
                quote! {
                    #(#attrs)*
                    #name(#ty)
                }
            } else {
                quote! {
                    #(#attrs)*
                    #name
                }
            }
        })
        .collect();

    let display_arms: Vec<_> = tokens
        .iter()
        .map(|t| {
            let name = &t.name;
            let fmt = t.fmt_str.as_ref().map(|s| s.value());
            if t.inner_type.is_some() {
                quote! {
                    Token::#name(v) => write!(f, "{}", v)
                }
            } else if let Some(ref fmt_val) = fmt {
                let escaped = fmt_val.replace('{', "{{").replace('}', "}}");
                quote! {
                    Token::#name => write!(f, #escaped)
                }
            } else {
                let attrs = &t.attrs;
                let literal = attrs.iter().find_map(|a| {
                    if a.path().is_ident("token") {
                        a.parse_args::<LitStr>().ok()
                    } else {
                        None
                    }
                });
                if let Some(lit) = literal {
                    let s = lit.value().replace('{', "{{").replace('}', "}}");
                    quote! {
                        Token::#name => write!(f, #s)
                    }
                } else {
                    let name_str = name.to_string();
                    quote! {
                        Token::#name => write!(f, "<{}>", #name_str)
                    }
                }
            }
        })
        .collect();

    let token_structs: Vec<_> = tokens
        .iter()
        .map(|t| {
            let TokenDef {
                name,
                inner_type,
                fmt_str,
                extra_derives,
                attrs,
                no_to_tokens,
                ..
            } = t;
            let struct_name = format_ident!("{}Token", name);

            let all_derives = if extra_derives.is_empty() {
                struct_derives_tokens.clone()
            } else {
                quote! { #struct_derives_tokens, #(#extra_derives),* }
            };

            let fmt_impl = if let Some(lit) = fmt_str {
                let s = lit.value();
                quote! { #s }
            } else {
                let literal = attrs.iter().find_map(|a| {
                    if a.path().is_ident("token") {
                        a.parse_args::<LitStr>().ok()
                    } else {
                        None
                    }
                });
                if let Some(lit) = literal {
                    let s = lit.value();
                    quote! { #s }
                } else {
                    let name_str = name.to_string().to_lowercase();
                    quote! { #name_str }
                }
            };

            // Generate ToTokens impl unless #[no_to_tokens] is specified
            // no_to_tokens means the user will implement themselves due to special requirements / logic
            // e.g. quoting etc
            let to_tokens_impl = if *no_to_tokens {
                quote! {}
            } else {
                quote! {
                    impl super::traits::ToTokens for #struct_name {
                        fn write(&self, p: &mut super::printer::Printer) {
                            use synkit::Printer as _;
                            p.token(&self.token());
                        }
                    }
                }
            };

            if let Some(ty) = inner_type {
                quote! {
                    #[derive(#all_derives)]
                    pub struct #struct_name(pub #ty);

                    impl #struct_name {
                        pub fn new(value: impl Into<#ty>) -> Self {
                            Self(value.into())
                        }

                        pub fn token(&self) -> Token {
                            Token::#name(self.0.clone())
                        }

                        pub fn fmt() -> &'static str {
                            #fmt_impl
                        }

                        pub fn into_inner(self) -> #ty {
                            self.0
                        }
                    }

                    impl Default for #struct_name {
                        fn default() -> Self {
                            Self(Default::default())
                        }
                    }

                    impl std::ops::Deref for #struct_name {
                        type Target = #ty;
                        fn deref(&self) -> &Self::Target {
                            &self.0
                        }
                    }

                    impl synkit::Diagnostic for #struct_name {
                        fn fmt() -> &'static str {
                            #fmt_impl
                        }
                    }

                    impl synkit::Peek for #struct_name {
                        type Token = Token;
                        fn is(token: &Token) -> bool {
                            matches!(token, Token::#name(_))
                        }
                    }

                    #to_tokens_impl
                }
            } else {
                quote! {
                    #[derive(#all_derives)]
                    pub struct #struct_name;

                    impl #struct_name {
                        pub fn new() -> Self {
                            Self
                        }

                        pub fn token(&self) -> Token {
                            Token::#name
                        }

                        pub fn fmt() -> &'static str {
                            #fmt_impl
                        }
                    }

                    impl Default for #struct_name {
                        fn default() -> Self {
                            Self::new()
                        }
                    }

                    impl synkit::Diagnostic for #struct_name {
                        fn fmt() -> &'static str {
                            #fmt_impl
                        }
                    }

                    impl synkit::Peek for #struct_name {
                        type Token = Token;
                        fn is(token: &Token) -> bool {
                            matches!(token, Token::#name)
                        }
                    }

                    #to_tokens_impl
                }
            }
        })
        .collect();

    let token_macro_arms: Vec<_> = tokens
        .iter()
        .filter_map(|t| {
            let name = &t.name;
            let struct_name = format_ident!("{}Token", name);

            // Find #[token("...")] attribute
            let literal = t.attrs.iter().find_map(|a| {
                if a.path().is_ident("token") {
                    a.parse_args::<LitStr>().ok()
                } else {
                    None
                }
            });

            if let Some(lit) = literal {
                let s = lit.value();
                // Try to parse the token string as token trees for the macro pattern
                // This handles punctuation like "=", "->", "::", etc.
                if let Ok(token_trees) = s.parse::<proc_macro2::TokenStream>() {
                    Some(quote! {
                        [#token_trees] => { $crate::tokens::#struct_name }
                    })
                } else if s.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    // Keywords like "struct", "enum", etc.
                    let ident = format_ident!("{}", s);
                    Some(quote! {
                        [#ident] => { $crate::tokens::#struct_name }
                    })
                } else {
                    // Can't create a macro arm for this token
                    None
                }
            } else {
                // No #[token] attr - use snake_case of variant name (for regex tokens)
                let name_snake = to_snake_case(&name.to_string());
                let name_ident = format_ident!("{}", name_snake);
                Some(quote! {
                    [#name_ident] => { $crate::tokens::#struct_name }
                })
            }
        })
        .collect();

    // Generate Token! macro as a local macro (not #[macro_export])
    // Users can bring it into scope with `use crate::tokens::Token;`
    // This avoids the issue with macro-expanded macro_export macros
    let token_macro = if token_macro_arms.is_empty() {
        quote! {}
    } else {
        quote! {
            /// Match token literals to their token struct types.
            ///
            /// # Example
            /// ```ignore
            /// use crate::tokens::Tok;
            /// let _: Tok![=] = stream.parse()?;
            /// let _: Tok![struct] = stream.parse()?;
            /// ```
            #[allow(non_snake_case)]
            macro_rules! Tok {
                #(#token_macro_arms);*
            }
            pub(crate) use Tok;

            /// Match token literals to spanned token types.
            ///
            /// # Example
            /// ```ignore
            /// use crate::tokens::SpannedTok;
            /// let tok: SpannedTok![=] = stream.parse()?;
            /// ```
            #[allow(non_snake_case)]
            macro_rules! SpannedTok {
                ($tt:tt) => { $crate::span::Spanned<$crate::tokens::Tok![$tt]> };
            }
            pub(crate) use SpannedTok;
        }
    };

    // Generate ToTokens arms for the Token enum
    // For tokens with no_to_tokens, we skip output (they handle their own serialization)
    let token_to_tokens_arms: Vec<_> = tokens
        .iter()
        .map(|t| {
            let name = &t.name;
            let struct_name = format_ident!("{}Token", name);
            if t.no_to_tokens {
                // Token marked with #[no_to_tokens] - user handles this case
                if t.inner_type.is_some() {
                    quote! {
                        Token::#name(_) => {}
                    }
                } else {
                    quote! {
                        Token::#name => {}
                    }
                }
            } else if t.inner_type.is_some() {
                quote! {
                    Token::#name(v) => #struct_name::new(v.clone()).write(p)
                }
            } else {
                quote! {
                    Token::#name => #struct_name::new().write(p)
                }
            }
        })
        .collect();

    let output = quote! {
        #span_import

        #[derive(logos::Logos, #derives_tokens)]
        #(#logos_attrs)*
        #[logos(error = #error_ref)]
        pub enum Token {
            #(#token_variants),*
        }

        impl std::fmt::Display for Token {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#display_arms),*
                }
            }
        }

        impl super::traits::ToTokens for Token {
            fn write(&self, p: &mut super::printer::Printer) {
                match self {
                    #(#token_to_tokens_arms),*
                }
            }
        }

        #(#token_structs)*

        pub type SpannedToken = Spanned<Token>;

        #token_macro
    };

    Ok(output)
}
