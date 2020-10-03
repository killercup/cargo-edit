use crate::errors::*;
use cargo_metadata::Package;
use failure::Fail;

/// Takes a pkgid and attempts to find the path to it's `Cargo.toml`, using `cargo`'s metadata
pub fn manifest_from_pkgid(pkgid: &str) -> Result<Package> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.no_deps();
    let result = cmd
        .exec()
        .map_err(|e| Error::from(e.compat()).chain_err(|| "Invalid manifest"))?;
    let packages = result.packages;
    let package = packages
        .into_iter()
        .find(|pkg| pkg.name == pkgid)
        .chain_err(|| {
            "Found virtual manifest, but this command requires running against an \
             actual package in this workspace. Try adding `--workspace`."
        })?;
    Ok(package)
}
