use std::{
    fs, ops,
    path::{Path, PathBuf},
};

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::{ToTokens, quote};
use syn::{Ident, TypePath, spanned::Spanned};

use crate::asset_enum::BsError;

pub mod asset_enum;
pub mod asset_set;
pub mod from_def;
pub mod spec;

pub const ASSET_MODULE_PATH: &'static str = "engine::asset";
pub const ASSET_SET_MODULE_PATH: &'static str = "engine::asset::set";

#[derive(Clone, Copy, Debug)]
pub enum AssetSource {
    Workspace,
    Editor,
    Game,
}

impl AssetSource {
    pub fn asset_root(crate_root: &Path) -> PathBuf {
        crate_root.to_path_buf().join("assets")
    }

    pub fn prefix(&self) -> Option<&'static str> {
        match self {
            Self::Workspace => None,
            Self::Editor => Some("editor://"),
            Self::Game => Some("game://"),
        }
    }
}

pub fn resolve_crate_name(orig_name: &str) -> syn::Result<proc_macro2::TokenStream> {
    match crate_name(orig_name) {
        Ok(FoundCrate::Itself) => Ok(quote!(crate)),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            Ok(quote!(#ident))
        }
        Err(e) => Err(syn::Error::new(
            Span::call_site(),
            format!("could not resolve crate {orig_name} - {e}"),
        )),
    }
}

pub fn write_out(path: &Path, content: &impl AsRef<[u8]>) -> Result<(), BsError> {
    let out_dir = std::env::var("OUT_DIR")?;
    let path = Path::new(&out_dir).join(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, content)
        .map_err(|e| BsError::io(e, format!("failed to write to {}", path.display())))
}

/// A [`syn::Path`] whose first segment is rewritten to the correct crate name.
///
/// Resolution happens in [`CratePath::try_from`] using
/// [`proc_macro_crate::crate_name`].
#[derive(Clone)]
pub struct CratePath(syn::Path);

impl TryFrom<&str> for CratePath {
    type Error = syn::Error;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        let path: syn::Path = syn::parse_str(path)?;
        Self::try_from(path)
    }
}

impl TryFrom<syn::Path> for CratePath {
    type Error = syn::Error;

    fn try_from(mut path: syn::Path) -> Result<Self, Self::Error> {
        let first_segment = match path.segments.first_mut() {
            Some(segment) => segment,
            None => {
                return Err(syn::Error::new(
                    path.span(),
                    "wtf is this? Comon man! Don't gimme that empty syn::Path abomination! I can't...",
                ));
            }
        };
        let crate_string = first_segment.ident.to_string();
        let span = first_segment.ident.span();
        first_segment.ident = match crate_name(&crate_string) {
            Ok(FoundCrate::Itself) => Ident::new("crate", span),
            Ok(FoundCrate::Name(name)) => Ident::new(&name, span),
            Err(e) => {
                return Err(syn::Error::new(
                    span,
                    format!("could not resolve crate `{crate_string}`: {e}"),
                ));
            }
        };
        Ok(Self(path))
    }
}

impl ops::Deref for CratePath {
    type Target = syn::Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<syn::Path> for CratePath {
    fn as_ref(&self) -> &syn::Path {
        &self.0
    }
}

impl From<CratePath> for syn::Path {
    fn from(p: CratePath) -> Self {
        p.0
    }
}

impl ToTokens for CratePath {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0.to_tokens(tokens);
    }
}

pub fn is_self(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(TypePath { qself: None, path }) => path.is_ident("Self"),
        _ => false,
    }
}

pub fn extension_matches(file_name: &str, extension: &str) -> bool {
    let extension_split: Vec<&str> = extension.split('.').collect();
    let mut file_name_split: Vec<&str> = file_name.split('.').collect();
    file_name_split.remove(0);
    if extension_split.len() != file_name_split.len() {
        return false;
    }

    extension_split
        .into_iter()
        .rev()
        .zip(file_name_split.into_iter().rev())
        .all(|(a, b)| a == b)
}

#[cfg(test)]
mod test {
    #[test]
    fn single_extension_matches() {
        let file_name = "foo.bar";
        let extension = "bar";
        assert!(super::extension_matches(file_name, extension));
    }

    #[test]
    fn double_extension_matches() {
        let file_name = "foo.bar.baz";
        let extension = "bar.baz";
        assert!(super::extension_matches(file_name, extension));
    }

    #[test]
    fn single_extension_doesnt_match() {
        let file_name = "foo.bar";
        let extension = "baz";
        assert!(!super::extension_matches(file_name, extension));
    }

    #[test]
    fn single_extension_doesnt_match_double() {
        let file_name = "foo.bar.baz";
        let extension = "bar";
        assert!(!super::extension_matches(file_name, extension));
        let file_name = "foo.bar.baz";
        let extension = "baz";
        assert!(!super::extension_matches(file_name, extension));
    }

    #[test]
    fn double_extension_doesnt_match_single() {
        let file_name = "foo.bar";
        let extension = "bar.baz";
        assert!(!super::extension_matches(file_name, extension));
        let file_name = "foo.baz";
        let extension = "bar.baz";
        assert!(!super::extension_matches(file_name, extension));
    }
}
