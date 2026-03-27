use build::asset_set::{generate_path_consts, scan_asset_dir};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

fn main() -> Result<(), io::Error> {
    let engine_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = engine_root
        .parent()
        .unwrap_or_else(|| panic!("engine crate root has no parent directory"));
    let asset_root = workspace_root.join("assets");
    println!("cargo:rerun-if-changed={}", asset_root.display());

    let mut asset_paths = HashMap::new();
    scan_asset_dir(&asset_root, &mut asset_paths)?;

    let output = generate_path_consts(&asset_root, &asset_paths);
    let out_dir = std::env::var("OUT_DIR").expect("missing OUT_DIR environment variable");
    fs::write(Path::new(&out_dir).join("asset_registry.rs"), &output)
}
