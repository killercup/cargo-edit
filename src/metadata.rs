use cargo_metadata::Package;
use anyhow::{anyhow, Result};

/// Takes a pkgid and attempts to find the path to it's `Cargo.toml`, using `cargo`'s metadata
pub fn manifest_from_pkgid(pkgid: &str) -> Result<Package> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.no_deps();
    let result = cmd.exec()?;
    let packages = result.packages;
    let package = packages
        .into_iter()
        .find(|pkg| &pkg.name == pkgid)
        .ok_or_else(|| anyhow!(
            "Found virtual manifest, but this command requires running against an \
             actual package in this workspace. Try adding `--all`."
        ))?;
    Ok(package)
}
