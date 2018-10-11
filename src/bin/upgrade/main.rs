//! `cargo upgrade`
#![warn(
    missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts,
    trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces,
    unused_qualifications
)]

extern crate cargo;
extern crate cargo_metadata;
extern crate docopt;
#[macro_use]
extern crate error_chain;
extern crate git2;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tempdir;
extern crate toml_edit;

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

extern crate cargo_edit;
use cargo_edit::{find, get_latest_dependency, CrateName, Dependency, LocalManifest};

extern crate termcolor;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

mod errors {
    error_chain!{
        links {
            CargoEditLib(::cargo_edit::Error, ::cargo_edit::ErrorKind);
            CargoMetadata(::cargo_metadata::Error, ::cargo_metadata::ErrorKind);
        }
    }
}
use errors::*;

static USAGE: &'static str = r"
Upgrade dependencies as specified in the local manifest file (i.e. Cargo.toml).

Usage:
    cargo upgrade [options] [<dependency>]...
    cargo upgrade (-h | --help)
    cargo upgrade (-V | --version)

Options:
    --all                   Upgrade all packages in the workspace.
    --manifest-path PATH    Path to the manifest to upgrade.
    --allow-prerelease      Include prerelease versions when fetching from crates.io (e.g.
                            '0.6.0-alpha'). Defaults to false.
    --dry-run               Print changes to be made without making them. Defaults to false.
    -h --help               Show this help page.
    -V --version            Show version.

This command differs from `cargo update`, which updates the dependency versions recorded in the
local lock file (Cargo.lock).

If `<dependency>`(s) are provided, only the specified dependencies will be upgraded. The version to
upgrade to for each can be specified with e.g. `docopt@0.8.0` or `serde@>=0.9,<2.0`.

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
    /// `--manifest-path PATH`
    flag_manifest_path: Option<String>,
    /// `--all`
    flag_all: bool,
    /// `--allow-prerelease`
    flag_allow_prerelease: bool,
    /// `--dry-run`
    flag_dry_run: bool,
    /// `--version`
    flag_version: bool,
}

/// A collection of manifests.
struct Manifests(Vec<(LocalManifest, cargo_metadata::Package)>);

impl Manifests {
    /// Get all manifests in the workspace.
    fn get_all(manifest_path: &Option<String>) -> Result<Self> {
        let manifest_path = manifest_path.clone().map(PathBuf::from);

        cargo_metadata::metadata(manifest_path.as_ref().map(Path::new))
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

        let packages = cargo_metadata::metadata(manifest_path.as_ref().map(Path::new))
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

    /// Get the the combined set of dependencies to upgrade. If the user has specified
    /// per-dependency desired versions, extract those here.
    fn get_dependencies(&self, only_update: Vec<String>) -> Result<DesiredUpgrades> {
        /// Helper function to check whether a `cargo_metadata::Dependency` is a version dependency.
        fn is_version_dep(dependency: &cargo_metadata::Dependency) -> bool {
            match dependency.source {
                // This is the criterion cargo uses (in `SourceId::from_url`) to decide whether a
                // dependency has the 'registry' kind.
                Some(ref s) => s.splitn(2, '+').next() == Some("registry"),
                _ => false,
            }
        }

        Ok(DesiredUpgrades(if only_update.is_empty() {
            // User hasn't asked for any specific dependencies to be upgraded, so upgrade all the
            // dependencies.
            self.0
                .iter()
                .flat_map(|&(_, ref package)| package.dependencies.clone())
                .filter(is_version_dep)
                .map(|dependency| (dependency.name, None))
                .collect()
        } else {
            only_update
                .into_iter()
                .map(|name| {
                    if let Some(dependency) = CrateName::new(&name.clone()).parse_as_version()? {
                        Ok((
                            dependency.name.clone(),
                            dependency.version().map(String::from),
                        ))
                    } else {
                        Ok((name, None))
                    }
                })
                .collect::<Result<_>>()?
        }))
    }

    /// Upgrade the manifests on disk following the previously-determined upgrade schema.
    fn upgrade(self, upgraded_deps: &ActualUpgrades, dry_run: bool) -> Result<()> {
        if dry_run {
            let bufwtr = BufferWriter::stdout(ColorChoice::Always);
            let mut buffer = bufwtr.buffer();
            buffer
                .set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))
                .chain_err(|| "Failed to set output colour")?;
            write!(&mut buffer, "Starting dry run. ")
                .chain_err(|| "Failed to write dry run message")?;
            buffer
                .set_color(&ColorSpec::new())
                .chain_err(|| "Failed to clear output colour")?;
            writeln!(&mut buffer, "Changes will not be saved.")
                .chain_err(|| "Failed to write dry run message")?;
            bufwtr
                .print(&buffer)
                .chain_err(|| "Failed to print dry run message")?;
        }

        for (mut manifest, package) in self.0 {
            println!("{}:", package.name);

            for (name, version) in &upgraded_deps.0 {
                manifest.upgrade(&Dependency::new(name).set_version(version), dry_run)?;
            }
        }

        Ok(())
    }
}

/// The set of dependencies to be upgraded, alongside desired versions, if specified by the user.
struct DesiredUpgrades(HashMap<String, Option<String>>);

/// The complete specification of the upgrades that will be performed. Map of the dependency names
/// to the new versions.
struct ActualUpgrades(HashMap<String, String>);

impl DesiredUpgrades {
    /// Transform the dependencies into their upgraded forms. If a version is specified, all
    /// dependencies will get that version.
    fn get_upgraded(self, allow_prerelease: bool) -> Result<ActualUpgrades> {
        self.0
            .into_iter()
            .map(|(name, version)| {
                if let Some(v) = version {
                    Ok((name, v))
                } else {
                    get_latest_dependency(&name, allow_prerelease, None)
                        .map(|new_dep| {
                            (
                                name,
                                new_dep
                                    .version()
                                    .expect("Invalid dependency type")
                                    .to_string(),
                            )
                        })
                        .chain_err(|| "Failed to get new version")
                }
            })
            .collect::<Result<_>>()
            .map(ActualUpgrades)
    }
}

/// Main processing function. Allows us to return a `Result` so that `main` can print pretty error
/// messages.
fn process(args: Args) -> Result<()> {
    let Args {
        arg_dependency,
        flag_manifest_path,
        flag_all,
        flag_allow_prerelease,
        flag_dry_run,
        ..
    } = args;

    let manifests = if flag_all {
        Manifests::get_all(&flag_manifest_path)
    } else {
        Manifests::get_local_one(&flag_manifest_path)
    }?;

    let existing_dependencies = manifests.get_dependencies(arg_dependency)?;

    let upgraded_dependencies = existing_dependencies.get_upgraded(flag_allow_prerelease)?;

    manifests.upgrade(&upgraded_dependencies, flag_dry_run)
}

fn main() {
    let args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.deserialize::<Args>())
        .unwrap_or_else(|err| err.exit());

    if args.flag_version {
        println!("cargo-upgrade version {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    if let Err(err) = process(args) {
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
