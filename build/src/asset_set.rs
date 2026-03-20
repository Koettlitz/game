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

use crate::{AssetSource, resolve_crate_path};

pub struct AssetSetArgs {
    pub name: Option<LitStr>,
    pub folder: LitStr,
    pub asset_type: syn::Path,
}

impl Parse for AssetSetArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<LitStr> = None;
        let mut folder: Option<LitStr> = None;
        let mut asset_type: Option<syn::Path> = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            match ident.to_string().as_str() {
                "name" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    name = Some(lit);
                }

                "folder" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    folder = Some(lit);
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
                        "Unknown parameter. Expected `name`, `folder`, or `asset_type`.",
                    ));
                }
            }
            // optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let folder = folder.ok_or_else(|| syn::Error::new(input.span(), "`folder` is required"))?;
        let asset_type =
            asset_type.ok_or_else(|| syn::Error::new(input.span(), "`asset_type` is required"))?;

        Ok(AssetSetArgs {
            name,
            folder,
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
    asset_source: AssetSource,
    asset_root: &Path,
    asset_paths: &HashMap<PathBuf, Vec<PathBuf>>,
) -> String {
    let mut output = String::new();
    let asset_source_prefix = asset_source.prefix();
    let mut asset_paths: Vec<_> = asset_paths.iter().collect();
    asset_paths.sort_by_key(|(d, _)| *d);
    for (dir, contents) in asset_paths {
        let mut asset_paths = Vec::new();
        for path in contents {
            let path = path.strip_prefix(&asset_root).unwrap_or_else(|e| {
                panic!(
                    "could not strip prefix asset root {} from asset path {} - {e}",
                    asset_root.display(),
                    path.display()
                )
            });
            let mut asset_path = asset_source_prefix.unwrap_or_default().to_string();
            asset_path.push_str(&path.to_string_lossy());
            asset_paths.push(asset_path);
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
        asset_paths.sort();
        for asset_path in asset_paths {
            let asset_path = asset_path.replace('\\', "/");
            output.push_str(&format!("\n    \"{asset_path}\","));
        }
        output.push_str(&format!("\n];\n"));
    }
    output
}

pub fn create_asset_set_impl(
    args: AssetSetArgs,
    input_struct: &ItemStruct,
) -> syn::Result<proc_macro2::TokenStream> {
    let asset_type = &args.asset_type;
    let folder_path = args.folder.value();
    let asset_path = folder_path
        .split_once("://")
        .map(|(_, p)| p)
        .unwrap_or(&folder_path);
    let const_name = asset_path.replace(['/', '\\', '.'], "_").to_uppercase();
    let const_ident = Ident::new(&const_name, Span::call_site());
    let struct_ident = &input_struct.ident;
    let engine_crate = resolve_crate_path("engine")?;
    let (impl_generics, ty_generics, where_clause) = input_struct.generics.split_for_impl();
    let name_impl = args.name.map(|name| {
        quote! {
            fn name() -> std::option::Option<&'static str> {
                Some(#name)
            }
        }
    });
    let expanded = quote! {
        impl #impl_generics #engine_crate::assets::folder::AssetSet for #struct_ident #ty_generics
            #where_clause
        {
            type Asset = #asset_type;
            const PATHS: &'static [&'static str] = crate::asset_registry::#const_ident;

            #name_impl
        }
    };
    Ok(expanded)
}
