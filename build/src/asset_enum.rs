use std::{
    fmt::{self, Display},
    fs,
    io::{self, ErrorKind},
    path::{Path, StripPrefixError},
};

use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::{AssetSource, extension_matches, resolve_crate_path};

pub fn generate_enum(
    asset_root: &Path,
    asset_folder: &Path,
    asset_source: AssetSource,
    enum_name: Option<&str>,
    extension: Option<&str>,
) -> Result<TokenStream, BsError> {
    let mut variants = Vec::new();
    let mut paths = Vec::new();
    for file in fs::read_dir(asset_folder)? {
        let file = file?;
        let file_name = file.file_name().to_string_lossy().to_string();
        if let Some(extension) = extension {
            if !extension_matches(&file_name, extension) {
                continue;
            }
        }
        let variant_name = file_name
            .split('.')
            .next()
            .unwrap()
            .to_case(Case::UpperCamel);
        variants.push(Ident::new(&variant_name, Span::call_site()));
        let asset_path = match file.path().strip_prefix(asset_root) {
            Ok(asset_path) => asset_path.to_string_lossy().to_string(),
            Err(e) => {
                return Err(BsError::PathError {
                    e,
                    msg: Some(format!(
                        "could not make asset path {} relative to asset root {}",
                        file.path().display(),
                        asset_root.display()
                    )),
                });
            }
        };
        let asset_path = if let Some(prefix) = asset_source.prefix() {
            format!("{prefix}{}", asset_path)
        } else {
            asset_path
        };
        paths.push(syn::LitStr::new(&asset_path, Span::call_site()));
    }
    // TODO: determine default another way (e.g. descriptor file)
    let default_variant = variants.remove(0);
    let default_variant_path = paths.remove(0);
    let enum_name = if let Some(enum_name) = enum_name {
        enum_name.to_string()
    } else {
        derive_enum_name(asset_folder)?
    };
    let enum_ident = Ident::new(&enum_name, Span::call_site());
    let bevy_crate = resolve_crate_path("bevy")?;
    Ok(quote! {
        #[derive(Default, Clone, Copy, Hash, PartialEq, Eq)]
        pub enum #enum_ident {
            #[default]
            #default_variant,
            #(#variants),*
        }

        impl #enum_ident {
            pub fn asset_path(&self) -> #bevy_crate::asset::AssetPath<'_> {
                match self {
                    Self::#default_variant => #bevy_crate::asset::AssetPath::from(#default_variant_path),
                    #(Self::#variants => #bevy_crate::asset::AssetPath::from(#paths),)*
                }
            }
        }
    })
}

fn derive_enum_name(asset_folder: &Path) -> io::Result<String> {
    let Some(folder_name) = asset_folder.file_name() else {
        return Err(io::Error::new(
            ErrorKind::InvalidFilename,
            format!("no file name for asset_folder {}", asset_folder.display()),
        ));
    };
    Ok(folder_name
        .to_string_lossy()
        .split('.')
        .next()
        .unwrap()
        .to_case(Case::UpperCamel))
}

#[derive(Debug)]
pub enum BsError {
    Io {
        e: io::Error,
        msg: Option<String>,
    },
    PathError {
        e: StripPrefixError,
        msg: Option<String>,
    },
    Syn(syn::Error),
}

impl From<io::Error> for BsError {
    fn from(e: io::Error) -> Self {
        Self::Io { e, msg: None }
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
        }
    }
}
