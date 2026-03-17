use build::AssetSource;
use build::generate_path_consts;
use build::scan_asset_dir;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() -> Result<(), io::Error> {
    let editor_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let editor_asset_root = editor_root.join("assets");
    let workspace_root = editor_root
        .parent()
        .unwrap_or_else(|| panic!("editor crate root has no parent directory"));
    let general_asset_root = workspace_root.join("assets");
    println!("cargo:rerun-if-changed={}", general_asset_root.display());
    println!("cargo:rerun-if-changed={}", editor_asset_root.display());

    let mut editor_asset_paths = HashMap::new();
    scan_asset_dir(&editor_asset_root, &mut editor_asset_paths)?;
    let mut general_asset_paths = HashMap::new();
    scan_asset_dir(&general_asset_root, &mut general_asset_paths)?;

    let mut output = String::new();
    output.push_str(&generate_path_consts(
        AssetSource::Editor,
        &editor_asset_root,
        &editor_asset_paths,
    ));
    output.push_str(&generate_path_consts(
        AssetSource::Workspace,
        &general_asset_root,
        &general_asset_paths,
    ));
    let out_dir = std::env::var("OUT_DIR").expect("missing OUT_DIR environment variable");
    fs::write(Path::new(&out_dir).join("asset_registry.rs"), &output)
}
