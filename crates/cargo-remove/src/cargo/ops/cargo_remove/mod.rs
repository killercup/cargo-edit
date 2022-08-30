//! Core of cargo-remove command

mod dependency;
mod manifest;
mod metadata;
mod util;

use cargo::core::Package;
use cargo::CargoResult;

pub use dependency::{Dependency, PathSource, RegistrySource, Source};
pub use manifest::{
    find, get_dep_version, set_dep_version, DepKind, DepTable, LocalManifest, Manifest,
};
pub use metadata::{manifest_from_pkgid, resolve_manifests, workspace_members};
pub use util::{
    colorize_stderr, shell_note, shell_print, shell_status, shell_warn, shell_write_stderr, Color,
    ColorChoice,
};

/// Remove a dependency from a Cargo.toml manifest file.
#[derive(Debug)]
pub struct RmOptions<'a> {
    /// Package to remove dependencies from
    pub spec: &'a Package,
    /// Dependencies to remove
    pub dependencies: Vec<&'a String>,
    /// Which dependency section to remove these from
    pub section: DepTable,
    /// Whether or not to actually write the manifest
    pub dry_run: bool,
    /// Do not print any output in case of success
    pub quiet: bool,
}

/// Remove dependencies from a manifest
pub fn remove(options: &RmOptions<'_>) -> CargoResult<()> {
    let dep_table = options
        .section
        .to_table()
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();

    let manifest_path = options.spec.manifest_path().to_path_buf();
    let mut manifest = LocalManifest::try_new(&manifest_path)?;

    options
        .dependencies
        .iter()
        .map(|dep| {
            if !options.quiet {
                let section = if dep_table.len() >= 3 {
                    format!("{} for target `{}`", &dep_table[2], &dep_table[1])
                } else {
                    dep_table[0].clone()
                };
                shell_status("Removing", &format!("{dep} from {section}",))?;
            }
            let result = manifest
                .remove_from_table(&dep_table, dep)
                .map_err(Into::into);

            // Now that we have removed the crate, if that was the last reference to that crate,
            // then we need to drop any explicitly activated features on that crate.
            manifest.gc_dep(dep);

            result
        })
        .collect::<CargoResult<Vec<_>>>()?;

    if options.dry_run {
        shell_warn("aborting remove due to dry run")?;
    } else {
        manifest.write()?;
    }

    Ok(())
}
