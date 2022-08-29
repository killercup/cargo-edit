//! Core of cargo-rm command

mod dependency;
mod manifest;
mod metadata;
mod util;

pub use dependency::Dependency;
pub use dependency::PathSource;
pub use dependency::RegistrySource;
pub use dependency::Source;
pub use manifest::{find, get_dep_version, set_dep_version, LocalManifest, Manifest};
pub use metadata::{manifest_from_pkgid, resolve_manifests, workspace_members};
pub use util::{
    colorize_stderr, shell_note, shell_print, shell_status, shell_warn, shell_write_stderr, Color,
    ColorChoice,
};

use cargo::CargoResult;

use std::borrow::Cow;
use std::path::PathBuf;

/// Remove a dependency from a Cargo.toml manifest file.
#[derive(Debug)]
pub struct RmOptions {
    /// Dependencies to be removed
    pub crates: Vec<String>,
    /// Remove as development dependency
    pub dev: bool,
    /// Remove as build dependency
    pub build: bool,
    /// Remove as dependency from the given target platform
    pub target: Option<String>,
    /// Path to the manifest to remove a dependency from
    pub manifest_path: Option<PathBuf>,
    /// Package to remove from
    pub pkg_id: Option<String>,
    /// Don't actually write the manifest
    pub dry_run: bool,
    /// Do not print any output in case of success
    pub quiet: bool,
}

impl RmOptions {
    /// Get dependency section
    pub fn get_section(&self) -> Vec<String> {
        let section_name = if self.dev {
            "dev-dependencies"
        } else if self.build {
            "build-dependencies"
        } else {
            "dependencies"
        };

        if let Some(ref target) = self.target {
            assert!(!target.is_empty(), "Target specification may not be empty");

            vec!["target".to_owned(), target.clone(), section_name.to_owned()]
        } else {
            vec![section_name.to_owned()]
        }
    }
}

/// Remove dependencies from a manifest
pub fn rm(args: &RmOptions) -> CargoResult<()> {
    let manifest_path = if let Some(ref pkg_id) = args.pkg_id {
        let pkg = manifest_from_pkgid(args.manifest_path.as_deref(), pkg_id)?;
        Cow::Owned(Some(pkg.manifest_path.into_std_path_buf()))
    } else {
        Cow::Borrowed(&args.manifest_path)
    };
    let mut manifest = LocalManifest::find(manifest_path.as_deref())?;
    let deps = &args.crates;

    deps.iter()
        .map(|dep| {
            if !args.quiet {
                let section = args.get_section();
                let section = if section.len() >= 3 {
                    format!("{} for target `{}`", &section[2], &section[1])
                } else {
                    section[0].clone()
                };
                shell_status("Removing", &format!("{dep} from {section}",))?;
            }
            let result = manifest
                .remove_from_table(&args.get_section(), dep)
                .map_err(Into::into);

            // Now that we have removed the crate, if that was the last reference to that crate,
            // then we need to drop any explicitly activated features on that crate.
            manifest.gc_dep(dep);

            result
        })
        .collect::<CargoResult<Vec<_>>>()?;

    if args.dry_run {
        shell_warn("aborting rm due to dry run")?;
    } else {
        manifest.write()?;
    }

    Ok(())
}
