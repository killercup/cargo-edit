use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use cargo_edit::{
    colorize_stderr, resolve_manifests, upgrade_requirement, workspace_members, LocalManifest,
};
use clap::Args;
use termcolor::{BufferWriter, Color, ColorSpec, WriteColor};

use crate::errors::*;
use crate::version::BumpLevel;
use crate::version::TargetVersion;

/// Change a package's version in the local manifest file (i.e. Cargo.toml).
#[derive(Debug, Args)]
#[clap(version)]
#[clap(group = clap::ArgGroup::new("ver").multiple(false))]
pub struct VersionArgs {
    /// Version to change manifests to
    #[clap(parse(try_from_str), group = "ver")]
    target: Option<semver::Version>,

    /// Increment manifest version
    #[clap(
        long,
        possible_values(crate::version::BumpLevel::variants()),
        group = "ver"
    )]
    bump: Option<BumpLevel>,

    /// Specify the version metadata field (e.g. a wrapped libraries version)
    #[clap(short, long)]
    pub metadata: Option<String>,

    /// Path to the manifest to upgrade
    #[clap(long, value_name = "PATH", parse(from_os_str))]
    manifest_path: Option<PathBuf>,

    /// Package id of the crate to change the version of.
    #[clap(
        long = "package",
        short = 'p',
        value_name = "PKGID",
        conflicts_with = "all",
        conflicts_with = "workspace"
    )]
    pkgid: Option<String>,

    /// Modify all packages in the workspace.
    #[clap(
        long,
        help = "[deprecated in favor of `--workspace`]",
        conflicts_with = "workspace",
        conflicts_with = "pkgid"
    )]
    all: bool,

    /// Modify all packages in the workspace.
    #[clap(long, conflicts_with = "all", conflicts_with = "pkgid")]
    workspace: bool,

    /// Print changes to be made without making them.
    #[clap(long)]
    dry_run: bool,

    /// Crates to exclude and not modify.
    #[clap(long)]
    exclude: Vec<String>,

    /// Unstable (nightly-only) flags
    #[clap(short = 'Z', value_name = "FLAG", global = true, arg_enum)]
    unstable_features: Vec<UnstableOptions>,
}

impl VersionArgs {
    pub fn exec(self) -> CargoResult<()> {
        exec(self)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ArgEnum)]
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
        unstable_features: _,
    } = args;

    let target = match (target, bump) {
        (None, None) => TargetVersion::Relative(BumpLevel::Release),
        (None, Some(level)) => TargetVersion::Relative(level),
        (Some(version), None) => TargetVersion::Absolute(version),
        (Some(_), Some(_)) => unreachable!("clap groups should prevent this"),
    };

    if all {
        deprecated_message("The flag `--all` has been deprecated in favor of `--workspace`")?;
    }
    let all = workspace || all;
    let manifests = Manifests(resolve_manifests(
        manifest_path.as_deref(),
        all,
        pkgid.as_deref(),
    )?);

    if dry_run {
        dry_run_message()?;
    }

    let workspace_members = workspace_members(manifest_path.as_deref())?;

    for package in manifests.0 {
        if exclude.contains(&package.name) {
            continue;
        }
        let current = &package.version;
        let next = target.bump(current, metadata.as_deref())?;
        if let Some(next) = next {
            {
                let mut manifest = LocalManifest::try_new(Path::new(&package.manifest_path))?;
                manifest.set_package_version(&next);

                upgrade_message(package.name.as_str(), current, &next)?;
                if !dry_run {
                    manifest.write()?;
                }
            }

            let crate_root =
                dunce::canonicalize(package.manifest_path.parent().expect("at least a parent"))?;
            for member in workspace_members.iter() {
                let mut dep_manifest = LocalManifest::try_new(member.manifest_path.as_std_path())?;
                let mut changed = false;
                let dep_crate_root = dep_manifest
                    .path
                    .parent()
                    .expect("at least a parent")
                    .to_owned();
                for dep in dep_manifest
                    .get_dependency_tables_mut()
                    .flat_map(|t| t.iter_mut().filter_map(|(_, d)| d.as_table_like_mut()))
                    .filter(|d| {
                        if !d.contains_key("version") {
                            return false;
                        }
                        match d.get("path").and_then(|i| i.as_str()).and_then(|relpath| {
                            dunce::canonicalize(dep_crate_root.join(relpath)).ok()
                        }) {
                            Some(dep_path) => dep_path == crate_root.as_path(),
                            None => false,
                        }
                    })
                {
                    let old_req = dep
                        .get("version")
                        .expect("filter ensures this")
                        .as_str()
                        .unwrap_or("*");
                    if let Some(new_req) = upgrade_requirement(old_req, &next)? {
                        upgrade_dependent_message(member.name.as_str(), old_req, &new_req)?;
                        dep.insert("version", toml_edit::value(new_req));
                        changed = true;
                    }
                }
                if changed && !dry_run {
                    dep_manifest.write()?;
                }
            }
        }
    }

    Ok(())
}

/// A collection of manifests.
struct Manifests(Vec<cargo_metadata::Package>);

fn dry_run_message() -> CargoResult<()> {
    let colorchoice = colorize_stderr();
    let bufwtr = BufferWriter::stderr(colorchoice);
    let mut buffer = bufwtr.buffer();
    buffer
        .set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))
        .with_context(|| "Failed to set output colour")?;
    write!(&mut buffer, "Starting dry run. ").with_context(|| "Failed to write dry run message")?;
    buffer
        .set_color(&ColorSpec::new())
        .with_context(|| "Failed to clear output colour")?;
    writeln!(&mut buffer, "Changes will not be saved.")
        .with_context(|| "Failed to write dry run message")?;
    bufwtr
        .print(&buffer)
        .with_context(|| "Failed to print dry run message")
}

fn deprecated_message(message: &str) -> CargoResult<()> {
    let colorchoice = colorize_stderr();
    let bufwtr = BufferWriter::stderr(colorchoice);
    let mut buffer = bufwtr.buffer();
    buffer
        .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))
        .with_context(|| "Failed to set output colour")?;
    writeln!(&mut buffer, "{}", message).with_context(|| "Failed to write dry run message")?;
    buffer
        .set_color(&ColorSpec::new())
        .with_context(|| "Failed to clear output colour")?;
    bufwtr
        .print(&buffer)
        .with_context(|| "Failed to print dry run message")
}

fn upgrade_message(name: &str, from: &semver::Version, to: &semver::Version) -> CargoResult<()> {
    let colorchoice = colorize_stderr();
    let bufwtr = BufferWriter::stderr(colorchoice);
    let mut buffer = bufwtr.buffer();
    buffer
        .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
        .with_context(|| "Failed to print dry run message")?;
    write!(&mut buffer, "{:>12}", "Upgraded").with_context(|| "Failed to print dry run message")?;
    buffer
        .reset()
        .with_context(|| "Failed to print dry run message")?;
    writeln!(&mut buffer, " {} from {} to {}", name, from, to)
        .with_context(|| "Failed to print dry run message")?;
    bufwtr
        .print(&buffer)
        .with_context(|| "Failed to print dry run message")
}

fn upgrade_dependent_message(name: &str, old_req: &str, new_req: &str) -> CargoResult<()> {
    let colorchoice = colorize_stderr();
    let bufwtr = BufferWriter::stderr(colorchoice);
    let mut buffer = bufwtr.buffer();
    buffer
        .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
        .with_context(|| "Failed to print dry run message")?;
    write!(&mut buffer, "{:>16}", "Updated dependency")
        .with_context(|| "Failed to print dry run message")?;
    buffer
        .reset()
        .with_context(|| "Failed to print dry run message")?;
    writeln!(&mut buffer, " {} from {} to {}", name, old_req, new_req)
        .with_context(|| "Failed to print dry run message")?;
    bufwtr
        .print(&buffer)
        .with_context(|| "Failed to print dry run message")
}
