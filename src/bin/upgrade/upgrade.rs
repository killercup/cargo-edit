use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use cargo_edit::{
    find, get_compatible_dependency, get_latest_dependency, registry_url, set_dep_version,
    shell_note, shell_status, shell_warn, shell_write_stdout, update_registry_index, CargoResult,
    CrateSpec, Dependency, LocalManifest, Source,
};
use clap::Args;
use indexmap::IndexMap;
use semver::{Op, VersionReq};
use termcolor::{Color, ColorSpec};

/// Upgrade dependency version requirements in Cargo.toml manifest files
#[derive(Debug, Args)]
#[command(version)]
pub struct UpgradeArgs {
    /// Print changes to be made without making them.
    #[arg(long)]
    dry_run: bool,

    /// Path to the manifest to upgrade
    #[arg(long, value_name = "PATH")]
    manifest_path: Option<PathBuf>,

    /// Run without accessing the network
    #[arg(long)]
    offline: bool,

    /// Require `Cargo.toml` to be up to date
    #[arg(long)]
    locked: bool,

    /// Use verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Unstable (nightly-only) flags
    #[arg(short = 'Z', value_name = "FLAG", global = true, value_enum)]
    unstable_features: Vec<UnstableOptions>,

    /// Upgrade to latest compatible version
    #[arg(
        long,
        num_args=0..=1,
        value_name = "allow|ignore",
        hide_possible_values = true,
        default_value = "allow",
        default_missing_value = "allow",
        help_heading = "Version",
        value_enum,
    )]
    compatible: Status,

    /// Upgrade to latest incompatible version
    #[arg(
        short,
        long,
        num_args=0..=1,
        value_name = "allow|ignore",
        hide_possible_values = true,
        default_value = "ignore",
        default_missing_value = "allow",
        help_heading = "Version",
        value_enum,
    )]
    incompatible: Status,

    /// Upgrade pinned to latest incompatible version
    #[arg(
        long,
        num_args=0..=1,
        value_name = "allow|ignore",
        hide_possible_values = true,
        default_value = "ignore",
        default_missing_value = "allow",
        help_heading = "Version",
        value_enum,
    )]
    pinned: Status,

    /// Crate to be upgraded
    #[arg(
        long,
        short,
        value_name = "PKGID[@<VERSION>]",
        help_heading = "Dependencies"
    )]
    package: Vec<String>,

    /// Crates to exclude and not upgrade.
    #[arg(long, value_name = "PKGID", help_heading = "Dependencies")]
    exclude: Vec<String>,

    /// Recursively update locked dependencies
    #[arg(
        long,
        num_args=0..=1,
        action = clap::ArgAction::Set,
        value_name = "true|false",
        default_value = "true",
        default_missing_value = "true",
        hide_possible_values = true,
        help_heading = "Dependencies"
    )]
    recursive: bool,
}

impl UpgradeArgs {
    pub fn exec(self) -> CargoResult<()> {
        exec(self)
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum Status {
    #[value(alias = "true")]
    Allow,
    #[value(alias = "false")]
    Ignore,
}

impl Status {
    fn as_bool(&self) -> bool {
        match self {
            Self::Allow => true,
            Self::Ignore => false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum UnstableOptions {}

/// Main processing function. Allows us to return a `Result` so that `main` can print pretty error
/// messages.
fn exec(args: UpgradeArgs) -> CargoResult<()> {
    if !args.offline {
        let url = registry_url(&find(args.manifest_path.as_deref())?, None)?;
        update_registry_index(&url, false)?;
    }

    let metadata = resolve_ws(args.manifest_path.as_deref(), args.locked, args.offline)?;
    let root_manifest_path = metadata.workspace_root.as_std_path().join("Cargo.toml");
    let manifests = find_ws_members(&metadata);
    let mut manifests = manifests
        .into_iter()
        .map(|p| (p.name, p.manifest_path.as_std_path().to_owned()))
        .collect::<Vec<_>>();
    if !manifests.iter().any(|(_, p)| *p == root_manifest_path) {
        manifests.insert(
            0,
            ("virtual workspace".to_owned(), root_manifest_path.clone()),
        );
    }

    let selected_dependencies = args
        .package
        .iter()
        .map(|name| {
            let spec = CrateSpec::resolve(name)?;
            Ok((spec.name, spec.version_req))
        })
        .collect::<CargoResult<IndexMap<_, Option<_>>>>()?;
    let mut processed_keys = BTreeSet::new();

    let mut updated_registries = BTreeSet::new();
    let mut modified_crates = BTreeSet::new();
    let mut git_crates = BTreeSet::new();
    let mut pinned_present = false;
    let mut incompatible_present = false;
    let mut uninteresting_crates = BTreeSet::new();
    for (pkg_name, manifest_path) in &manifests {
        let mut manifest = LocalManifest::try_new(manifest_path)?;
        let mut crate_modified = false;
        let mut table = Vec::new();
        shell_status("Checking", &format!("{pkg_name}'s dependencies"))?;
        for dep_table in manifest.get_dependency_tables_mut() {
            for (dep_key, dep_item) in dep_table.iter_mut() {
                let mut reason = None;

                let dep_key = dep_key.get();
                let dependency = match Dependency::from_toml(manifest_path, dep_key, dep_item) {
                    Ok(dependency) => dependency,
                    Err(err) => {
                        shell_warn(&format!("ignoring {dep_key}, unsupported entry: {err}"))?;
                        continue;
                    }
                };
                processed_keys.insert(dependency.name.clone());
                if !selected_dependencies.is_empty()
                    && !selected_dependencies.contains_key(&dependency.name)
                {
                    reason.get_or_insert(Reason::Excluded);
                }
                if args.exclude.contains(&dependency.name) {
                    reason.get_or_insert(Reason::Excluded);
                }
                let old_version_req = match dependency.version() {
                    Some(version_req) => version_req.to_owned(),
                    None => {
                        let maybe_reason = match dependency.source() {
                            Some(Source::Git(_)) => {
                                git_crates.insert(dependency.name.clone());
                                Some(Reason::GitSource)
                            }
                            Some(Source::Path(_)) => Some(Reason::PathSource),
                            Some(Source::Workspace(_)) | Some(Source::Registry(_)) | None => None,
                        };
                        if let Some(maybe_reason) = maybe_reason {
                            reason.get_or_insert(maybe_reason);
                            let display_name = if let Some(rename) = &dependency.rename {
                                format!("{} ({})", dependency.name, rename)
                            } else {
                                dependency.name.clone()
                            };
                            table.push(Dep {
                                name: display_name,
                                old_version_req: None,
                                compatible_version: None,
                                latest_version: None,
                                new_version_req: None,
                                reason,
                            });
                        } else {
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
                        }
                        continue;
                    }
                };

                let (latest_compatible, latest_incompatible) = if dependency
                    .source
                    .as_ref()
                    .and_then(|s| s.as_registry())
                    .is_some()
                {
                    // Update indices for any alternative registries, unless
                    // we're offline.
                    let registry_url = dependency
                        .registry()
                        .map(|registry| registry_url(manifest_path, Some(registry)))
                        .transpose()?;
                    if !args.offline {
                        if let Some(registry_url) = &registry_url {
                            if updated_registries.insert(registry_url.to_owned()) {
                                update_registry_index(registry_url, false)?;
                            }
                        }
                    }
                    let latest_compatible = semver::VersionReq::parse(&old_version_req)
                        .ok()
                        .and_then(|old_version_req| {
                            get_compatible_dependency(
                                &dependency.name,
                                &old_version_req,
                                manifest_path,
                                registry_url.as_ref(),
                            )
                            .ok()
                        })
                        .map(|d| {
                            d.version()
                                .expect("registry packages always have a version")
                                .to_owned()
                        });
                    let is_prerelease = old_version_req.contains('-');
                    let latest_version = get_latest_dependency(
                        &dependency.name,
                        is_prerelease,
                        manifest_path,
                        registry_url.as_ref(),
                    )
                    .map(|d| {
                        d.version()
                            .expect("registry packages always have a version")
                            .to_owned()
                    })
                    .ok();
                    let latest_incompatible = if latest_version != latest_compatible {
                        latest_version
                    } else {
                        // Its compatible
                        None
                    };
                    (latest_compatible, latest_incompatible)
                } else {
                    (None, None)
                };

                let is_pinned_dep = dependency.rename.is_some() || is_pinned_req(&old_version_req);

                let mut new_version_req = if reason.is_some() {
                    Some(old_version_req.clone())
                } else {
                    None
                };

                if new_version_req.is_none() {
                    if let Some(Some(explicit_version_req)) =
                        selected_dependencies.get(&dependency.name)
                    {
                        if is_pinned_dep && !args.pinned.as_bool() {
                            // `--pinned` is required in case the user meant an unpinned version
                            // in the dependency tree
                            reason.get_or_insert(Reason::Pinned);
                            pinned_present = true;
                        } else {
                            new_version_req = Some(explicit_version_req.to_owned())
                        }
                    }
                }

                if new_version_req.is_none() {
                    if let Some(latest_incompatible) = &latest_incompatible {
                        let new_version: semver::Version = latest_incompatible.parse()?;
                        let req_candidate =
                            match cargo_edit::upgrade_requirement(&old_version_req, &new_version) {
                                Ok(Some(version_req)) => Some(version_req),
                                Err(_) => {
                                    // Didn't know how to preserve existing format, so abandon it
                                    Some(latest_incompatible.clone())
                                }
                                _ => {
                                    // Already at latest
                                    None
                                }
                            };

                        if req_candidate.is_some() {
                            if is_pinned_dep && !args.pinned.as_bool() {
                                // `--pinned` is required for incompatible upgrades
                                reason.get_or_insert(Reason::Pinned);
                                pinned_present = true;
                            } else if !args.incompatible.as_bool() && !is_pinned_dep {
                                // `--incompatible` is required for non-pinned deps
                                reason.get_or_insert(Reason::Incompatible);
                                incompatible_present = true;
                            } else {
                                new_version_req = req_candidate;
                            }
                        }
                    }
                }

                if new_version_req.is_none() {
                    if let Some(latest_compatible) = &latest_compatible {
                        // Compatible upgrades are allowed for pinned
                        let new_version: semver::Version = latest_compatible.parse()?;
                        let req_candidate =
                            match cargo_edit::upgrade_requirement(&old_version_req, &new_version) {
                                Ok(Some(version_req)) => Some(version_req),
                                Err(_) => {
                                    // Do not change syntax for compatible upgrades
                                    Some(old_version_req.clone())
                                }
                                _ => {
                                    // Already at latest
                                    None
                                }
                            };

                        if req_candidate.is_some() {
                            if !args.compatible.as_bool() {
                                reason.get_or_insert(Reason::Compatible);
                            } else {
                                new_version_req = req_candidate;
                            }
                        }
                    }
                }

                let new_version_req = new_version_req.unwrap_or_else(|| old_version_req.clone());

                if new_version_req == old_version_req {
                    reason.get_or_insert(Reason::Unchanged);
                } else {
                    set_dep_version(dep_item, &new_version_req)?;
                    crate_modified = true;
                    modified_crates.insert(dependency.name.clone());
                }

                let display_name = if let Some(rename) = &dependency.rename {
                    format!("{} ({})", dependency.name, rename)
                } else {
                    dependency.name.clone()
                };
                let compatible_version = latest_compatible;
                let latest_version = latest_incompatible.or_else(|| compatible_version.clone());
                table.push(Dep {
                    name: display_name,
                    old_version_req: Some(old_version_req),
                    compatible_version,
                    latest_version,
                    new_version_req: Some(new_version_req),
                    reason,
                });
            }
        }
        if !table.is_empty() {
            let (interesting, uninteresting) = if args.verbose {
                (table, Vec::new())
            } else {
                table
                    .into_iter()
                    .partition::<Vec<_>, _>(Dep::is_interesting)
            };
            print_upgrade(interesting)?;
            uninteresting_crates.extend(uninteresting);
        }
        if !args.dry_run && !args.locked && crate_modified {
            manifest.write()?;
        }
    }

    if !modified_crates.is_empty() && !args.dry_run {
        if args.locked {
            anyhow::bail!("cannot upgrade due to `--locked`");
        } else {
            // Ensure lock file is updated and collect data for `recursive`
            let metadata = resolve_ws(Some(&root_manifest_path), args.locked, args.offline)?;
            let mut locked = metadata.packages;

            let precise_deps = selected_dependencies
                .iter()
                .filter_map(|(name, req)| {
                    req.as_ref()
                        .and_then(|req| semver::VersionReq::parse(req).ok())
                        .and_then(|req| {
                            let precise = precise_version(&req)?;
                            Some((name, (req, precise)))
                        })
                })
                .collect::<BTreeMap<_, _>>();
            if !precise_deps.is_empty() {
                // Rollback the updates to the precise version
                //
                // Reusing updates (resolve_ws) so we know what lock_version to reference
                for (name, (req, precise)) in &precise_deps {
                    #[allow(clippy::unnecessary_lazy_evaluations)] // requires 1.62
                    for lock_version in locked
                        .iter()
                        .filter(|p| p.name == **name)
                        .map(|p| &p.version)
                        .filter_map(|v| req.matches(v).then(|| v))
                    {
                        let mut cmd = std::process::Command::new("cargo");
                        cmd.arg("update");
                        cmd.arg("--manifest-path").arg(&root_manifest_path);
                        if args.locked {
                            cmd.arg("--locked");
                        }
                        // NOTE: This will skip the official recursive check and we don't
                        // recursively update its dependencies
                        let dep = format!("{name}@{lock_version}");
                        cmd.arg("--precise").arg(precise);
                        cmd.arg("--package").arg(dep);
                        // If we're going to request an update, it would have already been done by now
                        cmd.arg("--offline");
                        let output = cmd.output().context("failed to lock to precise version")?;
                        if !output.status.success() {
                            return Err(anyhow::format_err!(
                                "{}",
                                String::from_utf8_lossy(&output.stderr)
                            ))
                            .context("failed to lock to precise version");
                        }
                    }
                }

                // Update data for `recursive` with precise_deps
                let offline = true; // index should already be updated
                let metadata = resolve_ws(Some(&root_manifest_path), args.locked, offline)?;
                locked = metadata.packages;
            }

            if !git_crates.is_empty() && args.compatible.as_bool() {
                shell_status("Upgrading", "git dependencies")?;
                let mut cmd = std::process::Command::new("cargo");
                cmd.arg("update");
                cmd.arg("--manifest-path").arg(&root_manifest_path);
                if args.locked {
                    cmd.arg("--locked");
                }
                for dep in git_crates.iter() {
                    for lock_version in locked
                        .iter()
                        .filter(|p| {
                            p.name == *dep
                                && p.source
                                    .as_ref()
                                    .map(|s| s.repr.starts_with("git+"))
                                    .unwrap_or(false)
                        })
                        .map(|p| &p.version)
                    {
                        let dep = format!("{dep}@{lock_version}");
                        cmd.arg("--package").arg(dep);
                    }
                }
                // If we're going to request an update, it would have already been done by now
                cmd.arg("--offline");
                let status = cmd.status().context("recursive dependency update failed")?;
                if !status.success() {
                    anyhow::bail!("recursive dependency update failed");
                }

                // Update data for `recursive` with precise_deps
                let offline = true; // index should already be updated
                let metadata = resolve_ws(Some(&root_manifest_path), args.locked, offline)?;
                locked = metadata.packages;
            }

            if args.recursive {
                shell_status("Upgrading", "recursive dependencies")?;
                let mut cmd = std::process::Command::new("cargo");
                cmd.arg("update");
                cmd.arg("--manifest-path").arg(&root_manifest_path);
                if args.locked {
                    cmd.arg("--locked");
                }
                // Limit recursive update to what we touched
                cmd.arg("--aggressive");
                let mut still_run = false;
                for dep in modified_crates
                    .iter()
                    // Already updated so avoid discarding the precise version selection
                    .filter(|c| !precise_deps.contains_key(c))
                {
                    for lock_version in locked.iter().filter(|p| p.name == *dep).map(|p| &p.version)
                    {
                        let dep = format!("{dep}@{lock_version}");
                        cmd.arg("--package").arg(dep);
                        still_run = true;
                    }
                }
                // If we're going to request an update, it would have already been done by now
                cmd.arg("--offline");
                if still_run {
                    let status = cmd.status().context("recursive dependency update failed")?;
                    if !status.success() {
                        anyhow::bail!("recursive dependency update failed");
                    }
                }
            }
        }
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
    if incompatible_present {
        shell_note("Re-run with `--incompatible` to upgrade incompatible version requirements")?;
    }

    if !uninteresting_crates.is_empty() {
        let mut categorize = BTreeMap::new();
        for dep in uninteresting_crates {
            categorize
                .entry(dep.long_reason())
                .or_insert_with(BTreeSet::new)
                .insert(dep.name);
        }
        let mut note = "Re-run with `--verbose` to show all dependencies".to_owned();
        for (reason, deps) in categorize {
            use std::fmt::Write;
            write!(&mut note, "\n  {reason}: ")?;
            for (i, dep) in deps.into_iter().enumerate() {
                if 0 < i {
                    note.push_str(", ");
                }
                note.push_str(&dep);
            }
        }
        shell_note(&note)?;
    }

    if args.dry_run {
        shell_warn("aborting upgrade due to dry run")?;
    }

    Ok(())
}

fn resolve_ws(
    manifest_path: Option<&Path>,
    locked: bool,
    offline: bool,
) -> CargoResult<cargo_metadata::Metadata> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    if let Some(manifest_path) = manifest_path {
        cmd.manifest_path(manifest_path);
    }
    cmd.features(cargo_metadata::CargoOpt::AllFeatures);
    let mut other = Vec::new();
    if locked {
        other.push("--locked".to_owned());
    }
    if offline {
        other.push("--offline".to_owned());
    }
    cmd.other_options(other);

    let ws = cmd.exec().or_else(|_| {
        cmd.no_deps();
        cmd.exec()
    })?;
    Ok(ws)
}

fn find_ws_members(ws: &cargo_metadata::Metadata) -> Vec<cargo_metadata::Package> {
    let workspace_members: std::collections::HashSet<_> = ws.workspace_members.iter().collect();
    ws.packages
        .iter()
        .filter(|p| workspace_members.contains(&p.id))
        .cloned()
        .collect()
}

fn is_pinned_req(old_version_req: &str) -> bool {
    if let Ok(version_req) = VersionReq::parse(old_version_req) {
        version_req.comparators.iter().any(|comparator| {
            matches!(
                comparator.op,
                Op::Exact | Op::Less | Op::LessEq | Op::Wildcard
            )
        })
    } else {
        false
    }
}

fn precise_version(version_req: &VersionReq) -> Option<String> {
    version_req
        .comparators
        .iter()
        .filter(|c| {
            matches!(
                c.op,
                // Only ops we can determine a precise version from
                semver::Op::Exact
                    | semver::Op::GreaterEq
                    | semver::Op::LessEq
                    | semver::Op::Tilde
                    | semver::Op::Caret
                    | semver::Op::Wildcard
            )
        })
        .filter_map(|c| {
            // Only do it when full precision is specified
            c.minor.and_then(|minor| {
                c.patch.map(|patch| semver::Version {
                    major: c.major,
                    minor,
                    patch,
                    pre: c.pre.clone(),
                    build: Default::default(),
                })
            })
        })
        .max()
        .map(|v| v.to_string())
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Dep {
    name: String,
    old_version_req: Option<String>,
    compatible_version: Option<String>,
    latest_version: Option<String>,
    new_version_req: Option<String>,
    reason: Option<Reason>,
}

impl Dep {
    fn old_version_req(&self) -> &str {
        self.old_version_req.as_deref().unwrap_or("-")
    }

    fn old_version_req_spec(&self) -> ColorSpec {
        ColorSpec::new()
    }

    fn compatible_version(&self) -> &str {
        self.compatible_version.as_deref().unwrap_or("-")
    }

    fn compatible_version_spec(&self) -> ColorSpec {
        let mut spec = ColorSpec::new();
        if !self.is_compatible_latest() {
            spec.set_fg(Some(Color::Yellow));
        }
        spec
    }

    fn is_compatible_latest(&self) -> bool {
        if self.compatible_version.is_none() || self.latest_version.is_none() {
            true
        } else {
            self.compatible_version == self.latest_version
        }
    }

    fn latest_version(&self) -> &str {
        self.latest_version.as_deref().unwrap_or("-")
    }

    fn new_version_req(&self) -> &str {
        self.new_version_req.as_deref().unwrap_or("-")
    }

    fn new_version_req_spec(&self) -> ColorSpec {
        let mut spec = ColorSpec::new();
        if self.req_changed() {
            spec.set_fg(Some(Color::Green));
        }
        if self.reason.unwrap_or(Reason::Unchanged).is_upgradeable() {
            spec.set_fg(Some(Color::Yellow));
        }
        if let Some(latest_version) = self
            .latest_version
            .as_ref()
            .and_then(|v| semver::Version::parse(v).ok())
        {
            if let Some(new_version_req) = &self.new_version_req {
                if let Ok(new_version_req) = semver::VersionReq::parse(new_version_req) {
                    if !new_version_req.matches(&latest_version) {
                        spec.set_fg(Some(Color::Red));
                    }
                }
            }
        }
        spec
    }

    fn req_changed(&self) -> bool {
        self.new_version_req != self.old_version_req
    }

    fn short_reason(&self) -> &'static str {
        self.reason.map(|r| r.as_short()).unwrap_or("")
    }

    fn long_reason(&self) -> &'static str {
        self.reason.map(|r| r.as_long()).unwrap_or("")
    }

    fn reason_spec(&self) -> ColorSpec {
        let mut spec = ColorSpec::new();
        if self.reason.unwrap_or(Reason::Unchanged).is_warning() {
            spec.set_fg(Some(Color::Yellow));
        }
        spec
    }

    fn is_interesting(&self) -> bool {
        if self.reason.unwrap_or(Reason::Unchanged).is_upgradeable() {
            return true;
        }

        if self.req_changed() {
            return true;
        }

        if !self.old_req_matches_latest() {
            // Show excluded cases with potential
            return true;
        }

        false
    }

    fn old_req_matches_latest(&self) -> bool {
        if let Some(latest_version) = self
            .latest_version
            .as_ref()
            .and_then(|v| semver::Version::parse(v).ok())
        {
            if let Some(old_version_req) = &self.old_version_req {
                if let Ok(old_version_req) = semver::VersionReq::parse(old_version_req) {
                    return old_version_req.matches(&latest_version);
                }
            }
        }
        true
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Reason {
    Unchanged,
    Compatible,
    Incompatible,
    Pinned,
    GitSource,
    PathSource,
    Excluded,
}

impl Reason {
    fn is_upgradeable(&self) -> bool {
        match self {
            Self::Unchanged => false,
            Self::Compatible => true,
            Self::Incompatible => true,
            Self::Pinned => true,
            Self::GitSource => false,
            Self::PathSource => false,
            Self::Excluded => false,
        }
    }

    fn is_warning(&self) -> bool {
        match self {
            Self::Unchanged => false,
            Self::Compatible => false,
            Self::Incompatible => true,
            Self::Pinned => true,
            Self::GitSource => false,
            Self::PathSource => false,
            Self::Excluded => false,
        }
    }

    fn as_short(&self) -> &'static str {
        match self {
            Self::Unchanged => "",
            Self::Compatible => "compatible",
            Self::Incompatible => "incompatible",
            Self::Pinned => "pinned",
            Self::GitSource => "git",
            Self::PathSource => "local",
            Self::Excluded => "excluded",
        }
    }

    fn as_long(&self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Compatible => "compatible",
            Self::Incompatible => "incompatible",
            Self::Pinned => "pinned",
            Self::GitSource => "git",
            Self::PathSource => "local",
            Self::Excluded => "excluded",
        }
    }
}

/// Print a message if the new dependency version is different from the old one.
fn print_upgrade(mut interesting: Vec<Dep>) -> CargoResult<()> {
    if interesting.is_empty() {
        return Ok(());
    }
    interesting.splice(
        0..0,
        [
            Dep {
                name: "name".to_owned(),
                old_version_req: Some("old req".to_owned()),
                compatible_version: Some("compatible".to_owned()),
                latest_version: Some("latest".to_owned()),
                new_version_req: Some("new req".to_owned()),
                reason: None,
            },
            Dep {
                name: "====".to_owned(),
                old_version_req: Some("=======".to_owned()),
                compatible_version: Some("==========".to_owned()),
                latest_version: Some("======".to_owned()),
                new_version_req: Some("=======".to_owned()),
                reason: None,
            },
        ],
    );
    let mut width = [0; 6];
    for (i, dep) in interesting.iter().enumerate() {
        width[0] = width[0].max(dep.name.len());
        width[1] = width[1].max(dep.old_version_req().len());
        width[2] = width[2].max(dep.compatible_version().len());
        width[3] = width[3].max(dep.latest_version().len());
        width[4] = width[4].max(dep.new_version_req().len());
        if 1 < i {
            width[5] = width[5].max(dep.short_reason().len());
        }
    }
    if 0 < width[5] {
        width[5] = width[5].max("note".len());
    }

    for (i, dep) in interesting.iter().enumerate() {
        let is_header = (0..=1).contains(&i);
        let mut header_spec = ColorSpec::new();
        header_spec.set_bold(true);

        let spec = if is_header {
            header_spec.clone()
        } else {
            ColorSpec::new()
        };
        write_cell(&dep.name, width[0], &spec)?;

        shell_write_stdout(" ", &ColorSpec::new())?;
        let spec = if is_header {
            header_spec.clone()
        } else {
            dep.old_version_req_spec()
        };
        write_cell(dep.old_version_req(), width[1], &spec)?;

        shell_write_stdout(" ", &ColorSpec::new())?;
        let spec = if is_header {
            header_spec.clone()
        } else {
            dep.compatible_version_spec()
        };
        write_cell(dep.compatible_version(), width[2], &spec)?;

        shell_write_stdout(" ", &ColorSpec::new())?;
        let spec = if is_header {
            header_spec.clone()
        } else {
            ColorSpec::new()
        };
        write_cell(dep.latest_version(), width[3], &spec)?;

        shell_write_stdout(" ", &ColorSpec::new())?;
        let spec = if is_header {
            header_spec.clone()
        } else {
            dep.new_version_req_spec()
        };
        write_cell(dep.new_version_req(), width[4], &spec)?;

        if 0 < width[5] {
            shell_write_stdout(" ", &ColorSpec::new())?;
            let spec = if is_header {
                header_spec.clone()
            } else {
                dep.reason_spec()
            };
            let reason = match i {
                0 => "note",
                1 => "====",
                _ => dep.short_reason(),
            };
            write_cell(reason, width[5], &spec)?;
        }

        shell_write_stdout("\n", &ColorSpec::new())?;
    }

    Ok(())
}

fn write_cell(content: &str, width: usize, spec: &ColorSpec) -> CargoResult<()> {
    shell_write_stdout(content, spec)?;
    for _ in 0..(width - content.len()) {
        shell_write_stdout(" ", &ColorSpec::new())?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn exact_is_pinned_req() {
        let req = "=3";
        assert!(is_pinned_req(req));
    }

    #[test]
    fn less_than_is_pinned_req() {
        let req = "<3";
        assert!(is_pinned_req(req));
    }

    #[test]
    fn less_than_equal_is_pinned_req() {
        let req = "<=3";
        assert!(is_pinned_req(req));
    }

    #[test]
    fn minor_wildcard_is_pinned_req() {
        let req = "3.*";
        assert!(is_pinned_req(req));
    }

    #[test]
    fn major_wildcard_is_not_pinned() {
        let req = "*";
        assert!(!is_pinned_req(req));
    }

    #[test]
    fn greater_than_is_not_pinned() {
        let req = ">3";
        assert!(!is_pinned_req(req));
    }

    #[test]
    fn greater_than_equal_is_not_pinned() {
        let req = ">=3";
        assert!(!is_pinned_req(req));
    }

    #[test]
    fn caret_is_not_pinned() {
        let req = "^3";
        assert!(!is_pinned_req(req));
    }

    #[test]
    fn default_is_not_pinned() {
        let req = "3";
        assert!(!is_pinned_req(req));
    }
}
