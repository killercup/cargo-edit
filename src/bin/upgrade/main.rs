//! `cargo upgrade`
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts,
        trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces,
        unused_qualifications)]

extern crate cargo_metadata;
extern crate docopt;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;
extern crate toml_edit;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process;

extern crate cargo_edit;
use cargo_edit::{find, get_latest_dependency, Dependency, LocalManifest};

mod errors {
    error_chain!{
        links {
            CargoEditLib(::cargo_edit::Error, ::cargo_edit::ErrorKind);
        }

        foreign_links {
            // cargo-metadata doesn't (yet) export `ErrorKind`
            Metadata(::cargo_metadata::Error);
        }
    }
}
use errors::*;

static USAGE: &'static str = r"
Upgrade all dependencies in a manifest file to the latest version.

Usage:
    cargo upgrade [options]
    cargo upgrade [options] <dependency>... [--precise <PRECISE>]
    cargo upgrade (-h | --help)
    cargo upgrade (-V | --version)

Options:
    --all                       Upgrade all packages in the workspace.
    --precise PRECISE           Upgrade dependencies to exactly PRECISE.
    --manifest-path PATH        Path to the manifest to upgrade.
    --allow-prerelease          Include prerelease versions when fetching from crates.io (e.g.
                                '0.6.0-alpha'). Defaults to false.
    -h --help                   Show this help page.
    -V --version                Show version.

Dev, build, and all target dependencies will also be upgraded. Only dependencies from crates.io are
supported. Git/path dependencies will be ignored.

All packages in the workspace will be upgraded if the `--all` flag is supplied. The `--all` flag may
be supplied in the presence of a virtual manifest.
";

/// Docopts input args.
#[derive(Debug, Deserialize)]
struct Args {
    /// `<dependency>...`
    arg_dependency: Vec<String>,
    /// `--precise PRECISE`
    flag_precise: Option<String>,
    /// `--manifest-path PATH`
    flag_manifest_path: Option<String>,
    /// `--version`
    flag_version: bool,
    /// `--all`
    flag_all: bool,
    /// `--allow-prerelease`
    flag_allow_prerelease: bool,
}

/// A collection of manifests.
struct Manifests(Vec<(LocalManifest, cargo_metadata::Package)>);

impl Manifests {
    /// Get all manifests in the workspace.
    fn get_all(manifest_path: &Option<String>) -> Result<Self> {
        let manifest_path = manifest_path.clone().map(PathBuf::from);

        cargo_metadata::metadata_deps(manifest_path.as_ref().map(Path::new), true)
            .chain_err(|| "Failed to get workspace metadata")?
            .packages
            .into_iter()
            .map(|package| {
                Ok((
                    LocalManifest::try_new(Path::new(&package.manifest_path))?,
                    package,
                ))
            })
            .collect::<Result<Vec<_>>>()
            .map(Manifests)
    }

    /// Get the manifest specified by the manifest path. Try to make an educated guess if no path is
    /// provided.
    fn get_local_one(manifest_path: &Option<String>) -> Result<Self> {
        let manifest_path = manifest_path.clone().map(PathBuf::from);
        let resolved_manifest_path: String = find(&manifest_path)?.to_string_lossy().into();

        let manifest = LocalManifest::find(&manifest_path)?;

        let packages = cargo_metadata::metadata_deps(manifest_path.as_ref().map(Path::new), true)
            .chain_err(|| "Invalid manifest")?
            .packages;
        let package = packages
            .iter()
            .find(|p| p.manifest_path == resolved_manifest_path)
            // If we have successfully got metadata, but our manifest path does not correspond to a
            // package, we must have been called against a virtual manifest.
            .chain_err(|| "Found virtual manifest, but this command requires running against an \
                           actual package in this workspace. Try adding `--all`.")?;

        Ok(Manifests(vec![(manifest, package.to_owned())]))
    }

    /// Get the combined set of dependencies of the manifests.
    fn get_dependencies(&self, only_update: &[String]) -> Dependencies {
        /// Helper function to check whether a `cargo_metadata::Dependency` is a version dependency.
        fn is_version_dep(dependency: &cargo_metadata::Dependency) -> bool {
            match dependency.source {
                // This is the criterion cargo uses (in `SourceId::from_url`) to decide whether a
                // dependency has the 'registry' kind.
                Some(ref s) => s.splitn(2, '+').next() == Some("registry"),
                _ => false,
            }
        }

        Dependencies(
            self.0
                .iter()
                .flat_map(|&(_, ref package)| package.dependencies.clone())
                .filter(|dependency| {
                    only_update.is_empty() || only_update.contains(&dependency.name)
                })
                .filter(is_version_dep)
                .map(|dependency| {
                    // Convert manually from one dependency format to another. Ideally, this would
                    // be done by implementing `From`. However, that would require pulling in
                    // `cargo::SourceId::from_url`, which would entail pulling the entirety of
                    // cargo.
                    Dependency::new(&dependency.name)
                        .set_optional(dependency.optional)
                        .set_version(&dependency.req.to_string())
                })
                .collect(),
        )
    }

    ///  Upgrade the manifests on disk. They will upgrade using the new dependencies provided.
    fn upgrade(self, upgraded_deps: &Dependencies) -> Result<()> {
        for (mut manifest, _) in self.0 {
            for dependency in &upgraded_deps.0 {
                manifest.upgrade(dependency)?;
            }
        }

        Ok(())
    }
}

/// This represents the version dependencies of the manifests that `cargo-upgrade` will upgrade.
struct Dependencies(HashSet<Dependency>);

impl Dependencies {
    /// Transform the dependencies into their upgraded forms. If a version is specified, all
    /// dependencies will get that version.
    fn get_upgraded(
        self,
        precise: &Option<String>,
        allow_prerelease: bool,
    ) -> Result<Dependencies> {
        self.0
            .into_iter()
            .map(|dependency| {
                if let Some(ref precise) = *precise {
                    Ok(dependency.set_version(precise))
                } else {
                    get_latest_dependency(&dependency.name, allow_prerelease)
                        .chain_err(|| "Failed to get new version")
                }
            })
            .collect::<Result<_>>()
            .map(Dependencies)
    }
}

/// Main processing function. Allows us to return a `Result` so that `main` can print pretty error
/// messages.
fn process(args: &Args) -> Result<()> {
    let manifests = if args.flag_all {
        Manifests::get_all(&args.flag_manifest_path)
    } else {
        Manifests::get_local_one(&args.flag_manifest_path)
    }?;

    let existing_dependencies = manifests.get_dependencies(&args.arg_dependency.clone());

    let upgraded_dependencies =
        existing_dependencies.get_upgraded(&args.flag_precise, args.flag_allow_prerelease)?;

    manifests.upgrade(&upgraded_dependencies)
}

fn main() {
    let args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.deserialize::<Args>())
        .unwrap_or_else(|err| err.exit());

    if args.flag_version {
        println!("cargo-upgrade version {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    if let Err(err) = process(&args) {
        eprintln!("Command failed due to unhandled error: {}\n", err);

        for e in err.iter().skip(1) {
            eprintln!("Caused by: {}", e);
        }

        if let Some(backtrace) = err.backtrace() {
            eprintln!("Backtrace: {:?}", backtrace);
        }

        process::exit(1);
    }
}
