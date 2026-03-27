use std::env::VarError;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::{fs, io};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use build::asset_enum::{AssetKind, BsError, generate_enum};
use build::{AssetSource, resolve_crate_name};
use quote::quote;

#[derive(EnumIter, Clone, Copy)]
enum GameAssetKind {
    Lozo,
    SpriteSheet,
    Animation,
    TileSprite,
    GameObject,
}

impl AssetKind for GameAssetKind {
    fn enum_name(&self) -> Option<&'static str> {
        Some(match self {
            Self::Lozo => "Lozo",
            Self::SpriteSheet => "SpriteSheet",
            Self::Animation => "SpriteAnimation",
            Self::TileSprite => "TileSprite",
            Self::GameObject => "GameObject",
        })
    }

    fn asset_type(&self) -> syn::TypePath {
        let engine_crate = resolve_crate_name("engine").unwrap();
        syn::parse2(match self {
            Self::Lozo => quote!(#engine_crate::assets::overworld::lozo::LozoAsset),
            Self::SpriteSheet => quote!(#engine_crate::assets::sprite_sheet::SpriteSheetAsset),
            Self::Animation => quote!(#engine_crate::animation::SpriteAnimation),
            Self::TileSprite => quote!(#engine_crate::assets::overworld::TileSpriteAsset),
            Self::GameObject => quote!(#engine_crate::assets::overworld::GameObjectAsset),
        })
        .unwrap()
    }

    fn folder_path(&self) -> &'static Path {
        Path::new(match self {
            Self::Lozo => "lozo",
            Self::SpriteSheet => "sprite_sheets",
            Self::Animation => "sprite_animation",
            Self::TileSprite => "tile_sprites",
            Self::GameObject => "game_objects",
        })
    }

    fn file_extension(&self) -> Option<&'static str> {
        match self {
            Self::Lozo => Some("lozo.ron"),
            Self::Animation => Some("ani.ron"),
            Self::TileSprite => Some("ts.ron"),
            Self::GameObject => Some("obj.ron"),
            Self::SpriteSheet => None,
        }
    }
}

impl GameAssetKind {
    fn file_name(&self) -> &'static str {
        match self {
            Self::Lozo => "lozo.rs",
            Self::SpriteSheet => "sprite_sheet.rs",
            Self::Animation => "sprite_animation.rs",
            Self::TileSprite => "tile_sprite.rs",
            Self::GameObject => "game_object.rs",
        }
    }
}

fn main() -> Result<(), Error> {
    // let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // let asset_root = crate_root.join("assets");
    // if !asset_root.exists() {
    //     if let Err(e) = fs::create_dir(&asset_root) {
    //         return Err(Error::Io {
    //             msg: format!(
    //                 "could not create missing asset root {}",
    //                 asset_root.display()
    //             ),
    //             e,
    //         });
    //     }
    // }
    // let workspace_root = crate_root
    //     .parent()
    //     .unwrap_or_else(|| panic!("game crate root has no parent directory?"));
    // let workspace_asset_root = workspace_root.join("assets");
    // println!("cargo:rerun-if-changed={}", workspace_asset_root.display());
    // println!("cargo:rerun-if-changed={}", asset_root.display());
    //
    // for asset_kind in GameAssetKind::iter() {
    //     let generated = generate_enum(&asset_root, AssetSource::Game, &asset_kind)?;
    //     write_file(asset_kind.file_name(), &generated.to_string())?;
    // }
    Ok(())
}

fn write_file(file_name: &str, content: &impl AsRef<[u8]>) -> Result<(), Error> {
    let out_dir = std::env::var("OUT_DIR")?;
    if let Err(e) = fs::write(Path::new(&out_dir).join(file_name), content) {
        Err(Error::Io {
            e,
            msg: format!("could not write generated file {file_name} to OUT_DIR {out_dir}"),
        })
    } else {
        Ok(())
    }
}

#[derive(Debug)]
enum Error {
    Io { e: io::Error, msg: String },
    MissingEnv(VarError),
    BuildScript(BsError),
}

impl From<VarError> for Error {
    fn from(e: VarError) -> Self {
        Self::MissingEnv(e)
    }
}

impl From<BsError> for Error {
    fn from(e: BsError) -> Self {
        Self::BuildScript(e)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { e, msg } => write!(f, "{msg} - {e}"),
            Self::MissingEnv(e) => e.fmt(f),
            Self::BuildScript(e) => e.fmt(f),
        }
    }
}
