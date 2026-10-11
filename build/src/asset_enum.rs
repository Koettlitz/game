use bevy_elf::ResolveStrategy;
use ron::de::SpannedError;
use std::{
    collections::HashMap,
    env::VarError,
    fmt::{self, Display},
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf, StripPrefixError},
};
use thiserror::Error;

use crate::{ASSET_SET_MODULE_PATH, AssetSource, CratePath, resolve_crate_name};
use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

const DESCRIPTOR_FILE_NAME: &str = ".build.ron";

pub fn generate_resolver_enums(
    asset_source: AssetSource,
    asset_root: &Path,
) -> Result<HashMap<PathBuf, TokenStream>, BsError> {
    let mut result = HashMap::new();
    generate_resolver_enums_in(asset_source, asset_root, asset_root, &mut result)?;
    Ok(result)
}

struct InspectedDir {
    descriptor_file: Option<PathBuf>,
    asset_files: Vec<PathBuf>,
    asset_dirs: Vec<PathBuf>,
}

fn inspect_dir(asset_root: &Path, asset_dir: &Path) -> Result<InspectedDir, BsError> {
    let mut descriptor_file = None;
    let mut asset_files = Vec::new();
    let mut asset_dirs = Vec::new();

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
        let absolute_path = file.path();
        let relative_path = absolute_path.strip_prefix(asset_root)?;
        if file.file_type()?.is_dir() {
            asset_dirs.push(relative_path.to_path_buf());
        } else if relative_path
            .file_name()
            .is_some_and(|n| n == DESCRIPTOR_FILE_NAME)
        {
            descriptor_file = Some(absolute_path.to_path_buf());
        } else {
            // TODO: remove this fragile workaround and implement paths for top level assets
            // properly
            if asset_root != asset_dir {
                asset_files.push(relative_path.to_path_buf());
            }
        }
    }

    Ok(InspectedDir {
        descriptor_file,
        asset_files,
        asset_dirs,
    })
}

fn generate_resolver_enums_in(
    asset_source: AssetSource,
    asset_root: &Path,
    asset_dir: &Path,
    enums: &mut HashMap<PathBuf, TokenStream>,
) -> Result<(), BsError> {
    let inspected_dir = inspect_dir(asset_root, asset_dir)?;

    eprintln!(
        "found {} dirs under {}",
        inspected_dir.asset_dirs.len(),
        asset_dir.display()
    );
    for asset_dir in inspected_dir.asset_dirs {
        generate_resolver_enums_in(asset_source, asset_root, &asset_dir, enums)?;
    }

    let Some(descriptor) = inspected_dir.descriptor_file else {
        eprintln!(
            "No build descriptor file in {} found. Skipping directory.",
            asset_dir.display()
        );
        return Ok(());
    };

    eprintln!("parsing build descriptor: {}", descriptor.display());
    // Leaking the value here is ok, cause this is the build process, so its short lived
    let descriptor = fs::read_to_string(&descriptor)
        .map_err(|e| BsError::Io {
            e,
            msg: Some(format!(
                "Invalid path to build descriptor: {}",
                descriptor.display()
            )),
        })?
        .leak();
    eprintln!(
        "parsed descriptor for dir {}: {descriptor}",
        asset_dir.display()
    );
    let resolve_strategy: ResolveStrategy = ron::de::from_str(descriptor).map_err(|e| {
        let msg = format!("Failed to parse descriptor {descriptor} - {e}");
        BsError::Spanned { e, msg: Some(msg) }
    })?;

    if asset_root != asset_dir {
        let enum_name = derive_enum_type_name(asset_dir)?;
        let progress_name = derive_progress_name(asset_dir)?;

        let resolver_enum = generate_resolver_enum(
            &enum_name,
            &progress_name,
            inspected_dir.asset_files,
            &resolve_strategy,
            asset_source,
        )?;
        let generated_file_path = derive_enum_file_name(asset_dir)?;
        enums.insert(generated_file_path, resolver_enum);
    }

    Ok(())
}

fn generate_resolver_enum(
    enum_name: &str,
    progress_name: &str,
    file_paths: impl IntoIterator<Item = impl AsRef<Path>>,
    resolve_strategy: &ResolveStrategy,
    asset_source: AssetSource,
) -> Result<TokenStream, BsError> {
    let mut variant_idents = Vec::new();
    let mut variant_ids = Vec::new();
    let mut asset_paths = Vec::new();

    for file_path in file_paths {
        let file_path = file_path.as_ref();
        let Some(file_name) = file_path.file_name() else {
            continue;
        };
        let file_name = file_name.to_string_lossy().to_string();

        match resolve_strategy {
            ResolveStrategy::FileStemIsId { .. } => {
                let variant_id = file_name.split('.').next().unwrap().to_string();
                let variant_ident = variant_id.to_case(Case::UpperCamel);
                variant_idents.push(Ident::new(&variant_ident, Span::call_site()));
                variant_ids.push(variant_id);
            }
            ResolveStrategy::FileNameIsId | ResolveStrategy::DirectoryNameIsId { .. } => {
                let variant_ident = file_name.to_case(Case::UpperCamel);
                variant_idents.push(Ident::new(&variant_ident, Span::call_site()));
                variant_ids.push(file_name);
            }
        }

        let base_path = if let Some(prefix) = asset_source.prefix() {
            format!("{prefix}{}", file_path.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };
        let asset_path = match resolve_strategy {
            ResolveStrategy::FileStemIsId { .. } | ResolveStrategy::FileNameIsId => base_path,
            ResolveStrategy::DirectoryNameIsId { file_name } => {
                format!("{base_path}/{file_name}")
            }
        };

        asset_paths.push(syn::LitStr::new(&asset_path, Span::call_site()));
    }

    let enum_type: syn::Type = syn::parse_str(enum_name)?;
    let bevy_crate = resolve_crate_name("bevy")?;
    let elf_module = CratePath::try_from("bevy_elf")?;
    let asset_set_module = CratePath::try_from(ASSET_SET_MODULE_PATH)?;

    let asset_descriptor_impl = if asset_paths.is_empty() {
        quote! {
            impl #asset_set_module::AssetDescriptor for #enum_type {
                fn id(&self) -> &'static str {
                    panic!("How was this method even called on an enum without variants?")
                }

                fn asset_path(&self) -> #bevy_crate::asset::AssetPath<'static> {
                    panic!("How was this method even called on an enum without variants?")
                }
            }
        }
    } else {
        quote! {
            impl #asset_set_module::AssetDescriptor for #enum_type {
                fn id(&self) -> &'static str {
                    match self {
                        #(Self::#variant_idents => #variant_ids,)*
                    }
                }

                fn asset_path(&self) -> #bevy_crate::asset::AssetPath<'static> {
                    match self {
                        #(Self::#variant_idents => #bevy_crate::asset::AssetPath::from(#asset_paths),)*
                    }
                }
            }
        }
    };

    Ok(quote! {
        #[derive(Clone, Copy, Hash, PartialEq, Eq, #bevy_crate::prelude::TypePath, strum_macros::EnumIter)]
        pub enum #enum_type {
            #(#variant_idents),*
        }

        impl std::str::FromStr for #enum_type {
            type Err = #asset_set_module::InvalidAssetLinkError;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                match s {
                    #(#variant_ids => Ok(Self::#variant_idents),)*
                    _ => Err(#asset_set_module::InvalidAssetLinkError(s.to_string())),
                }
            }
        }

        #asset_descriptor_impl

        impl #asset_set_module::ProgressName for #enum_type {
            fn name<'a>() -> &'a str {
                #progress_name
            }
        }

        impl #elf_module::StaticAssetResolver for #enum_type {
            fn resolve(asset_id: &str) -> std::result::Result<#bevy_crate::asset::AssetPath<'static>, #elf_module::ResolveError> {
                let instance = <Self as std::str::FromStr>::from_str(asset_id)?;
                Ok(<Self as #asset_set_module::AssetDescriptor>::asset_path(&instance))
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
    Spanned {
        e: SpannedError,
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

impl From<SpannedError> for BsError {
    fn from(e: SpannedError) -> Self {
        Self::Spanned { e, msg: None }
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
            Self::Spanned { e, msg } => with_err_and_optional_msg(f, e, msg.as_deref()),
            Self::Syn(e) => e.fmt(f),
            Self::Var(e) => e.fmt(f),
        }
    }
}
