use super::errors::*;
use cargo_metadata::Package;
use std::path::Path;

/// Takes a pkgid and attempts to find the path to it's `Cargo.toml`, using `cargo`'s metadata
pub fn manifest_from_pkgid(manifest_path: Option<&Path>, pkgid: &str) -> CargoResult<Package> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.no_deps();
    if let Some(manifest_path) = manifest_path {
        cmd.manifest_path(manifest_path);
    }
    let result = cmd.exec().with_context(|| "Invalid manifest")?;
    let packages = result.packages;
    let package = packages
        .into_iter()
        .find(|pkg| pkg.name == pkgid)
        .with_context(|| {
            "Found virtual manifest, but this command requires running against an \
             actual package in this workspace. Try adding `--workspace`."
        })?;
    Ok(package)
}

/// Search for Cargo.toml in this directory and recursively up the tree until one is found.
pub(crate) fn find_manifest_path(dir: &Path) -> CargoResult<std::path::PathBuf> {
    const MANIFEST_FILENAME: &str = "Cargo.toml";
    for path in dir.ancestors() {
        let manifest = path.join(MANIFEST_FILENAME);
        if std::fs::metadata(&manifest).is_ok() {
            return Ok(manifest);
        }
    }
    anyhow::bail!("Unable to find Cargo.toml for {}", dir.display());
}
