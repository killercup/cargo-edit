use crate::errors::*;
use cargo_metadata::Package;
use std::convert::TryInto;
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

/// Lookup all members of the current workspace
pub fn workspace_members(manifest_path: Option<&Path>) -> CargoResult<Vec<Package>> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.no_deps();
    if let Some(manifest_path) = manifest_path {
        cmd.manifest_path(manifest_path);
    }
    let result = cmd.exec().with_context(|| "Invalid manifest")?;
    let workspace_members: std::collections::HashSet<_> =
        result.workspace_members.into_iter().collect();
    let workspace_members: Vec<_> = result
        .packages
        .into_iter()
        .filter(|p| workspace_members.contains(&p.id))
        .map(|mut p| {
            p.manifest_path = canonicalize_path(p.manifest_path);
            for dep in p.dependencies.iter_mut() {
                dep.path = dep.path.take().map(canonicalize_path);
            }
            p
        })
        .collect();
    Ok(workspace_members)
}

fn canonicalize_path(
    path: cargo_metadata::camino::Utf8PathBuf,
) -> cargo_metadata::camino::Utf8PathBuf {
    if let Ok(path) = dunce::canonicalize(&path) {
        if let Ok(path) = path.try_into() {
            return path;
        }
    }

    path
}
