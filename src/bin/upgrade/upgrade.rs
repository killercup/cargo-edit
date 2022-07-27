use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;

use cargo_edit::{
    colorize_stderr, find, get_latest_dependency, registry_url, resolve_manifests, set_dep_version,
    shell_note, shell_status, shell_warn, shell_write_stderr, update_registry_index, CargoResult,
    Context, CrateSpec, Dependency, LocalManifest,
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

    if !args.offline && !args.to_lockfile {
        let url = registry_url(&find(args.manifest_path.as_deref())?, None)?;
        update_registry_index(&url, false)?;
    }

    let manifests = args.resolve_targets()?;
    let locked = load_lockfile(&manifests, args.offline).unwrap_or_default();

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
        let mut table = Vec::new();
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
                        shell_warn(&format!("ignoring {}, unsupported entry: {}", dep_key, err))?;
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

                let mut reason = None;
                if !args.pinned {
                    if dependency.rename.is_some() {
                        reason.get_or_insert("pinned");
                        pinned_present = true;
                    }

                    if let Ok(version_req) = VersionReq::parse(&old_version_req) {
                        if version_req.comparators.iter().any(|comparator| {
                            matches!(comparator.op, Op::Exact | Op::Less | Op::LessEq)
                        }) {
                            reason.get_or_insert("pinned");
                            pinned_present = true;
                        }
                    }
                }

                let locked_version =
                    find_locked_version(&dependency.name, &old_version_req, &locked);

                let latest_version = if dependency
                    .source
                    .as_ref()
                    .and_then(|s| s.as_registry())
                    .is_some()
                {
                    // Update indices for any alternative registries, unless
                    // we're offline.
                    let registry_url = dependency
                        .registry()
                        .map(|registry| registry_url(&manifest_path, Some(registry)))
                        .transpose()?;
                    if !args.offline {
                        if let Some(registry_url) = &registry_url {
                            if updated_registries.insert(registry_url.to_owned()) {
                                update_registry_index(registry_url, false)?;
                            }
                        }
                    }
                    let is_prerelease = old_version_req.contains('-');
                    let latest_version = get_latest_dependency(
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
                    latest_version.ok()
                } else {
                    None
                };

                let new_version_req = if reason.is_some() {
                    old_version_req.clone()
                } else if let Some(Some(new_version_req)) =
                    selected_dependencies.get(dependency.toml_key())
                {
                    new_version_req.to_owned()
                } else {
                    let new_version = if args.to_lockfile {
                        if let Some(locked_version) = &locked_version {
                            Some(locked_version.clone())
                        } else {
                            None
                        }
                    } else if let Some(latest_version) = &latest_version {
                        if old_version_compatible(&old_version_req, latest_version) {
                            reason.get_or_insert("compatible");
                            compatible_present = true;
                            None
                        } else {
                            Some(latest_version.clone())
                        }
                    } else {
                        None
                    };
                    if let Some(mut new_version_req) = new_version {
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
                    } else {
                        old_version_req.clone()
                    }
                };
                if new_version_req != old_version_req {
                    set_dep_version(dep_item, &new_version_req)?;
                    crate_modified = true;
                    any_crate_modified = true;
                }
                table.push(Dep {
                    name: dependency.toml_key().to_owned(),
                    old_version_req,
                    locked_version,
                    latest_version,
                    new_version_req,
                    reason,
                });
            }
        }
        if !table.is_empty() {
            print_upgrade(table)?;
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

fn load_lockfile(
    targets: &[cargo_metadata::Package],
    offline: bool,
) -> CargoResult<Vec<cargo_metadata::Package>> {
    // Get locked dependencies. For workspaces with multiple Cargo.toml
    // files, there is only a single lockfile, so it suffices to get
    // metadata for any one of Cargo.toml files.
    let package = targets
        .get(0)
        .ok_or_else(|| anyhow::format_err!("Invalid cargo config"))?;
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.manifest_path(package.manifest_path.clone());
    cmd.features(cargo_metadata::CargoOpt::AllFeatures);
    let mut other = vec!["--locked".to_owned()];
    if offline {
        other.push("--offline".to_owned());
    }
    cmd.other_options(other);

    let result = cmd.exec()?;

    let locked = result.packages;

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

struct Dep {
    name: String,
    old_version_req: String,
    locked_version: Option<String>,
    latest_version: Option<String>,
    new_version_req: String,
    reason: Option<&'static str>,
}

impl Dep {
    fn old_version_req_spec(&self) -> ColorSpec {
        let mut spec = ColorSpec::new();
        if let Some(latest_version) = self
            .latest_version
            .as_ref()
            .and_then(|v| semver::Version::parse(v).ok())
        {
            if let Ok(old_version_req) = semver::VersionReq::parse(&self.old_version_req) {
                if !old_version_req.matches(&latest_version) {
                    spec.set_fg(Some(Color::Yellow));
                }
            }
        }
        spec
    }

    fn locked_version(&self) -> &str {
        self.locked_version.as_deref().unwrap_or("-")
    }

    fn locked_version_spec(&self) -> ColorSpec {
        let mut spec = ColorSpec::new();
        if self.locked_version.is_none() {
        } else if self.locked_version != self.latest_version {
            spec.set_fg(Some(Color::Yellow));
        }
        spec
    }

    fn latest_version(&self) -> &str {
        self.latest_version.as_deref().unwrap_or("-")
    }

    fn new_version_req_spec(&self) -> ColorSpec {
        let mut spec = ColorSpec::new();
        if self.reason.is_some() {
            spec.set_fg(Some(Color::Yellow));
        } else if self.new_version_req != self.old_version_req {
            spec.set_fg(Some(Color::Green));
        }
        spec
    }

    fn reason(&self) -> &str {
        self.reason.unwrap_or("")
    }

    fn reason_spec(&self) -> ColorSpec {
        let mut spec = ColorSpec::new();
        if self.reason.is_some() {
            spec.set_fg(Some(Color::Yellow));
        }
        spec
    }
}

/// Print a message if the new dependency version is different from the old one.
fn print_upgrade(mut deps: Vec<Dep>) -> CargoResult<()> {
    deps.splice(
        0..0,
        [
            Dep {
                name: "name".to_owned(),
                old_version_req: "old req".to_owned(),
                locked_version: Some("locked".to_owned()),
                latest_version: Some("latest".to_owned()),
                new_version_req: "new req".to_owned(),
                reason: Some("note"),
            },
            Dep {
                name: "====".to_owned(),
                old_version_req: "=======".to_owned(),
                locked_version: Some("======".to_owned()),
                latest_version: Some("======".to_owned()),
                new_version_req: "=======".to_owned(),
                reason: Some("===="),
            },
        ],
    );
    let mut width = [0; 6];
    for dep in &deps {
        width[0] = width[0].max(dep.name.len());
        width[1] = width[1].max(dep.old_version_req.len());
        width[2] = width[2].max(dep.locked_version().len());
        width[3] = width[3].max(dep.latest_version().len());
        width[4] = width[4].max(dep.new_version_req.len());
        width[5] = width[5].max(dep.reason().len());
    }
    for (i, dep) in deps.iter().enumerate() {
        let is_header = (0..=1).contains(&i);
        let mut header_spec = ColorSpec::new();
        header_spec.set_bold(true);

        let spec = if is_header {
            header_spec.clone()
        } else {
            ColorSpec::new()
        };
        write_cell(&dep.name, width[0], &spec)?;
        shell_write_stderr(" ", &ColorSpec::new())?;

        let spec = if is_header {
            header_spec.clone()
        } else {
            dep.old_version_req_spec()
        };
        write_cell(&dep.old_version_req, width[1], &spec)?;
        shell_write_stderr(" ", &ColorSpec::new())?;

        let spec = if is_header {
            header_spec.clone()
        } else {
            dep.locked_version_spec()
        };
        write_cell(dep.locked_version(), width[2], &spec)?;
        shell_write_stderr(" ", &ColorSpec::new())?;

        let spec = if is_header {
            header_spec.clone()
        } else {
            ColorSpec::new()
        };
        write_cell(&dep.latest_version(), width[3], &spec)?;
        shell_write_stderr(" ", &ColorSpec::new())?;

        let spec = if is_header {
            header_spec.clone()
        } else {
            dep.new_version_req_spec()
        };
        write_cell(&dep.new_version_req, width[4], &spec)?;
        shell_write_stderr(" ", &ColorSpec::new())?;

        let spec = if is_header {
            header_spec.clone()
        } else {
            dep.reason_spec()
        };
        write_cell(&dep.reason(), width[5], &spec)?;
        shell_write_stderr("\n", &ColorSpec::new())?;
    }

    Ok(())
}

fn write_cell(content: &str, width: usize, spec: &ColorSpec) -> CargoResult<()> {
    shell_write_stderr(content, spec)?;
    for _ in 0..(width - content.len()) {
        shell_write_stderr(" ", &ColorSpec::new())?;
    }
    Ok(())
}
