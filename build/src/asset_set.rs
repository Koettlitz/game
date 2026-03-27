use std::{
    collections::HashMap,
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use proc_macro2::Span;
use quote::quote;
use syn::{
    Ident, ItemStruct, LitStr, Token,
    parse::{Parse, ParseStream},
};

use crate::resolve_crate_name;

pub struct AssetSetArgs {
    pub name: Option<LitStr>,
    pub base_path: LitStr,
    pub extension: LitStr,
    pub asset_registry: syn::Path,
    pub asset_type: syn::Path,
}

impl Parse for AssetSetArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<LitStr> = None;
        let mut base_path: Option<LitStr> = None;
        let mut extension: Option<LitStr> = None;
        let mut asset_registry: Option<syn::Path> = None;
        let mut asset_type: Option<syn::Path> = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    name = Some(lit);
                }

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

                "asset_registry" => {
                    let content;
                    syn::parenthesized!(content in input);
                    let path: syn::Path = content.parse()?;
                    asset_registry = Some(path);
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
                        "Unknown parameter. Expected `name`, `base_path`, `extension`, `asset_registry`, or `asset_type`.",
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
        let extension =
            extension.ok_or_else(|| syn::Error::new(input.span(), "`extension` is required"))?;
        let asset_type =
            asset_type.ok_or_else(|| syn::Error::new(input.span(), "`asset_type` is required"))?;
        let asset_registry = asset_registry
            .ok_or_else(|| syn::Error::new(input.span(), "`asset_registry` is required"))?;

        Ok(AssetSetArgs {
            name,
            base_path,
            extension,
            asset_registry,
            asset_type,
        })
    }
}

pub fn scan_asset_dir(
    dir: &Path,
    contents: &mut HashMap<PathBuf, Vec<PathBuf>>,
) -> Result<(), io::Error> {
    let mut dir_contents = Vec::new();
    let mut extension = None;
    for file in fs::read_dir(dir)? {
        let file = file?.path();
        if file.is_dir() {
            scan_asset_dir(&file, contents)?;
            continue;
        }

        let current_ext = file.extension().ok_or_else(|| {
            io::Error::new(
                ErrorKind::Other,
                format!("missing file extension for file {file:?}"),
            )
        })?;
        if let Some(ref ext) = extension {
            if ext != current_ext {
                println!(
                    "cargo::warning=mixed file extensions in asset directory {}",
                    dir.display()
                );
            }
        } else {
            extension = Some(current_ext.to_os_string());
        }
        dir_contents.push(file);
    }
    if !dir_contents.is_empty() {
        contents.insert(dir.to_path_buf(), dir_contents);
    }

    Ok(())
}

pub fn generate_path_consts(
    asset_root: &Path,
    asset_paths: &HashMap<PathBuf, Vec<PathBuf>>,
) -> String {
    let mut output = String::new();
    let mut asset_paths: Vec<_> = asset_paths.iter().collect();
    asset_paths.sort_by_key(|(d, _)| *d);
    for (dir, contents) in asset_paths {
        let mut names = Vec::new();
        for path in contents {
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let name = file_name
                .to_string_lossy()
                .split('.')
                .next()
                .unwrap()
                .to_string();
            names.push(name);
        }
        let const_name = &dir
            .strip_prefix(asset_root)
            .unwrap_or_else(|e| {
                panic!(
                    "could not strip prefix asset root {} from asset dir path {} - {e}",
                    asset_root.display(),
                    dir.display()
                )
            })
            .to_string_lossy()
            .replace(['/', '\\', '.'], "_")
            .to_uppercase();
        output.push_str(&format!("pub const {const_name}: &[&str] = &["));
        names.sort();
        for name in names {
            output.push_str(&format!("\n    \"{name}\","));
        }
        output.push_str(&format!("\n];\n"));
    }
    output
}

pub fn create_asset_set_impl(
    args: &AssetSetArgs,
    input_struct: &ItemStruct,
    resolver_type: &syn::Path,
) -> syn::Result<proc_macro2::TokenStream> {
    let base_path = args.base_path.value();
    let asset_path = base_path
        .split_once("://")
        .map(|(_, p)| p)
        .unwrap_or(&base_path);
    let const_name = asset_path.replace(['/', '\\', '.'], "_").to_uppercase();
    let const_ident = Ident::new(&const_name, Span::call_site());
    let struct_ident = &input_struct.ident;
    let asset_registry = &args.asset_registry;
    let engine_crate = resolve_crate_name("engine")?;
    let (impl_generics, ty_generics, where_clause) = input_struct.generics.split_for_impl();
    let name_impl = args.name.as_ref().map(|name| {
        quote! {
            fn name() -> &'static str {
                #name
            }
        }
    });
    let expanded = quote! {
        impl #impl_generics #engine_crate::assets::folder::AssetSet for #struct_ident #ty_generics
            #where_clause
        {
            type Resolver = #resolver_type;
            const NAMES: &'static [&'static str] = #asset_registry::#const_ident;

            #name_impl
        }
    };
    Ok(expanded)
}
