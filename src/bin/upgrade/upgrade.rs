use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use cargo_edit::{
    find, get_compatible_dependency, get_latest_dependency, registry_url, set_dep_version,
    shell_note, shell_status, shell_warn, shell_write_stderr, update_registry_index, CargoResult,
    CrateSpec, Dependency, LocalManifest,
};
use clap::Args;
use indexmap::IndexMap;
use semver::{Op, VersionReq};
use termcolor::{Color, ColorSpec};

/// Upgrade dependency version requirements in Cargo.toml manifest files
#[derive(Debug, Args)]
#[clap(version)]
pub struct UpgradeArgs {
    /// Path to the manifest to upgrade
    #[clap(long, value_name = "PATH", action)]
    manifest_path: Option<PathBuf>,

    /// Crate to be upgraded
    #[clap(long, short, value_name = "PKGID")]
    package: Vec<String>,

    /// Crates to exclude and not upgrade.
    #[clap(long)]
    exclude: Vec<String>,

    /// Print changes to be made without making them.
    #[clap(long)]
    dry_run: bool,

    /// Recursively update locked dependencies
    #[clap(long, value_name = "true|false", default_value_t = true, action = clap::ArgAction::Set, hide_possible_values = true)]
    recursive: bool,

    /// Upgrade dependencies pinned in the manifest.
    #[clap(long)]
    pinned: bool,

    /// Run without accessing the network
    #[clap(long)]
    offline: bool,

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
    if !args.offline {
        let url = registry_url(&find(args.manifest_path.as_deref())?, None)?;
        update_registry_index(&url, false)?;
    }

    let metadata = resolve_ws(args.manifest_path.as_deref(), args.locked, args.offline)?;
    let manifest_path = metadata.workspace_root.as_std_path().join("Cargo.toml");
    let manifests = find_ws_members(&metadata);

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
    let mut uninteresting_crates = BTreeSet::new();
    for package in &manifests {
        let mut manifest = LocalManifest::try_new(package.manifest_path.as_std_path())?;
        let mut crate_modified = false;
        let mut table = Vec::new();
        let manifest_path = manifest.path.clone();
        shell_status("Checking", &format!("{}'s dependencies", package.name))?;
        for dep_table in manifest.get_dependency_tables_mut() {
            for (dep_key, dep_item) in dep_table.iter_mut() {
                let mut reason = None;

                let dep_key = dep_key.get();
                let dependency = match Dependency::from_toml(&manifest_path, dep_key, dep_item) {
                    Ok(dependency) => dependency,
                    Err(err) => {
                        shell_warn(&format!("ignoring {}, unsupported entry: {}", dep_key, err))?;
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
                        if dependency
                            .source()
                            .map(|s| s.as_git().is_some())
                            .unwrap_or(false)
                        {
                            git_crates.insert(dependency.name.clone());
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

                if !args.pinned {
                    if dependency.rename.is_some() {
                        reason.get_or_insert(Reason::Pinned);
                        pinned_present = true;
                    }

                    if is_pinned_req(&old_version_req) {
                        reason.get_or_insert(Reason::Pinned);
                        pinned_present = true;
                    }
                }

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
                    selected_dependencies.get(&dependency.name)
                {
                    new_version_req.to_owned()
                } else {
                    let new_version_req = if let Some(latest_version) = &latest_version {
                        let mut new_version_req = latest_version.clone();
                        let new_version: semver::Version = latest_version.parse()?;
                        match cargo_edit::upgrade_requirement(&old_version_req, &new_version) {
                            Ok(Some(version_req)) => {
                                new_version_req = version_req;
                            }
                            Err(_) => {}
                            _ => {
                                new_version_req = old_version_req.clone();
                            }
                        }
                        if new_version_req == old_version_req {
                            None
                        } else {
                            Some(new_version_req)
                        }
                    } else {
                        None
                    };
                    new_version_req.unwrap_or_else(|| old_version_req.clone())
                };
                if new_version_req == old_version_req {
                    reason.get_or_insert(Reason::Unchanged);
                }
                if new_version_req != old_version_req {
                    set_dep_version(dep_item, &new_version_req)?;
                    crate_modified = true;
                    modified_crates.insert(dependency.name.clone());
                }
                let display_name = if let Some(rename) = &dependency.rename {
                    format!("{} ({})", dependency.name, rename)
                } else {
                    dependency.name.clone()
                };
                let compatible_version = if dependency
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
                    let latest_version = get_compatible_dependency(
                        &dependency.name,
                        &semver::VersionReq::parse(&old_version_req)
                            .expect("validated when parsing the file"),
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
                table.push(Dep {
                    name: display_name,
                    old_version_req,
                    compatible_version,
                    latest_version,
                    new_version_req,
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
            let offline = true; // index should already be updated
            let metadata = resolve_ws(Some(&manifest_path), args.locked, offline)?;
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
                    for lock_version in locked
                        .iter()
                        .filter(|p| p.name == **name)
                        .map(|p| &p.version)
                        .filter_map(|v| req.matches(v).then(|| v))
                    {
                        let mut cmd = std::process::Command::new("cargo");
                        cmd.arg("update");
                        cmd.arg("--manifest-path").arg(&manifest_path);
                        if args.locked {
                            cmd.arg("--locked");
                        }
                        if args.recursive {
                            // HACK: Since we'll need to skip this during the official recursive
                            // check, let's handle it here
                            cmd.arg("--aggressive");
                        }
                        let dep = format!("{name}@{lock_version}");
                        cmd.arg("--precise").arg(&precise);
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
                let metadata = resolve_ws(Some(&manifest_path), args.locked, offline)?;
                locked = metadata.packages;
            }

            if !git_crates.is_empty() {
                shell_status("Upgrading", "git dependencies")?;
                let mut cmd = std::process::Command::new("cargo");
                cmd.arg("update");
                cmd.arg("--manifest-path").arg(&manifest_path);
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
                let metadata = resolve_ws(Some(&manifest_path), args.locked, offline)?;
                locked = metadata.packages;
            }

            if args.recursive {
                shell_status("Upgrading", "recursive dependencies")?;
                let mut cmd = std::process::Command::new("cargo");
                cmd.arg("update");
                cmd.arg("--manifest-path").arg(&manifest_path);
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
            write!(&mut note, "\n  {}: ", reason)?;
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
    old_version_req: String,
    compatible_version: Option<String>,
    latest_version: Option<String>,
    new_version_req: String,
    reason: Option<Reason>,
}

impl Dep {
    fn old_version_req_spec(&self) -> ColorSpec {
        let mut spec = ColorSpec::new();
        if !self.old_req_matches_latest() {
            spec.set_fg(Some(Color::Yellow));
        }
        spec
    }

    fn old_req_matches_latest(&self) -> bool {
        if let Some(latest_version) = self
            .latest_version
            .as_ref()
            .and_then(|v| semver::Version::parse(v).ok())
        {
            if let Ok(old_version_req) = semver::VersionReq::parse(&self.old_version_req) {
                return old_version_req.matches(&latest_version);
            }
        }
        true
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

    fn new_version_req_spec(&self) -> ColorSpec {
        let mut spec = ColorSpec::new();
        if self.req_changed() {
            if self.reason.is_some() {
                spec.set_fg(Some(Color::Yellow));
            } else {
                spec.set_fg(Some(Color::Green));
                if let Some(latest_version) = self
                    .latest_version
                    .as_ref()
                    .and_then(|v| semver::Version::parse(v).ok())
                {
                    if let Ok(new_version_req) = semver::VersionReq::parse(&self.new_version_req) {
                        if !new_version_req.matches(&latest_version) {
                            spec.set_fg(Some(Color::Yellow));
                        }
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
        if self.reason.is_some() {
            spec.set_fg(Some(Color::Yellow));
        }
        spec
    }

    fn is_interesting(&self) -> bool {
        if self.reason.is_none() {
            return true;
        }

        if self.req_changed() {
            return true;
        }

        if !self.old_req_matches_latest() {
            return true;
        }

        false
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Reason {
    Unchanged,
    Pinned,
    Excluded,
}

impl Reason {
    fn as_short(&self) -> &'static str {
        match self {
            Self::Unchanged => "",
            Self::Pinned => "pinned",
            Self::Excluded => "excluded",
        }
    }

    fn as_long(&self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::Pinned => "pinned",
            Self::Excluded => "excluded",
        }
    }
}

/// Print a message if the new dependency version is different from the old one.
fn print_upgrade(mut interesting: Vec<Dep>) -> CargoResult<()> {
    if !interesting.is_empty() {
        interesting.splice(
            0..0,
            [
                Dep {
                    name: "name".to_owned(),
                    old_version_req: "old req".to_owned(),
                    compatible_version: Some("compatible".to_owned()),
                    latest_version: Some("latest".to_owned()),
                    new_version_req: "new req".to_owned(),
                    reason: None,
                },
                Dep {
                    name: "====".to_owned(),
                    old_version_req: "=======".to_owned(),
                    compatible_version: Some("==========".to_owned()),
                    latest_version: Some("======".to_owned()),
                    new_version_req: "=======".to_owned(),
                    reason: None,
                },
            ],
        );
        let mut width = [0; 6];
        for (i, dep) in interesting.iter().enumerate() {
            width[0] = width[0].max(dep.name.len());
            width[1] = width[1].max(dep.old_version_req.len());
            width[2] = width[2].max(dep.compatible_version().len());
            width[3] = width[3].max(dep.latest_version().len());
            width[4] = width[4].max(dep.new_version_req.len());
            if 1 < i {
                width[5] = width[5].max(dep.short_reason().len());
            }
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
                dep.compatible_version_spec()
            };
            write_cell(dep.compatible_version(), width[2], &spec)?;

            shell_write_stderr(" ", &ColorSpec::new())?;
            let spec = if is_header {
                header_spec.clone()
            } else {
                ColorSpec::new()
            };
            write_cell(dep.latest_version(), width[3], &spec)?;

            shell_write_stderr(" ", &ColorSpec::new())?;
            let spec = if is_header {
                header_spec.clone()
            } else {
                dep.new_version_req_spec()
            };
            write_cell(&dep.new_version_req, width[4], &spec)?;

            if 0 < width[5] {
                shell_write_stderr(" ", &ColorSpec::new())?;
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

            shell_write_stderr("\n", &ColorSpec::new())?;
        }
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
