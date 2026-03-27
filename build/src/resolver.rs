use std::borrow::Cow;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Ident, LitStr, Token, parse::Parse};

use crate::resolve_crate_name;

pub struct ResolverArgs<'a> {
    pub base_path: Cow<'a, LitStr>,
    pub extension: Cow<'a, LitStr>,
    pub asset_type: Cow<'a, syn::Path>,
}

impl<'a> Parse for ResolverArgs<'a> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut base_path: Option<LitStr> = None;
        let mut extension: Option<LitStr> = None;
        let mut asset_type: Option<syn::Path> = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "base_path" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    base_path = Some(lit);
                }
                "extension" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    extension = Some(lit);
                }
                "asset_type" => {
                    let content;
                    syn::parenthesized!(content in input);
                    let ty: syn::Path = content.parse()?;
                    asset_type = Some(ty);
                }
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "Unknown parameter. Expected `base_path`, `extension`, or `asset_type`.",
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
        let asset_type =
            asset_type.ok_or_else(|| syn::Error::new(input.span(), "`asset_type` is required"))?;
        let extension =
            extension.ok_or_else(|| syn::Error::new(input.span(), "`extension` is required"))?;

        Ok(ResolverArgs {
            base_path: Cow::Owned(base_path),
            asset_type: Cow::Owned(asset_type),
            extension: Cow::Owned(extension),
        })
    }
}

pub fn create_resolver_impl(
    type_ident: &impl ToTokens,
    args: &ResolverArgs,
) -> Result<TokenStream, syn::Error> {
    let engine_crate = resolve_crate_name("engine")?;
    let base_path = &args.base_path;
    let extension = &args.extension;
    let asset_type = &args.asset_type;
    Ok(quote! {
        impl #engine_crate::assets::AssetResolver for #type_ident {
            type Asset = #asset_type;
            const BASE_PATH: &'static str = #base_path;
            const EXTENSION: &'static str = #extension;
        }
    })
}
