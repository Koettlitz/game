use std::path::{Path, PathBuf};

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::Ident;

pub mod asset_enum;
pub mod asset_set;
pub mod from_def;
pub mod resolver;

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
