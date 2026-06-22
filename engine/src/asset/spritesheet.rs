use std::borrow::Cow;

use bevy::prelude::*;
use bevy_elf::{AssetPathSpecProvider, FromDef};
use serde::{Deserialize, Serialize};

#[derive(FromDef, Clone)]
#[elf(def_type(()))]
pub struct Spritesheet {
    #[elf(implicit, with_spec(sub_path = "images", extension = "png"))]
    pub image: Handle<Image>,

    #[elf(implicit, with_spec(sub_path = "layouts", extension = "tl.ron"))]
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

    fn extension(&self) -> Option<&'static str> {
        Some("png")
    }
}
