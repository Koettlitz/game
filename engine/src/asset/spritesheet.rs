use std::borrow::Cow;

use bevy::prelude::*;
use bevy_elf::{AssetPathSpecProvider, FromDef, ResolveStrategy};
use serde::{Deserialize, Serialize};

#[derive(FromDef, Clone)]
#[elf(def_type(()))]
pub struct Spritesheet {
    #[elf(implicit, with_spec(
        sub_path = "images",
        resolve_strategy = ResolveStrategy::FileStemIsId { extension: "png" }
    ))]
    pub image: Handle<Image>,

    #[elf(implicit, with_spec(
        sub_path = "layouts",
        resolve_strategy = ResolveStrategy::FileStemIsId { extension: "tl.ron" }
    ))]
    pub layout: Handle<TextureAtlasLayout>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum SpritesheetKind {
    Tile,
    Object,
}

impl AssetPathSpecProvider for SpritesheetKind {
    fn base_path(&self) -> Cow<'static, str> {
        match self {
            Self::Tile => Cow::Borrowed("tiles/spritesheets"),
            Self::Object => Cow::Borrowed("objects/spritesheets"),
        }
    }

    fn resolve_strategy(&self) -> ResolveStrategy {
        ResolveStrategy::FileStemIsId { extension: "png" }
    }
}
