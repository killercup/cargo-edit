use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use cargo_edit::{
    colorize_stderr, find, get_latest_dependency, manifest_from_pkgid, registry_url,
    update_registry_index, CargoResult, Context, CrateSpec, Dependency, LocalManifest,
};
use clap::Args;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use url::Url;

/// Upgrade dependencies as specified in the local manifest file (i.e. Cargo.toml).
#[derive(Debug, Args)]
#[clap(version)]
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
pub struct UpgradeArgs {
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
    offline: bool,

    /// Upgrade all packages to the version in the lockfile.
    #[clap(long, conflicts_with = "dependency")]
    to_lockfile: bool,

    /// Crates to exclude and not upgrade.
    #[clap(long)]
    exclude: Vec<String>,

    /// Unstable (nightly-only) flags
    #[clap(short = 'Z', value_name = "FLAG", global = true, arg_enum)]
    unstable_features: Vec<UnstableOptions>,
}

impl UpgradeArgs {
    pub fn exec(self) -> CargoResult<()> {
        exec(self)
    }

    fn workspace(&self) -> bool {
        self.all || self.workspace
    }

    fn resolve_targets(&self) -> CargoResult<Vec<(LocalManifest, cargo_metadata::Package)>> {
        if self.workspace() {
            resolve_all(self.manifest_path.as_deref())
        } else if let Some(pkgid) = self.pkgid.as_deref() {
            resolve_pkgid(self.manifest_path.as_deref(), pkgid)
        } else {
            resolve_local_one(self.manifest_path.as_deref())
        }
    }

    fn preserve_precision(&self) -> bool {
        self.unstable_features
            .contains(&UnstableOptions::PreservePrecision)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ArgEnum)]
enum UnstableOptions {
    PreservePrecision,
}

/// Main processing function. Allows us to return a `Result` so that `main` can print pretty error
/// messages.
fn exec(args: UpgradeArgs) -> CargoResult<()> {
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
    let preserve_precision = args.preserve_precision();

    let mut updated_registries = BTreeSet::new();
    for (manifest, package) in manifests {
        let existing_dependencies = get_dependencies(&manifest, &args.dependency, &args.exclude)?;

        let upgraded_dependencies = if args.to_lockfile {
            existing_dependencies.into_lockfile(&locked, preserve_precision)?
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
                        update_registry_index(registry_url, false)?;
                    }
                }
            }

            existing_dependencies.into_latest(
                args.allow_prerelease,
                &find(args.manifest_path.as_deref())?,
                preserve_precision,
            )?
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
    manifest: &LocalManifest,
    only_update: &[String],
    exclude: &[String],
) -> CargoResult<DesiredUpgrades> {
    // Map the names of user-specified dependencies to the (optionally) requested version.
    let selected_dependencies = only_update
        .iter()
        .map(|name| match CrateSpec::resolve(name)? {
            CrateSpec::PkgId { name, version_req } => Ok((name, version_req)),
            CrateSpec::Path(path) => Err(anyhow::format_err!("Invalid name: {}", path.display())),
        })
        .collect::<CargoResult<BTreeMap<_, _>>>()?;

    let mut upgrades = DesiredUpgrades::default();
    for (dependency, old_version) in manifest
        .get_dependencies()
        .map(|(_, result)| result)
        .collect::<CargoResult<Vec<_>>>()?
        .into_iter()
        .filter(|dependency| dependency.path().is_none())
        .filter_map(|dependency| {
            dependency
                .version()
                .map(ToOwned::to_owned)
                .map(|version| (dependency, version))
        })
        .filter(|(dependency, _)| !exclude.contains(&dependency.name))
        // Exclude renamed dependencies as well
        .filter(|(dependency, _)| {
            dependency
                .rename()
                .map_or(true, |rename| !exclude.iter().any(|s| s == rename))
        })
    {
        let registry = dependency
            .registry()
            .map(|registry| registry_url(&manifest.path, Some(registry)))
            .transpose()?;
        let is_prerelease = dependency
            .version()
            .map_or(false, |version| version.contains('-'));
        if selected_dependencies.is_empty() {
            upgrades.0.insert(
                dependency,
                UpgradeMetadata {
                    registry,
                    version: None,
                    old_version,
                    is_prerelease,
                },
            );
        } else {
            // User has asked for specific dependencies. Check if this dependency
            // was specified, populating the registry from the lockfile metadata.
            if let Some(version) = selected_dependencies.get(&dependency.name) {
                upgrades.0.insert(
                    dependency,
                    UpgradeMetadata {
                        registry,
                        version: version.clone(),
                        old_version,
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
) -> CargoResult<()> {
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
) -> CargoResult<Vec<cargo_metadata::Package>> {
    // Get locked dependencies. For workspaces with multiple Cargo.toml
    // files, there is only a single lockfile, so it suffices to get
    // metadata for any one of Cargo.toml files.
    let (manifest, _package) = targets
        .get(0)
        .ok_or(anyhow::format_err!("Invalid cargo config"))?;
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.manifest_path(manifest.path.clone());
    cmd.features(cargo_metadata::CargoOpt::AllFeatures);
    cmd.other_options(vec!["--locked".to_string()]);

    let result = cmd.exec().with_context(|| "Invalid manifest")?;

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
    registry: Option<Url>,
    // `Some` if the user has specified an explicit
    // version to upgrade to.
    version: Option<String>,
    old_version: String,
    is_prerelease: bool,
}

/// The set of dependencies to be upgraded, alongside the registries returned from cargo metadata, and
/// the desired versions, if specified by the user.
#[derive(Default, Clone, Debug)]
struct DesiredUpgrades(BTreeMap<Dependency, UpgradeMetadata>);

impl DesiredUpgrades {
    /// Transform the dependencies into their upgraded forms. If a version is specified, all
    /// dependencies will get that version.
    fn into_latest(
        self,
        allow_prerelease: bool,
        manifest_path: &Path,
        preserve_precision: bool,
    ) -> CargoResult<ActualUpgrades> {
        let mut upgrades = ActualUpgrades::default();
        for (
            dep,
            UpgradeMetadata {
                registry,
                version,
                old_version,
                is_prerelease,
            },
        ) in self.0.into_iter()
        {
            if let Some(v) = version {
                upgrades.0.insert(dep, v);
                continue;
            }

            let allow_prerelease = allow_prerelease || is_prerelease;

            let latest = get_latest_dependency(
                &dep.name,
                allow_prerelease,
                manifest_path,
                registry.as_ref(),
            )
            .with_context(|| "Failed to get new version")?;
            let latest_version = latest.version().expect("Invalid dependency type");
            if preserve_precision {
                let latest_version: semver::Version = latest_version.parse()?;
                if let Some(version) =
                    cargo_edit::upgrade_requirement(&old_version, &latest_version)?
                {
                    upgrades.0.insert(dep, version);
                }
            } else {
                upgrades.0.insert(dep, latest_version.to_owned());
            }
        }
        Ok(upgrades)
    }

    fn into_lockfile(
        self,
        locked: &[cargo_metadata::Package],
        preserve_precision: bool,
    ) -> CargoResult<ActualUpgrades> {
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
                let req = semver::VersionReq::parse(&old_version)?;
                if dep.name == p.name && req.matches(&p.version) {
                    let locked_version = &p.version;
                    if preserve_precision {
                        if let Some(version) =
                            cargo_edit::upgrade_requirement(&old_version, locked_version)?
                        {
                            upgrades.0.insert(dep, version);
                        }
                    } else {
                        upgrades.0.insert(dep, locked_version.to_string());
                    }
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
) -> CargoResult<Vec<(LocalManifest, cargo_metadata::Package)>> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.no_deps();
    if let Some(path) = manifest_path {
        cmd.manifest_path(path);
    }
    let result = cmd
        .exec()
        .with_context(|| "Failed to get workspace metadata")?;
    result
        .packages
        .into_iter()
        .map(|package| {
            Ok((
                LocalManifest::try_new(Path::new(&package.manifest_path))?,
                package,
            ))
        })
        .collect::<CargoResult<Vec<_>>>()
}

fn resolve_pkgid(
    manifest_path: Option<&Path>,
    pkgid: &str,
) -> CargoResult<Vec<(LocalManifest, cargo_metadata::Package)>> {
    let package = manifest_from_pkgid(manifest_path, pkgid)?;
    let manifest = LocalManifest::try_new(Path::new(&package.manifest_path))?;
    Ok(vec![(manifest, package)])
}

/// Get the manifest specified by the manifest path. Try to make an educated guess if no path is
/// provided.
fn resolve_local_one(
    manifest_path: Option<&Path>,
) -> CargoResult<Vec<(LocalManifest, cargo_metadata::Package)>> {
    let resolved_manifest_path: String = find(manifest_path)?.to_string_lossy().into();

    let manifest = LocalManifest::find(manifest_path)?;

    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.no_deps();
    if let Some(path) = manifest_path {
        cmd.manifest_path(path);
    }
    let result = cmd.exec().with_context(|| "Invalid manifest")?;
    let packages = result.packages;
    let package = packages
        .iter()
        .find(|p| p.manifest_path == resolved_manifest_path)
        // If we have successfully got metadata, but our manifest path does not correspond to a
        // package, we must have been called against a virtual manifest.
        .with_context(|| {
            "Found virtual manifest, but this command requires running against an \
                 actual package in this workspace. Try adding `--workspace`."
        })?;

    Ok(vec![(manifest, package.to_owned())])
}

fn deprecated_message(message: &str) -> CargoResult<()> {
    let colorchoice = colorize_stderr();
    let mut output = StandardStream::stderr(colorchoice);
    output
        .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))
        .with_context(|| "Failed to set output colour")?;
    writeln!(output, "{}", message).with_context(|| "Failed to write deprecated message")?;
    output
        .set_color(&ColorSpec::new())
        .with_context(|| "Failed to clear output colour")?;
    Ok(())
}

fn dry_run_message() -> CargoResult<()> {
    let colorchoice = colorize_stderr();
    let mut output = StandardStream::stderr(colorchoice);
    output
        .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))
        .with_context(|| "Failed to set output colour")?;
    write!(output, "warning").with_context(|| "Failed to write dry run message")?;
    output
        .set_color(&ColorSpec::new())
        .with_context(|| "Failed to clear output colour")?;
    writeln!(output, ": aborting upgrade due to dry run")
        .with_context(|| "Failed to write dry run message")?;
    Ok(())
}
