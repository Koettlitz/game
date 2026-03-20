use std::env::VarError;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::{fs, io};

use build::AssetSource;
use build::asset_enum::{BsError, generate_enum};

const LOZO_EXTENSION: &str = "lozo.ron";
const LOZO_ENUM_NAME: &str = "Lozo";
const LOZO_ENUM_FILE_NAME: &str = "lozo.rs";

fn main() -> Result<(), Error> {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let asset_root = crate_root.join("assets");
    if !asset_root.exists() {
        if let Err(e) = fs::create_dir(&asset_root) {
            return Err(Error::Io {
                msg: format!(
                    "could not create missing asset root {}",
                    asset_root.display()
                ),
                e,
            });
        }
    }
    let workspace_root = crate_root
        .parent()
        .unwrap_or_else(|| panic!("game crate root has no parent directory?"));
    let workspace_asset_root = workspace_root.join("assets");
    println!("cargo:rerun-if-changed={}", workspace_asset_root.display());
    println!("cargo:rerun-if-changed={}", asset_root.display());

    let lozo_enum = generate_lozo_enum(&asset_root)?;
    write_file(LOZO_ENUM_FILE_NAME, &lozo_enum.to_string())
}

fn generate_lozo_enum(asset_root: &Path) -> Result<proc_macro2::TokenStream, BsError> {
    let lozo_folder = asset_root.join("lozo");
    generate_enum(
        asset_root,
        &lozo_folder,
        AssetSource::Game,
        Some(LOZO_ENUM_NAME),
        Some(LOZO_EXTENSION),
    )
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
