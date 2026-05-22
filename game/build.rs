use build::AssetSource;
use build::asset_enum::BsError;
use build::asset_enum::generate_resolver_enums;
use build::write_out;
use std::path::PathBuf;

fn main() -> Result<(), BsError> {
    let game_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let game_asset_root = game_root.join("assets");
    println!("cargo:rerun-if-changed={}", game_asset_root.display());
    let resolver_enums = generate_resolver_enums(AssetSource::Game, &game_asset_root)?;
    for (path, resolver_enum) in resolver_enums {
        write_out(&path, &resolver_enum.to_string())?;
    }
    Ok(())
}
