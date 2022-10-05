use std::path::Path;
use std::path::PathBuf;

use cargo_edit::{shell_status, shell_warn, upgrade_requirement, LocalManifest};
use clap::Args;

use crate::errors::*;
use crate::version::BumpLevel;
use crate::version::TargetVersion;

/// Change a package's version in the local manifest file (i.e. Cargo.toml).
#[derive(Debug, Args)]
#[command(version)]
#[command(group = clap::ArgGroup::new("ver").multiple(false))]
pub struct VersionArgs {
    /// Version to change manifests to
    #[arg(group = "ver")]
    target: Option<semver::Version>,

    /// Increment manifest version
    #[arg(long, group = "ver")]
    bump: Option<BumpLevel>,

    /// Specify the version metadata field (e.g. a wrapped libraries version)
    #[arg(short, long)]
    pub metadata: Option<String>,

    /// Path to the manifest to upgrade
    #[arg(long, value_name = "PATH")]
    manifest_path: Option<PathBuf>,

    /// Package id of the crate to change the version of.
    #[arg(
        long = "package",
        short = 'p',
        value_name = "PKGID",
        conflicts_with = "all",
        conflicts_with = "workspace"
    )]
    pkgid: Vec<String>,

    /// Modify all packages in the workspace.
    #[arg(
        long,
        help = "[deprecated in favor of `--workspace`]",
        conflicts_with = "workspace",
        conflicts_with = "pkgid"
    )]
    all: bool,

    /// Modify all packages in the workspace.
    #[arg(long, conflicts_with = "all", conflicts_with = "pkgid")]
    workspace: bool,

    /// Print changes to be made without making them.
    #[arg(long)]
    dry_run: bool,

    /// Crates to exclude and not modify.
    #[arg(long)]
    exclude: Vec<String>,

    /// Run without accessing the network
    #[arg(long)]
    offline: bool,

    /// Require `Cargo.toml` to be up to date
    #[arg(long)]
    locked: bool,

    /// Unstable (nightly-only) flags
    #[arg(short = 'Z', value_name = "FLAG", global = true, value_enum)]
    unstable_features: Vec<UnstableOptions>,
}

impl VersionArgs {
    pub fn exec(self) -> CargoResult<()> {
        exec(self)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum UnstableOptions {}

/// Main processing function. Allows us to return a `Result` so that `main` can print pretty error
/// messages.
fn exec(args: VersionArgs) -> CargoResult<()> {
    let VersionArgs {
        target,
        bump,
        metadata,
        manifest_path,
        pkgid,
        all,
        dry_run,
        workspace,
        exclude,
        locked,
        offline,
        unstable_features: _,
    } = args;

    let target = match (target, bump) {
        (None, None) => TargetVersion::Relative(BumpLevel::Release),
        (None, Some(level)) => TargetVersion::Relative(level),
        (Some(version), None) => TargetVersion::Absolute(version),
        (Some(_), Some(_)) => unreachable!("clap groups should prevent this"),
    };

    let ws_metadata = resolve_ws(manifest_path.as_deref(), locked, offline)?;
    let root_manifest_path = ws_metadata.workspace_root.as_std_path().join("Cargo.toml");
    let workspace_members = find_ws_members(&ws_metadata);

    if all {
        shell_warn("The flag `--all` has been deprecated in favor of `--workspace`")?;
    }
    let all = workspace || all;
    let selected = if all {
        workspace_members
            .iter()
            .filter(|p| !exclude.contains(&p.name))
            .collect::<Vec<_>>()
    } else if pkgid.is_empty() {
        workspace_members
            .iter()
            .filter(|p| !exclude.contains(&p.name))
            .collect::<Vec<_>>()
    } else {
        workspace_members
            .iter()
            .filter(|p| pkgid.contains(&p.name))
            .collect::<Vec<_>>()
    };

    let update_workspace_version = all
        || selected.iter().any(|p| {
            LocalManifest::try_new(Path::new(&p.manifest_path))
                .map_or(false, |m| m.version_is_inherited())
        });
    if update_workspace_version {
        let mut ws_manifest = LocalManifest::try_new(&root_manifest_path)?;
        if let Some(current) = ws_manifest.get_workspace_version() {
            if let Some(next) = target.bump(&current, metadata.as_deref())? {
                shell_status(
                    "Upgrading",
                    &format!("workspace version from {} to {}", current, next),
                )?;
                ws_manifest.set_workspace_version(&next);
                if !dry_run {
                    ws_manifest.write()?;
                }

                // Deferring `update_dependents` to the per-package logic
            }
        }
    }

    for package in selected {
        let current = &package.version;
        let next = target.bump(current, metadata.as_deref())?;
        if let Some(next) = next {
            let mut manifest = LocalManifest::try_new(Path::new(&package.manifest_path))?;
            if manifest.version_is_inherited() {
                shell_status(
                    "Upgrading",
                    &format!(
                        "{} from {} to {} (inherited from workspace)",
                        package.name, current, next,
                    ),
                )?;
            } else {
                shell_status(
                    "Upgrading",
                    &format!("{} from {} to {}", package.name, current, next),
                )?;
                manifest.set_package_version(&next);
                if !dry_run {
                    manifest.write()?;
                }
            }

            let crate_root =
                dunce::canonicalize(package.manifest_path.parent().expect("at least a parent"))?;
            update_dependents(
                &crate_root,
                &next,
                &root_manifest_path,
                &workspace_members,
                dry_run,
            )?
        }
    }

    if dry_run {
        shell_warn("aborting set-version due to dry run")?;
    }

    Ok(())
}

fn update_dependents(
    crate_root: &Path,
    next: &semver::Version,
    root_manifest_path: &Path,
    workspace_members: &[cargo_metadata::Package],
    dry_run: bool,
) -> CargoResult<()> {
    // This is redundant with iterating over `workspace_members`
    // - As `get_dependency_tables_mut` returns workspace dependencies
    // - If there is a root package
    //
    // But split this out for
    // - Virtual manifests
    // - Nicer message to the user
    {
        update_dependent(crate_root, next, root_manifest_path, "workspace", dry_run)?;
    }

    for member in workspace_members.iter() {
        update_dependent(
            crate_root,
            next,
            member.manifest_path.as_std_path(),
            &member.name,
            dry_run,
        )?;
    }

    Ok(())
}

fn is_relevant(d: &dyn toml_edit::TableLike, dep_crate_root: &Path, crate_root: &Path) -> bool {
    if !d.contains_key("version") {
        return false;
    }
    match d
        .get("path")
        .and_then(|i| i.as_str())
        .and_then(|relpath| dunce::canonicalize(dep_crate_root.join(relpath)).ok())
    {
        Some(dep_path) => dep_path == crate_root,
        None => false,
    }
}

fn update_dependent(
    crate_root: &Path,
    next: &semver::Version,
    manifest_path: &Path,
    name: &str,
    dry_run: bool,
) -> CargoResult<()> {
    let mut dep_manifest = LocalManifest::try_new(manifest_path)?;
    let mut changed = false;
    let dep_crate_root = dep_manifest
        .path
        .parent()
        .expect("at least a parent")
        .to_owned();

    for dep in dep_manifest
        .get_dependency_tables_mut()
        .flat_map(|t| t.iter_mut().filter_map(|(_, d)| d.as_table_like_mut()))
        .filter(|d| is_relevant(*d, &dep_crate_root, crate_root))
    {
        let old_req = dep
            .get("version")
            .expect("filter ensures this")
            .as_str()
            .unwrap_or("*");
        if let Some(new_req) = upgrade_requirement(old_req, next)? {
            shell_status(
                "Updating",
                &format!("{}'s dependency from {} to {}", name, old_req, new_req),
            )?;
            dep.insert("version", toml_edit::value(new_req));
            changed = true;
        }
    }
    if changed && !dry_run {
        dep_manifest.write()?;
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
