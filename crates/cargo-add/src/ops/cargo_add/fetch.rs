use std::path::Path;

use anyhow::Context;
use cargo::CargoResult;

use super::LocalManifest;

/// Load Cargo.toml in a local path
///
/// This will fail, when Cargo.toml is not present in the root of the path.
pub fn get_manifest_from_path(path: &Path) -> CargoResult<LocalManifest> {
    let cargo_file = path.join("Cargo.toml");
    LocalManifest::try_new(&cargo_file).with_context(|| "Unable to open local Cargo.toml")
}
