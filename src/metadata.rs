use super::errors::*;
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
    let workspace_members: std::collections::BTreeSet<_> =
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

/// Determine packages selected by user
pub fn resolve_manifests(
    manifest_path: Option<&Path>,
    workspace: bool,
    pkgid: Option<&str>,
) -> CargoResult<Vec<Package>> {
    let manifest_path = manifest_path.map(|p| Ok(p.to_owned())).unwrap_or_else(|| {
        find_manifest_path(
            &std::env::current_dir().with_context(|| "Failed to get current directory")?,
        )
    })?;
    let manifest_path = dunce::canonicalize(manifest_path)?;

    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.no_deps();
    cmd.manifest_path(&manifest_path);
    let result = cmd.exec().with_context(|| "Invalid manifest")?;
    let pkgs = if workspace {
        result
            .packages
            .into_iter()
            .map(|package| Ok(package))
            .collect::<CargoResult<Vec<_>>>()?
    } else if let Some(pkgid) = pkgid {
        let package = result
            .packages
            .into_iter()
            .find(|pkg| pkg.name == pkgid)
            .with_context(|| {
                "Found virtual manifest, but this command requires running against an \
                 actual package in this workspace. Try adding `--workspace`."
            })?;
        vec![package]
    } else {
        let package = result
            .packages
            .iter()
            .find(|p| p.manifest_path == manifest_path)
            // If we have successfully got metadata, but our manifest path does not correspond to a
            // package, we must have been called against a virtual manifest.
            .with_context(|| {
                "Found virtual manifest, but this command requires running against an \
                 actual package in this workspace. Try adding `--workspace`."
            })?;

        vec![(package.to_owned())]
    };
    Ok(pkgs)
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
