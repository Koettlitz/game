use std::fs;

use bevy::{
    asset::{AssetPath, io::file::FileAssetReader},
    prelude::*,
    tasks::IoTaskPool,
};
use bevy_elf::AssetResolver;
use ron::ser::PrettyConfig;
use serde::Serialize;

pub fn resolve_and_flush<A, I, R>(assets: I, resolver: R)
where
    A: Serialize + Send + 'static,
    I: IntoIterator<Item = (String, A)> + Send + Sync + 'static,
    I::IntoIter: Send + 'static,
    R: AssetResolver + Send + Sync + 'static,
{
    flush_assets(
        assets
            .into_iter()
            .map(|(id, asset)| {
                (
                    resolver
                        .resolve(&id)
                        .unwrap_or_else(|e| panic!("failed to resolve asset path: {e}")),
                    asset,
                )
            })
            .collect::<Vec<_>>(),
    );
}

pub fn flush_assets<A: Serialize>(
    assets: impl IntoIterator<Item = (AssetPath<'static>, A)> + Send + 'static,
) {
    IoTaskPool::get()
        .spawn(async move {
            for (asset_path, asset) in assets {
                write_asset(asset_path, asset).expect("failed to save asset");
            }
        })
        .detach();
}

pub fn write_asset<A: Serialize>(asset_path: AssetPath, asset: A) -> Result {
    let file_path = {
        let base_path = FileAssetReader::get_base_path();
        let sub_path = asset_path.path();

        if let Some(source) = asset_path.source().as_str() {
            base_path.join(source).join("assets").join(sub_path)
        } else {
            base_path.join("assets").join(sub_path)
        }
    };

    if let Some(dir_path) = file_path.parent()
        && !dir_path.exists()
    {
        info!("ensuring parent dirs exist for {}", dir_path.display());
        fs::create_dir_all(dir_path)?;
    }

    info!(
        "writing asset to {asset_path} => \"{}\"",
        file_path.display()
    );
    let serialized = ron::ser::to_string_pretty(&asset, PrettyConfig::default())?;
    fs::write(file_path, serialized)?;

    Ok(())
}
