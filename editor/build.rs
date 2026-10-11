use build::AssetSource;
use build::asset_enum::BsError;
use build::asset_enum::generate_resolver_enums;
use build::write_out;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprint!("build.rs script failed: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), BsError> {
    let editor_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let editor_asset_root = editor_root.join("assets");
    println!("cargo:rerun-if-changed={}", editor_asset_root.display());
    let resolver_enums = generate_resolver_enums(AssetSource::Editor, &editor_asset_root)?;
    for (path, resolver_enum) in resolver_enums {
        println!("Writing {} enum:\n{resolver_enum}", path.display());
        write_out(&path, &resolver_enum.to_string())?;
    }
    Ok(())
}
