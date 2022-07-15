use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;

use cargo_edit::{
    colorize_stderr, find, get_latest_dependency, registry_url, resolve_manifests, set_dep_version,
    shell_note, shell_status, shell_warn, update_registry_index, CargoResult, Context, CrateSpec,
    Dependency, LocalManifest,
};
use clap::Args;
use indexmap::IndexMap;
use semver::{Op, VersionReq};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

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
    #[clap(long, value_name = "PATH", parse(from_os_str))]
    manifest_path: Option<PathBuf>,

    /// Package id of the crate to add this dependency to.
    #[clap(
        long = "package",
        short = 'p',
        value_name = "PKGID",
        conflicts_with = "all",
        conflicts_with = "workspace"
    )]
    pkgid: Vec<String>,

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

    /// Print changes to be made without making them.
    #[clap(long)]
    dry_run: bool,

    /// Upgrade dependencies pinned in the manifest.
    #[clap(long)]
    pinned: bool,

    /// Run without accessing the network
    #[clap(long)]
    offline: bool,

    /// Upgrade all packages to the version in the lockfile.
    #[clap(long, conflicts_with = "dependency")]
    to_lockfile: bool,

    /// Crates to exclude and not upgrade.
    #[clap(long)]
    exclude: Vec<String>,

    /// Require `Cargo.toml` to be up to date
    #[clap(long)]
    locked: bool,

    /// Use verbose output
    #[clap(short, long)]
    verbose: bool,

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

    fn resolve_targets(&self) -> CargoResult<Vec<cargo_metadata::Package>> {
        resolve_manifests(
            self.manifest_path.as_deref(),
            self.workspace(),
            self.pkgid.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        )
    }

    fn verbose<F>(&self, mut callback: F) -> CargoResult<()>
    where
        F: FnMut() -> CargoResult<()>,
    {
        if self.verbose {
            callback()
        } else {
            Ok(())
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ArgEnum)]
enum UnstableOptions {}

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

    let selected_dependencies = args
        .dependency
        .iter()
        .map(|name| {
            let spec = CrateSpec::resolve(name)?;
            Ok((spec.name, spec.version_req))
        })
        .collect::<CargoResult<IndexMap<_, _>>>()?;
    let mut processed_keys = BTreeSet::new();

    let mut updated_registries = BTreeSet::new();
    let mut any_crate_modified = false;
    let mut compatible_present = false;
    let mut pinned_present = false;
    for package in manifests {
        let mut manifest = LocalManifest::try_new(package.manifest_path.as_std_path())?;
        let mut crate_modified = false;
        let manifest_path = manifest.path.clone();
        shell_status("Checking", &format!("{}'s dependencies", package.name))?;
        for dep_table in manifest.get_dependency_tables_mut() {
            for (dep_key, dep_item) in dep_table.iter_mut() {
                let dep_key = dep_key.get();
                processed_keys.insert(dep_key.to_owned());
                if !selected_dependencies.is_empty() && !selected_dependencies.contains_key(dep_key)
                {
                    args.verbose(|| {
                        shell_warn(&format!("ignoring {}, excluded by user", dep_key))
                    })?;
                    continue;
                }
                if args.exclude.contains(&dep_key.to_owned()) {
                    args.verbose(|| {
                        shell_warn(&format!("ignoring {}, excluded by user", dep_key))
                    })?;
                    continue;
                }
                let dependency = match Dependency::from_toml(&manifest_path, dep_key, dep_item) {
                    Ok(dependency) => dependency,
                    Err(err) => {
                        shell_warn(&format!("ignoring {}, invalid entry: {}", dep_key, err))?;
                        continue;
                    }
                };
                let old_version_req = match dependency.version() {
                    Some(version_req) => version_req.to_owned(),
                    None => {
                        args.verbose(|| {
                            let source = dependency
                                .source()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "unknown".to_owned());
                            shell_warn(&format!(
                                "ignoring {}, source is {}",
                                dependency.toml_key(),
                                source,
                            ))
                        })?;
                        continue;
                    }
                };
                if !args.pinned {
                    if dependency.rename.is_some() {
                        args.verbose(|| {
                            shell_warn(&format!(
                                "ignoring {}, renamed dependencies are pinned",
                                dependency.toml_key(),
                            ))
                        })?;
                        pinned_present = true;
                        continue;
                    }

                    if let Ok(version_req) = VersionReq::parse(&old_version_req) {
                        if version_req.comparators.iter().any(|comparator| {
                            matches!(comparator.op, Op::Exact | Op::Less | Op::LessEq)
                        }) {
                            args.verbose(|| {
                                shell_warn(&format!(
                                    "ignoring {}, version ({}) is pinned",
                                    dependency.toml_key(),
                                    old_version_req
                                ))
                            })?;
                            pinned_present = true;
                            continue;
                        }
                    }
                }

                let new_version_req = if let Some(Some(new_version_req)) =
                    selected_dependencies.get(dependency.toml_key())
                {
                    new_version_req.to_owned()
                } else {
                    // Not checking `selected_dependencies.is_empty`, it was checked earlier
                    let new_version = if args.to_lockfile {
                        match find_locked_version(&dependency.name, &old_version_req, &locked) {
                            Some(new_version) => new_version,
                            None => {
                                args.verbose(|| {
                                    shell_warn(&format!(
                                        "ignoring {}, could not find package: not in lock file",
                                        dependency.toml_key(),
                                    ))
                                })?;
                                continue;
                            }
                        }
                    } else {
                        if dependency
                            .source
                            .as_ref()
                            .and_then(|s| s.as_registry())
                            .is_none()
                        {
                            args.verbose(|| {
                                let source = dependency
                                    .source()
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| "unknown".to_owned());
                                shell_warn(&format!(
                                    "ignoring {}, source is {}",
                                    dependency.toml_key(),
                                    source
                                ))
                            })?;
                            continue;
                        }
                        // Update indices for any alternative registries, unless
                        // we're offline.
                        let registry_url = dependency
                            .registry()
                            .map(|registry| registry_url(&manifest_path, Some(registry)))
                            .transpose()?;
                        if !args.offline && std::env::var("CARGO_IS_TEST").is_err() {
                            if let Some(registry_url) = &registry_url {
                                if updated_registries.insert(registry_url.to_owned()) {
                                    update_registry_index(registry_url, false)?;
                                }
                            }
                        }
                        let is_prerelease = old_version_req.contains('-');
                        let new_version = get_latest_dependency(
                            &dependency.name,
                            is_prerelease,
                            &manifest_path,
                            registry_url.as_ref(),
                        )
                        .map(|d| {
                            d.version()
                                .expect("registry packages always have a version")
                                .to_owned()
                        });
                        let new_version = match new_version {
                            Ok(new_version) => new_version,
                            Err(err) => {
                                args.verbose(|| {
                                    shell_warn(&format!(
                                        "ignoring {}, could not find package: {}",
                                        dependency.toml_key(),
                                        err
                                    ))
                                })?;
                                continue;
                            }
                        };
                        if old_version_compatible(&old_version_req, &new_version) {
                            args.verbose(|| {
                                shell_warn(&format!(
                                    "ignoring {}, version ({}) is compatible with {}",
                                    dependency.toml_key(),
                                    old_version_req,
                                    new_version
                                ))
                            })?;
                            compatible_present = true;
                            continue;
                        }
                        new_version
                    };
                    let mut new_version_req = new_version;
                    let new_ver: semver::Version = new_version_req.parse()?;
                    match cargo_edit::upgrade_requirement(&old_version_req, &new_ver) {
                        Ok(Some(version)) => {
                            new_version_req = version;
                        }
                        Err(_) => {}
                        _ => {
                            new_version_req = old_version_req.clone();
                        }
                    }
                    new_version_req
                };
                if new_version_req == old_version_req {
                    args.verbose(|| {
                        shell_warn(&format!(
                            "ignoring {}, version ({}) is unchanged",
                            dependency.toml_key(),
                            new_version_req
                        ))
                    })?;
                    continue;
                }
                print_upgrade(dependency.toml_key(), &old_version_req, &new_version_req)?;
                set_dep_version(dep_item, &new_version_req)?;
                crate_modified = true;
                any_crate_modified = true;
            }
        }
        if !args.dry_run && !args.locked && crate_modified {
            manifest.write()?;
        }
    }

    if args.locked && any_crate_modified {
        anyhow::bail!("cannot upgrade due to `--locked`");
    }

    let unused = selected_dependencies
        .keys()
        .filter(|k| !processed_keys.contains(k.as_str()))
        .map(|k| k.as_str())
        .collect::<Vec<_>>();
    match unused.len() {
        0 => {}
        1 => anyhow::bail!("dependency {} doesn't exist", unused.join(", ")),
        _ => anyhow::bail!("dependencies {} don't exist", unused.join(", ")),
    }

    if pinned_present {
        shell_note("Re-run with `--pinned` to upgrade pinned version requirements")?;
    }
    if compatible_present {
        shell_note("Re-run with `--to-lockfile` to upgrade compatible version requirements")?;
    }

    if args.dry_run {
        shell_warn("aborting upgrade due to dry run")?;
    }

    Ok(())
}

fn load_lockfile(targets: &[cargo_metadata::Package]) -> CargoResult<Vec<cargo_metadata::Package>> {
    // Get locked dependencies. For workspaces with multiple Cargo.toml
    // files, there is only a single lockfile, so it suffices to get
    // metadata for any one of Cargo.toml files.
    let package = targets
        .get(0)
        .ok_or_else(|| anyhow::format_err!("Invalid cargo config"))?;
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.manifest_path(package.manifest_path.clone());
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

fn find_locked_version(
    dep_name: &str,
    old_version: &str,
    locked: &[cargo_metadata::Package],
) -> Option<String> {
    let req = semver::VersionReq::parse(old_version).ok()?;
    for p in locked {
        if dep_name == p.name && req.matches(&p.version) {
            return Some(p.version.to_string());
        }
    }
    None
}

fn old_version_compatible(old_version_req: &str, new_version: &str) -> bool {
    let old_version_req = match VersionReq::parse(old_version_req) {
        Ok(req) => req,
        Err(_) => return false,
    };

    let new_version = match semver::Version::parse(new_version) {
        Ok(new_version) => new_version,
        // HACK: Skip compatibility checks on incomplete version reqs
        Err(_) => return false,
    };

    old_version_req.matches(&new_version)
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

/// Print a message if the new dependency version is different from the old one.
fn print_upgrade(dep_name: &str, old_version: &str, new_version: &str) -> CargoResult<()> {
    let old_version = format_version_req(old_version);
    let new_version = format_version_req(new_version);

    shell_status(
        "Upgrading",
        &format!("{dep_name}: {old_version} -> {new_version}"),
    )?;

    Ok(())
}

fn format_version_req(version: &str) -> String {
    if version.chars().next().unwrap_or('0').is_ascii_digit() {
        format!("v{}", version)
    } else {
        version.to_owned()
    }
}
