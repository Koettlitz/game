use build::AssetSource;
use build::asset_enum::BsError;
use build::asset_enum::generate_resolver_enums;
use build::write_out;
use std::io;
use std::path::PathBuf;

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
