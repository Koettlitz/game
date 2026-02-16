use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use std::{fs, path::PathBuf};
use syn::{
    DeriveInput, Error, Ident, ItemStruct, LitStr, Path, Result, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

#[proc_macro_derive(FileAsset)]
pub fn file_asset(item: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(item as DeriveInput);
    match resolve_crate_path("engine") {
        Ok(engine_crate) => quote! {
            impl #engine_crate::assets::folder::FileAsset for #ident {}
        }
        .into(),
        Err(e) => e.into_compile_error().into(),
    }
}

struct AssetSetArgs {
    name: Option<LitStr>,
    extension: Option<LitStr>,
    folder: LitStr,
    asset_type: Path,
}

impl Parse for AssetSetArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name: Option<LitStr> = None;
        let mut extension: Option<LitStr> = None;
        let mut folder: Option<LitStr> = None;
        let mut asset_type: Option<Path> = None;

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
                    let ty: Path = content.parse()?;
                    asset_type = Some(ty);
                }
                "extension" => {
                    input.parse::<Token![=]>()?;
                    let lit: LitStr = input.parse()?;
                    extension = Some(lit);
                }
                _ => {
                    return Err(Error::new(
                        ident.span(),
                        "Unknown parameter. Expected `name`, `extension`, `folder`, or `asset_type`.",
                    ));
                }
            }
            // optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let folder = folder.ok_or_else(|| Error::new(input.span(), "`folder` is required"))?;
        let asset_type =
            asset_type.ok_or_else(|| Error::new(input.span(), "`asset_type` is required"))?;

        Ok(AssetSetArgs {
            name,
            extension,
            folder,
            asset_type,
        })
    }
}

/// NOTE:
/// This macro reads the filesystem during expansion.
/// If asset folders change and rebuilds do not trigger,
/// consider moving fs scanning to build.rs.
#[proc_macro_attribute]
pub fn asset_set(attrs: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attrs as AssetSetArgs);
    let input_struct = parse_macro_input!(item as ItemStruct);
    match create_asset_set_impl(args, &input_struct) {
        Ok(impl_block) => quote! {
            #input_struct
            #impl_block
        }
        .into(),
        Err(e) => {
            let compile_error = e.to_compile_error();
            let expanded = quote! {
                #input_struct
                #compile_error
            };
            expanded.into()
        }
    }
}

fn create_asset_set_impl(
    args: AssetSetArgs,
    input_struct: &ItemStruct,
) -> Result<proc_macro2::TokenStream> {
    let asset_type = &args.asset_type;
    let set_name = args.name;
    let folder = args.folder;
    let asset_path = AssetFolderPath::new(&folder)?;
    let read_dir = fs::read_dir(&asset_path.fs_path);
    let read_dir = match read_dir {
        Ok(dir) => dir,
        Err(e) => {
            return Err(syn::Error::new(
                folder.span(),
                format!(
                    "could not access directory {} - {e}",
                    asset_path.fs_path.display()
                ),
            ));
        }
    };
    let mut paths = Vec::with_capacity(read_dir.size_hint().0);
    for file in read_dir {
        match file {
            Ok(file) => {
                if let Some(ref extension) = args.extension {
                    if !has_extension(&file.file_name().to_string_lossy(), &extension.value()) {
                        continue;
                    }
                }
                let mut path = asset_path.asset_path.value();
                if !path.ends_with("/") {
                    path.push('/');
                }
                path.push_str(&file.file_name().to_string_lossy());
                paths.push(path);
            }
            Err(e) => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    format!("could not access asset file - {e}"),
                ));
            }
        }
    }
    if paths.is_empty() {
        let error_msg = if let Some(extension) = args.extension {
            format!(
                "no file with extension {} found in asset folder {}",
                extension.value(),
                asset_path.fs_path.display()
            )
        } else {
            format!("no file found in asset folder {}", folder.value())
        };
        return Err(syn::Error::new(folder.span(), error_msg));
    }
    paths.sort();
    let engine_crate = resolve_crate_path("engine")?;
    let bevy_crate = resolve_crate_path("bevy")?;
    let struct_ident = &input_struct.ident;
    let (impl_generics, ty_generics, where_clause) = input_struct.generics.split_for_impl();
    let name_impl = set_name.map(|name| {
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

            fn paths() -> std::vec::Vec<impl std::convert::Into<#bevy_crate::asset::AssetPath<'static>>> {
                vec![#(#paths),*]
            }

            #name_impl
        }
    };
    Ok(expanded)
}

struct AssetFolderPath {
    asset_path: LitStr,
    fs_path: PathBuf,
}

impl AssetFolderPath {
    fn new(asset_path: &LitStr) -> Result<Self> {
        let Some(mut fs_path) = workspace_root() else {
            return Err(syn::Error::new(
                Span::call_site(),
                "could not find workspace root",
            ));
        };
        let asset_path_value = asset_path.value();
        let split: Vec<&str> = asset_path_value.split("://").collect();
        if split.len() == 1 {
            fs_path.push("assets");
            fs_path.push(&asset_path_value);
            Ok(Self {
                asset_path: asset_path.clone(),
                fs_path,
            })
        } else if split.len() == 2 {
            let prefix = match split[0] {
                "editor" => "editor/assets",
                prefix => {
                    return Err(Error::new(
                        asset_path.span(),
                        format!("unknown asset source prefix {prefix}"),
                    ));
                }
            };
            fs_path.push(prefix);
            fs_path.push(split[1]);
            Ok(Self {
                asset_path: asset_path.clone(),
                fs_path,
            })
        } else {
            Err(Error::new(
                asset_path.span(),
                format!("invalid asset path {}", asset_path.value()),
            ))
        }
    }
}

fn workspace_root() -> Option<PathBuf> {
    let mut dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("missing environment variable CARGO_MANIFEST_DIR."),
    );
    loop {
        let cargo_toml = dir.join("Cargo.toml");

        if cargo_toml.exists() {
            let contents = std::fs::read_to_string(&cargo_toml).ok()?;
            if contents.contains("[workspace]") {
                return Some(dir);
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn has_extension(file_name: &str, extension: &str) -> bool {
    let extension: Vec<&str> = extension.split('.').collect();
    let path: Vec<&str> = file_name.split('.').collect();
    if extension.len() > path.len() {
        false
    } else {
        extension
            .iter()
            .rev()
            .zip(path.iter().rev())
            .all(|(a, b)| a == b)
    }
}

fn resolve_crate_path(crate_path: &str) -> Result<proc_macro2::TokenStream> {
    match crate_name(crate_path) {
        Ok(FoundCrate::Itself) => Ok(quote!(crate)),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            Ok(quote!(#ident))
        }
        Err(e) => Err(Error::new(
            Span::call_site(),
            format!("could not resolve crate {crate_path} - {e}"),
        )),
    }
}
