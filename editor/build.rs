use build::AssetSource;
use build::asset_enum::BsError;
use build::asset_enum::generate_resolver_enums;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), BsError> {
    let editor_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let editor_asset_root = editor_root.join("assets");
    println!("cargo:rerun-if-changed={}", editor_asset_root.display());
    let resolver_enums = generate_resolver_enums(AssetSource::Editor, &editor_asset_root)?;
    for (path, resolver_enum) in resolver_enums {
        write_out(&path, &resolver_enum.to_string())?;
    }
    Ok(())
}

fn write_out(path: &Path, content: &impl AsRef<[u8]>) -> Result<(), BsError> {
    let out_dir = std::env::var("OUT_DIR")?;
    fs::write(Path::new(&out_dir).join(path), content)?;
    Ok(())
}
