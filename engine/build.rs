use build::AssetSource;
use build::asset_enum::BsError;
use build::asset_enum::generate_resolver_enums;
use std::fs;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

fn main() -> Result<(), BsError> {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "crate root had now parent"))?;
    let asset_root = workspace_root.join("assets");
    println!("cargo:rerun-if-changed={}", asset_root.display());
    let resolver_enums = generate_resolver_enums(AssetSource::Workspace, &asset_root)?;
    for (path, resolver_enum) in resolver_enums {
        write_out(&path, &resolver_enum.to_string())?;
    }
    Ok(())
}

fn write_out(path: &Path, content: &impl AsRef<[u8]>) -> Result<(), BsError> {
    let out_dir = std::env::var("OUT_DIR")?;
    let path = Path::new(&out_dir).join(path);
    fs::create_dir_all(
        &path
            .parent()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "path {path:?} had no parent"))?,
    )?;
    fs::write(path, content)?;
    Ok(())
}
