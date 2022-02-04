//! `cargo upgrade`
#![warn(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

#[macro_use]
extern crate error_chain;

use crate::errors::*;
use cargo_edit::{
    colorize_stderr, find, get_latest_dependency, manifest_from_pkgid, registry_url,
    update_registry_index, CrateSpec, Dependency, LocalManifest,
};
use clap::Parser;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use url::Url;

mod errors {
    error_chain! {
        links {
            CargoEditLib(::cargo_edit::Error, ::cargo_edit::ErrorKind);
        }
        foreign_links {
            CargoMetadata(::cargo_metadata::Error)#[doc = "An error from the cargo_metadata crate"];
            Semver(::semver::Error)#[doc = "An error from the semver crate"];
        }
    }
}

fn main() {
    let args: Command = Command::parse();
    let Command::Upgrade(args) = args;

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

#[derive(Debug, Parser)]
#[clap(bin_name = "cargo")]
enum Command {
    /// Upgrade dependencies as specified in the local manifest file (i.e. Cargo.toml).
    #[clap(name = "upgrade")]
    #[clap(after_help = "\
This command differs from `cargo update`, which updates the dependency versions recorded in the \
local lock file (Cargo.lock).

If `<dependency>`(s) are provided, only the specified dependencies will be upgraded. The version \
to upgrade to for each can be specified with e.g. `docopt@0.8.0` or `serde@>=0.9,<2.0`.

Dev, build, and all target dependencies will also be upgraded. Only dependencies from crates.io \
are supported. Git/path dependencies will be ignored.

All packages in the workspace will be upgraded if the `--workspace` flag is supplied. The \
`--workspace` flag may be supplied in the presence of a virtual manifest.

If the '--to-lockfile' flag is supplied, all dependencies will be upgraded to the currently locked \
version as recorded in the Cargo.lock file. This flag requires that the Cargo.lock file is \
up-to-date. If the lock file is missing, or it needs to be updated, cargo-upgrade will exit with \
an error. If the '--to-lockfile' flag is supplied then the network won't be accessed.")]
    Upgrade(Args),
}

#[derive(Debug, Parser)]
#[clap(about, version)]
struct Args {
    /// Crates to be upgraded.
    dependency: Vec<String>,

    /// Path to the manifest to upgrade
    #[clap(
        long,
        value_name = "PATH",
        parse(from_os_str),
        conflicts_with = "pkgid"
    )]
    manifest_path: Option<PathBuf>,

    /// Package id of the crate to add this dependency to.
    #[clap(
        long = "package",
        short = 'p',
        value_name = "PKGID",
        conflicts_with = "manifest-path",
        conflicts_with = "all",
        conflicts_with = "workspace"
    )]
    pkgid: Option<String>,

    /// Upgrade all packages in the workspace.
    #[clap(
        long,
        help = "[deprecated in favor of `--workspace`]",
        conflicts_with = "workspace",
        conflicts_with = "pkgid"
    )]
    all: bool,

    /// Upgrade all packages in the workspace.
    #[clap(long, conflicts_with = "all", conflicts_with = "pkgid")]
    workspace: bool,

    /// Include prerelease versions when fetching from crates.io (e.g. 0.6.0-alpha').
    #[clap(long)]
    allow_prerelease: bool,

    /// Print changes to be made without making them.
    #[clap(long)]
    dry_run: bool,

    /// Only update a dependency if the new version is semver incompatible.
    #[clap(long, conflicts_with = "to-lockfile")]
    skip_compatible: bool,

    /// Run without accessing the network
    #[clap(long)]
    pub offline: bool,

    /// Upgrade all packages to the version in the lockfile.
    #[clap(long, conflicts_with = "dependency")]
    pub to_lockfile: bool,

    /// Crates to exclude and not upgrade.
    #[clap(long)]
    exclude: Vec<String>,

    /// Unstable (nightly-only) flags
    #[clap(short = 'Z', value_name = "FLAG", global = true, arg_enum)]
    pub unstable_features: Vec<UnstableOptions>,
}

impl Args {
    fn workspace(&self) -> bool {
        self.all || self.workspace
    }

    fn resolve_targets(&self) -> Result<Vec<(LocalManifest, cargo_metadata::Package)>> {
        if self.workspace() {
            resolve_all(self.manifest_path.as_deref())
        } else if let Some(pkgid) = self.pkgid.as_deref() {
            resolve_pkgid(self.manifest_path.as_deref(), pkgid)
        } else {
            resolve_local_one(self.manifest_path.as_deref())
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ArgEnum)]
enum UnstableOptions {}

#[test]
fn verify_app() {
    use clap::IntoApp;
    Command::into_app().debug_assert()
}

/// Main processing function. Allows us to return a `Result` so that `main` can print pretty error
/// messages.
fn process(args: Args) -> Result<()> {
    if args.all {
        deprecated_message("The flag `--all` has been deprecated in favor of `--workspace`")?;
    }

    if !args.offline && !args.to_lockfile && std::env::var("CARGO_IS_TEST").is_err() {
        let url = registry_url(&find(args.manifest_path.as_deref())?, None)?;
        update_registry_index(&url, false)?;
    }

    let manifests = args.resolve_targets()?;
    let locked = if std::env::var("CARGO_IS_TEST").is_err() {
        load_lockfile(&manifests)?
    } else {
        load_lockfile(&manifests).unwrap_or_default()
    };

    let mut updated_registries = BTreeSet::new();
    for (manifest, package) in manifests {
        let existing_dependencies = get_dependencies(&package, &args.dependency, &args.exclude)?;

        let upgraded_dependencies = if args.to_lockfile {
            existing_dependencies.into_lockfile(&locked)?
        } else {
            // Update indices for any alternative registries, unless
            // we're offline.
            if !args.offline && std::env::var("CARGO_IS_TEST").is_err() {
                for registry_url in existing_dependencies
                    .0
                    .values()
                    .filter_map(|UpgradeMetadata { registry, .. }| registry.as_ref())
                {
                    if updated_registries.insert(registry_url.to_owned()) {
                        update_registry_index(
                            &Url::parse(registry_url).map_err(|_| {
                                ErrorKind::CargoEditLib(::cargo_edit::ErrorKind::InvalidCargoConfig)
                            })?,
                            false,
                        )?;
                    }
                }
            }

            existing_dependencies
                .into_latest(args.allow_prerelease, &find(args.manifest_path.as_deref())?)?
        };

        upgrade(
            manifest,
            package,
            &upgraded_dependencies,
            args.dry_run,
            args.skip_compatible,
        )?;
    }

    if args.dry_run {
        dry_run_message()?;
    }

    Ok(())
}

/// Get the combined set of dependencies to upgrade. If the user has specified
/// per-dependency desired versions, extract those here.
fn get_dependencies(
    package: &cargo_metadata::Package,
    only_update: &[String],
    exclude: &[String],
) -> Result<DesiredUpgrades> {
    // Map the names of user-specified dependencies to the (optionally) requested version.
    let selected_dependencies = only_update
        .iter()
        .map(|name| match CrateSpec::resolve(name)? {
            CrateSpec::PkgId { name, version_req } => Ok((name, version_req)),
            CrateSpec::Path(path) => Err(format!("Invalid name: {}", path.display()).into()),
        })
        .collect::<Result<BTreeMap<_, _>>>()?;

    let mut upgrades = DesiredUpgrades::default();
    for dependency in package
        .dependencies
        .clone()
        .into_iter()
        .filter(is_version_dep)
        .filter(|dependency| !exclude.contains(&dependency.name))
        // Exclude renamed dependencies as well
        .filter(|dependency| {
            dependency
                .rename
                .as_ref()
                .map_or(true, |rename| !exclude.contains(rename))
        })
    {
        let is_prerelease = dependency.req.to_string().contains('-');
        if selected_dependencies.is_empty() {
            // User hasn't asked for any specific dependencies to be upgraded,
            // so upgrade all the dependencies.
            let mut dep = Dependency::new(&dependency.name);
            if let Some(rename) = dependency.rename {
                dep = dep.set_rename(&rename);
            }
            upgrades.0.insert(
                dep,
                UpgradeMetadata {
                    registry: dependency.registry,
                    version: None,
                    old_version: dependency.req.clone(),
                    is_prerelease,
                },
            );
        } else {
            // User has asked for specific dependencies. Check if this dependency
            // was specified, populating the registry from the lockfile metadata.
            if let Some(version) = selected_dependencies.get(&dependency.name) {
                upgrades.0.insert(
                    Dependency::new(&dependency.name),
                    UpgradeMetadata {
                        registry: dependency.registry,
                        version: version.clone(),
                        old_version: dependency.req.clone(),
                        is_prerelease,
                    },
                );
            }
        }
    }
    Ok(upgrades)
}

/// Upgrade the manifests on disk following the previously-determined upgrade schema.
fn upgrade(
    mut manifest: LocalManifest,
    package: cargo_metadata::Package,
    upgraded_deps: &ActualUpgrades,
    dry_run: bool,
    skip_compatible: bool,
) -> Result<()> {
    println!("{}:", package.name);

    for (dep, version) in &upgraded_deps.0 {
        let mut new_dep = Dependency::new(&dep.name).set_version(version);
        if let Some(rename) = dep.rename() {
            new_dep = new_dep.set_rename(rename);
        }
        manifest.upgrade(&new_dep, dry_run, skip_compatible)?;
    }

    Ok(())
}

fn load_lockfile(
    targets: &[(LocalManifest, cargo_metadata::Package)],
) -> Result<Vec<cargo_metadata::Package>> {
    // Get locked dependencies. For workspaces with multiple Cargo.toml
    // files, there is only a single lockfile, so it suffices to get
    // metadata for any one of Cargo.toml files.
    let (manifest, _package) = targets.get(0).ok_or(ErrorKind::CargoEditLib(
        ::cargo_edit::ErrorKind::InvalidCargoConfig,
    ))?;
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.manifest_path(manifest.path.clone());
    cmd.features(cargo_metadata::CargoOpt::AllFeatures);
    cmd.other_options(vec!["--locked".to_string()]);

    let result = cmd.exec().chain_err(|| "Invalid manifest")?;

    let locked = result
        .packages
        .into_iter()
        .filter(|p| p.source.is_some()) // Source is none for local packages
        .collect::<Vec<_>>();

    Ok(locked)
}

// Some metadata about the dependency
// we're trying to upgrade.
#[derive(Clone, Debug)]
struct UpgradeMetadata {
    registry: Option<String>,
    // `Some` if the user has specified an explicit
    // version to upgrade to.
    version: Option<String>,
    old_version: semver::VersionReq,
    is_prerelease: bool,
}

/// The set of dependencies to be upgraded, alongside the registries returned from cargo metadata, and
/// the desired versions, if specified by the user.
#[derive(Default, Clone, Debug)]
struct DesiredUpgrades(BTreeMap<Dependency, UpgradeMetadata>);

impl DesiredUpgrades {
    /// Transform the dependencies into their upgraded forms. If a version is specified, all
    /// dependencies will get that version.
    fn into_latest(self, allow_prerelease: bool, manifest_path: &Path) -> Result<ActualUpgrades> {
        let mut upgrades = ActualUpgrades::default();
        for (
            dep,
            UpgradeMetadata {
                registry,
                version,
                old_version: _,
                is_prerelease,
            },
        ) in self.0.into_iter()
        {
            if let Some(v) = version {
                upgrades.0.insert(dep, v);
                continue;
            }

            let registry_url = match registry {
                Some(x) => Some(Url::parse(&x).map_err(|_| {
                    ErrorKind::CargoEditLib(::cargo_edit::ErrorKind::InvalidCargoConfig)
                })?),
                None => None,
            };
            let allow_prerelease = allow_prerelease || is_prerelease;

            let latest = get_latest_dependency(
                &dep.name,
                allow_prerelease,
                manifest_path,
                registry_url.as_ref(),
            )
            .chain_err(|| "Failed to get new version")?;
            let version = latest
                .version()
                .expect("Invalid dependency type")
                .to_string();
            upgrades.0.insert(dep, version);
        }
        Ok(upgrades)
    }

    fn into_lockfile(self, locked: &[cargo_metadata::Package]) -> Result<ActualUpgrades> {
        let mut upgrades = ActualUpgrades::default();
        for (
            dep,
            UpgradeMetadata {
                registry: _,
                version,
                old_version,
                is_prerelease: _,
            },
        ) in self.0.into_iter()
        {
            if let Some(v) = version {
                upgrades.0.insert(dep, v);
                continue;
            }

            for p in locked {
                // The requested dependency may be present in the lock file with different versions,
                // but only one will be semver-compatible with the requested version.
                if dep.name == p.name && old_version.matches(&p.version) {
                    upgrades.0.insert(dep, p.version.to_string());
                    break;
                }
            }
        }
        Ok(upgrades)
    }
}

/// The complete specification of the upgrades that will be performed. Map of the dependency names
/// to the new versions.
#[derive(Default, Clone, Debug)]
struct ActualUpgrades(BTreeMap<Dependency, String>);

/// Get all manifests in the workspace.
fn resolve_all(
    manifest_path: Option<&Path>,
) -> Result<Vec<(LocalManifest, cargo_metadata::Package)>> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.no_deps();
    if let Some(path) = manifest_path {
        cmd.manifest_path(path);
    }
    let result = cmd
        .exec()
        .chain_err(|| "Failed to get workspace metadata")?;
    result
        .packages
        .into_iter()
        .map(|package| {
            Ok((
                LocalManifest::try_new(Path::new(&package.manifest_path))?,
                package,
            ))
        })
        .collect::<Result<Vec<_>>>()
}

fn resolve_pkgid(
    manifest_path: Option<&Path>,
    pkgid: &str,
) -> Result<Vec<(LocalManifest, cargo_metadata::Package)>> {
    let package = manifest_from_pkgid(manifest_path, pkgid)?;
    let manifest = LocalManifest::try_new(Path::new(&package.manifest_path))?;
    Ok(vec![(manifest, package)])
}

/// Get the manifest specified by the manifest path. Try to make an educated guess if no path is
/// provided.
fn resolve_local_one(
    manifest_path: Option<&Path>,
) -> Result<Vec<(LocalManifest, cargo_metadata::Package)>> {
    let resolved_manifest_path: String = find(manifest_path)?.to_string_lossy().into();

    let manifest = LocalManifest::find(manifest_path)?;

    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.no_deps();
    if let Some(path) = manifest_path {
        cmd.manifest_path(path);
    }
    let result = cmd.exec().chain_err(|| "Invalid manifest")?;
    let packages = result.packages;
    let package = packages
        .iter()
        .find(|p| p.manifest_path == resolved_manifest_path)
        // If we have successfully got metadata, but our manifest path does not correspond to a
        // package, we must have been called against a virtual manifest.
        .chain_err(|| {
            "Found virtual manifest, but this command requires running against an \
                 actual package in this workspace. Try adding `--workspace`."
        })?;

    Ok(vec![(manifest, package.to_owned())])
}

/// Helper function to check whether a `cargo_metadata::Dependency` is a version dependency.
fn is_version_dep(dependency: &cargo_metadata::Dependency) -> bool {
    match dependency.source {
        // This is the criterion cargo uses (in `SourceId::from_url`) to decide whether a
        // dependency has the 'registry' kind.
        Some(ref s) => s.split_once('+').map(|(x, _)| x) == Some("registry"),
        _ => false,
    }
}

fn deprecated_message(message: &str) -> Result<()> {
    let colorchoice = colorize_stderr();
    let mut output = StandardStream::stderr(colorchoice);
    output
        .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))
        .chain_err(|| "Failed to set output colour")?;
    writeln!(output, "{}", message).chain_err(|| "Failed to write deprecated message")?;
    output
        .set_color(&ColorSpec::new())
        .chain_err(|| "Failed to clear output colour")?;
    Ok(())
}

fn dry_run_message() -> Result<()> {
    let colorchoice = colorize_stderr();
    let mut output = StandardStream::stderr(colorchoice);
    output
        .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))
        .chain_err(|| "Failed to set output colour")?;
    write!(output, "warning").chain_err(|| "Failed to write dry run message")?;
    output
        .set_color(&ColorSpec::new())
        .chain_err(|| "Failed to clear output colour")?;
    writeln!(output, ": aborting upgrade due to dry run")
        .chain_err(|| "Failed to write dry run message")?;
    Ok(())
}
