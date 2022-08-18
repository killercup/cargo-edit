//! Core of cargo-rm command

mod crate_spec;
mod dependency;
mod manifest;

use cargo::{core::Package, CargoResult, Config};
use dependency::Dependency;
use dependency::RegistrySource;

pub use self::manifest::DepTable;

use self::manifest::LocalManifest;

/// Information on what dependencies should be removed
#[derive(Clone, Debug)]
pub struct RmOptions<'a> {
    /// Configuration information for Cargo operations
    pub config: &'a Config,
    /// Package to remove dependencies from
    pub spec: &'a Package,
    /// Dependencies to remove
    pub dependencies: Vec<&'a String>,
    /// Which dependency section to remove these from
    pub section: DepTable,
    /// Whether or not to actually write the manifest
    pub dry_run: bool,
}

/// Remove dependencies from a manifest
pub fn rm(options: &RmOptions<'_>) -> CargoResult<()> {
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
            let table = &options.section.to_table();
            // TODO fix
            let section_name = if table.len() >= 3 {
                format!("{} for target `{}`", &table[2], &table[1])
            } else {
                table[0].to_owned()
            };
            options
                .config
                .shell()
                .status("Removing", format!("{dep} from {section_name}"))?;

            let result = manifest
                .remove_from_table(&dep_table, dep)
                .map_err(Into::into);

            // Now that we have removed the crate, if that was the last
            // reference to that crate, then we need to drop any explicitly
            // activated features on that crate.
            manifest.gc_dep(dep);

            result
        })
        .collect::<CargoResult<Vec<_>>>()?;

    if options.dry_run {
        options.config.shell().warn("aborting rm due to dry run")?;
    } else {
        manifest.write()?;
    }

    Ok(())
}
