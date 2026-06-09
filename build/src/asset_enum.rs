use std::{
    collections::HashMap,
    env::VarError,
    fmt::{self, Display},
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf, StripPrefixError},
};
use thiserror::Error;

use crate::{ASSET_MODULE_PATH, ASSET_SET_MODULE_PATH, AssetSource, CratePath, resolve_crate_name};
use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

pub fn generate_resolver_enums(
    asset_source: AssetSource,
    asset_root: &Path,
) -> Result<HashMap<PathBuf, TokenStream>, BsError> {
    let mut result = HashMap::new();
    generate_resolver_enums_in(asset_source, asset_root, asset_root, &mut result)?;
    Ok(result)
}

fn generate_resolver_enums_in(
    asset_source: AssetSource,
    asset_root: &Path,
    asset_dir: &Path,
    enums: &mut HashMap<PathBuf, TokenStream>,
) -> Result<(), BsError> {
    let mut asset_files = Vec::new();
    let read_dir = match fs::read_dir(asset_root.join(asset_dir)) {
        Ok(dir) => dir,
        Err(e) => {
            return Err(BsError::io(
                e,
                format!("could not read asset_dir \"{}\"", asset_dir.display()),
            ));
        }
    };

    for file in read_dir {
        let file = file?;
        let path = file.path();
        let path = path.strip_prefix(asset_root)?;
        if file.file_type()?.is_dir() {
            generate_resolver_enums_in(asset_source, asset_root, path, enums)?;
        } else {
            // TODO: remove this fragile workaround and implement paths for top level assets
            // properly
            if asset_root != asset_dir {
                asset_files.push(path.to_path_buf());
            }
        }
    }

    if !asset_files.is_empty() {
        let enum_name = derive_enum_type_name(asset_dir)?;
        let progress_name = derive_progress_name(asset_dir)?;
        let resolver_enum =
            generate_resolver_enum(&enum_name, &progress_name, asset_files, asset_source)?;
        let generated_file_path = derive_enum_file_name(asset_dir)?;
        enums.insert(generated_file_path, resolver_enum);
    };
    Ok(())
}

fn generate_resolver_enum(
    enum_name: &str,
    progress_name: &str,
    file_paths: impl IntoIterator<Item = impl AsRef<Path>>,
    asset_source: AssetSource,
) -> Result<TokenStream, BsError> {
    let mut variant_idents = Vec::new();
    let mut variant_strings = Vec::new();
    let mut asset_paths = Vec::new();
    for file in file_paths {
        let file = file.as_ref();
        let Some(file_name) = file.file_name() else {
            continue;
        };
        let file_name = file_name.to_string_lossy().to_string();
        let variant_string = file_name.split('.').next().unwrap().to_string();
        let variant_name = variant_string.to_case(Case::UpperCamel);
        variant_idents.push(Ident::new(&variant_name, Span::call_site()));
        variant_strings.push(variant_string);
        let asset_path = if let Some(prefix) = asset_source.prefix() {
            format!("{prefix}{}", file.to_string_lossy())
        } else {
            file.to_string_lossy().to_string()
        };
        asset_paths.push(syn::LitStr::new(&asset_path, Span::call_site()));
    }
    // TODO: determine default another way (e.g. descriptor file)
    let default_variant_ident = variant_idents.remove(0);
    let default_variant_path = asset_paths.remove(0);
    let default_variant_string = variant_strings.remove(0);
    let enum_type: syn::Type = syn::parse_str(&enum_name)?;
    let bevy_crate = resolve_crate_name("bevy")?;
    let asset_module = CratePath::try_from(ASSET_MODULE_PATH)?;
    let asset_set_module = CratePath::try_from(ASSET_SET_MODULE_PATH)?;
    Ok(quote! {
        #[derive(Default, Clone, Copy, Hash, PartialEq, Eq, #bevy_crate::prelude::TypePath, strum_macros::EnumIter)]
        pub enum #enum_type {
            #[default]
            #default_variant_ident,
            #(#variant_idents),*
        }

        impl std::str::FromStr for #enum_type {
            type Err = #asset_set_module::InvalidAssetLinkError;
            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                match s {
                    #default_variant_string => Ok(Self::#default_variant_ident),
                    #(#variant_strings => Ok(Self::#variant_idents),)*
                    _ => Err(#asset_set_module::InvalidAssetLinkError(s.to_string())),
                }
            }
        }

        impl #asset_set_module::AsAssetPath for #enum_type {
            fn as_asset_path(&self) -> #bevy_crate::asset::AssetPath<'static> {
                match self {
                    Self::#default_variant_ident => #bevy_crate::asset::AssetPath::from(#default_variant_path),
                    #(Self::#variant_idents => #bevy_crate::asset::AssetPath::from(#asset_paths),)*
                }
            }
        }

        impl #asset_set_module::ProgressName for #enum_type {
            fn name<'a>() -> &'a str {
                #progress_name
            }
        }

        impl #asset_module::StaticAssetResolver for #enum_type {
            fn resolve(asset_id: &str) -> std::result::Result<#bevy_crate::asset::AssetPath<'static>, #asset_module::FromDefError> {
                let instance = <Self as std::str::FromStr>::from_str(asset_id)?;
                Ok(<Self as #asset_set_module::AsAssetPath>::as_asset_path(&instance))
            }
        }
    })
}

pub fn derive_enum_file_name(base_path: &Path) -> io::Result<PathBuf> {
    let base_name = stripped_file_name(base_path)?;
    let file_name = base_name.to_case(Case::Snake) + ".rs";
    Ok(if let Some(parent) = base_path.parent() {
        parent.join(&file_name)
    } else {
        PathBuf::from(file_name)
    })
}

pub fn derive_enum_type_name(base_path: &Path) -> io::Result<String> {
    let base_name = stripped_file_name(base_path)?.to_case(Case::UpperCamel) + "ResolverSet";
    Ok(base_name)
}

pub fn derive_progress_name(base_path: &Path) -> io::Result<String> {
    stripped_file_name(base_path)
}

fn stripped_file_name(base_path: &Path) -> io::Result<String> {
    let Some(dir_name) = base_path.file_name() else {
        return Err(io::Error::new(
            ErrorKind::InvalidFilename,
            format!("no file name for base_path {}", base_path.display()),
        ));
    };
    let base_name = dir_name
        .to_string_lossy()
        .to_string()
        .split('.')
        .next()
        .unwrap()
        .to_string();
    Ok(base_name
        .strip_suffix('s')
        .unwrap_or(&base_name)
        .to_string())
}

#[derive(Error, Debug)]
pub enum BsError {
    Io {
        #[source]
        e: io::Error,
        msg: Option<String>,
    },
    PathError {
        #[source]
        e: StripPrefixError,
        msg: Option<String>,
    },
    Syn(syn::Error),
    Var(#[from] VarError),
}

impl BsError {
    pub fn io(e: io::Error, msg: String) -> Self {
        Self::Io { e, msg: Some(msg) }
    }
}

impl From<io::Error> for BsError {
    fn from(e: io::Error) -> Self {
        Self::Io { e, msg: None }
    }
}

impl From<StripPrefixError> for BsError {
    fn from(e: StripPrefixError) -> Self {
        Self::PathError { e, msg: None }
    }
}

impl From<syn::Error> for BsError {
    fn from(e: syn::Error) -> Self {
        Self::Syn(e)
    }
}

impl Display for BsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn with_err_and_optional_msg(
            f: &mut fmt::Formatter<'_>,
            e: impl Display,
            msg: Option<&str>,
        ) -> fmt::Result {
            if let Some(msg) = msg {
                write!(f, "{msg} - {e}")
            } else {
                e.fmt(f)
            }
        }
        match self {
            Self::Io { e, msg } => with_err_and_optional_msg(f, e, msg.as_deref()),
            Self::PathError { e, msg } => with_err_and_optional_msg(f, e, msg.as_deref()),
            Self::Syn(e) => e.fmt(f),
            Self::Var(e) => e.fmt(f),
        }
    }
}
