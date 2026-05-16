use std::borrow::Cow;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Ident, LitStr, Token, parse::Parse};

use crate::{ASSET_MODULE_PATH, CratePath};

pub struct SpecArgs<'a> {
    pub base_path: Cow<'a, LitStr>,
    pub extension: Option<Cow<'a, LitStr>>,
}

impl<'a> Parse for SpecArgs<'a> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut base_path: Option<LitStr> = None;
        let mut extension: Option<LitStr> = None;

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
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "Unknown parameter. Expected `base_path`, or `extension`",
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

        Ok(SpecArgs {
            base_path: Cow::Owned(base_path),
            extension: extension.map(|e| Cow::Owned(e)),
        })
    }
}

pub fn create_spec_impl(
    type_ident: &impl ToTokens,
    args: &SpecArgs,
) -> Result<TokenStream, syn::Error> {
    let asset_module = CratePath::try_from(ASSET_MODULE_PATH)?;
    let base_path = &args.base_path;
    let extension = args
        .extension
        .as_ref()
        .map(|e| quote!(Some(#e)))
        .unwrap_or_else(|| quote!(None));
    Ok(quote! {
        impl #asset_module::AssetPathSpec for #type_ident {
            const BASE_PATH: &'static str = #base_path;
            const EXTENSION: Option<&'static str> = #extension;
        }
    })
}
