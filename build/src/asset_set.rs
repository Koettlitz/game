use std::ops::Deref;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Ident, LitStr, Token, braced,
    parse::{Parse, ParseStream},
};

use bevy_elf::ResolveStrategy;

use crate::CratePath;

pub struct AssetSetArgs {
    pub base_path: LitStr,
    pub resolve_strategy: ParsedResolveStrategy,
    pub progress_name: Option<LitStr>,
}

impl Parse for AssetSetArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut base_path: Option<LitStr> = None;
        let mut progress_name: Option<LitStr> = None;
        let mut resolve_strategy: Option<ParsedResolveStrategy> = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "progress_name" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    progress_name = Some(lit);
                }

                "base_path" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    base_path = Some(lit);
                }
                "resolve_strategy" => {
                    input.parse::<Token![=]>()?;
                    let strategy: ParsedResolveStrategy = input.parse()?;
                    resolve_strategy = Some(strategy);
                }
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "Unknown parameter. Expected `progress_name`, `base_path` or `resolve_strategy`.",
                    ));
                }
            }
            // optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let base_path =
            base_path.ok_or_else(|| syn::Error::new(input.span(), "`base_path` is required"))?;

        Ok(AssetSetArgs {
            progress_name,
            resolve_strategy: resolve_strategy.unwrap_or_default(),
            base_path,
        })
    }
}

/// A parsed `ResolveStrategy` together with the exact tokens it was parsed from.
#[derive(Clone, Default)]
pub struct ParsedResolveStrategy {
    pub _value: ResolveStrategy,
    pub tokens: Option<TokenStream>,
}

impl Parse for ParsedResolveStrategy {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let begin = input.cursor();
        let value = *input.parse::<ParseResolveStrategy>()?;
        let end = input.cursor();

        let mut tokens = TokenStream::new();
        let mut cursor = begin;
        while cursor != end {
            let Some((tt, next)) = cursor.token_tree() else {
                break;
            };
            tokens.extend(std::iter::once(tt));
            cursor = next;
        }

        Ok(Self {
            _value: value,
            tokens: Some(tokens),
        })
    }
}

impl ToTokens for ParsedResolveStrategy {
    fn to_tokens(&self, out: &mut TokenStream) {
        if let Some(ref tokens) = self.tokens {
            out.extend(tokens.clone());
        } else {
            match CratePath::try_from("bevy_elf") {
                Ok(module_path) => {
                    out.extend(quote!(#module_path::ResolveStrategy::FileNameIsId));
                }
                Err(e) => {
                    syn::Error::new(
                        Span::call_site(),
                        format!("Could not resolve crate `bevy_elf`: {e}"),
                    )
                    .to_compile_error()
                    .to_tokens(out);
                }
            }
        }
    }
}

struct ParseResolveStrategy(ResolveStrategy);

impl Deref for ParseResolveStrategy {
    type Target = ResolveStrategy;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Parse for ParseResolveStrategy {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Accepts `Variant` as well as `ResolveStrategy::Variant` (or any longer path).
        let path: syn::Path = input.parse()?;
        let last = path
            .segments
            .last()
            .ok_or_else(|| syn::Error::new_spanned(&path, "empty path"))?;

        match last.ident.to_string().as_str() {
            "FileNameIsId" => Ok(Self(ResolveStrategy::FileNameIsId)),
            "FileStemIsId" => Ok(Self(ResolveStrategy::FileStemIsId {
                extension: parse_str_field(input, "extension")?,
            })),
            "DirectoryNameIsId" => Ok(Self(ResolveStrategy::DirectoryNameIsId {
                file_name: parse_str_field(input, "file_name")?,
            })),
            _ => Err(syn::Error::new_spanned(
                &last.ident,
                "unknown variant, expected `FileNameIsId`, `FileStemIsId` or `DirectoryNameIsId`",
            )),
        }
    }
}

/// Parses `{ <name>: "literal" }` (trailing comma optional) into a `&'static str`.
fn parse_str_field(input: ParseStream, name: &str) -> syn::Result<&'static str> {
    let content;
    braced!(content in input);

    let key: Ident = content.parse()?;
    if key != name {
        return Err(syn::Error::new_spanned(
            &key,
            format!("expected field `{name}`"),
        ));
    }
    content.parse::<Token![:]>()?;
    let lit: LitStr = content.parse()?;

    if content.peek(Token![,]) {
        content.parse::<Token![,]>()?;
    }
    if !content.is_empty() {
        return Err(content.error("unexpected tokens after field"));
    }

    // The enum requires `&'static str`, so the string has to be leaked.
    // That is fine inside a proc macro, since the process is short-lived.
    Ok(Box::leak(lit.value().into_boxed_str()))
}
