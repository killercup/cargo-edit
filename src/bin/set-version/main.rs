//! `cargo version`
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
#![allow(clippy::comparison_chain)]

use std::io::Write;
use std::path::Path;
use std::process;

use cargo_edit::{
    colorize_stderr, find, manifest_from_pkgid, upgrade_requirement, workspace_members,
    LocalManifest,
};
use clap::Parser;
use termcolor::{BufferWriter, Color, ColorSpec, WriteColor};

mod args;
mod errors;
mod version;
use crate::args::*;
use crate::errors::*;

fn main() {
    let args = Command::parse();
    let Command::Version(args) = args;

    if let Err(err) = process(args) {
        eprintln!("Error: {:?}", err);

        process::exit(1);
    }
}

/// Main processing function. Allows us to return a `Result` so that `main` can print pretty error
/// messages.
fn process(args: VersionArgs) -> CargoResult<()> {
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
        (None, None) => version::TargetVersion::Relative(version::BumpLevel::Release),
        (None, Some(level)) => version::TargetVersion::Relative(level),
        (Some(version), None) => version::TargetVersion::Absolute(version),
        (Some(_), Some(_)) => unreachable!("clap groups should prevent this"),
    };

    if all {
        deprecated_message("The flag `--all` has been deprecated in favor of `--workspace`")?;
    }
    let all = workspace || all;
    let manifests = if all {
        Manifests::get_all(manifest_path.as_deref())
    } else if let Some(ref pkgid) = pkgid {
        Manifests::get_pkgid(manifest_path.as_deref(), pkgid)
    } else {
        Manifests::get_local_one(manifest_path.as_deref())
    }?;

    if dry_run {
        dry_run_message()?;
    }

    let workspace_members = workspace_members(manifest_path.as_deref())?;

    for (mut manifest, package) in manifests.0 {
        if exclude.contains(&package.name) {
            continue;
        }
        let current = &package.version;
        let next = target.bump(current, metadata.as_deref())?;
        if let Some(next) = next {
            manifest.set_package_version(&next);

            upgrade_message(package.name.as_str(), current, &next)?;
            if !dry_run {
                manifest.write()?;
            }

            let crate_root =
                dunce::canonicalize(manifest.path.parent().expect("at least a parent"))?;
            for member in workspace_members.iter() {
                let mut dep_manifest = LocalManifest::try_new(member.manifest_path.as_std_path())?;
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
                    }
                }
                if !dry_run {
                    dep_manifest.write()?;
                }
            }
        }
    }

    Ok(())
}

/// A collection of manifests.
struct Manifests(Vec<(LocalManifest, cargo_metadata::Package)>);

impl Manifests {
    /// Get all manifests in the workspace.
    fn get_all(manifest_path: Option<&Path>) -> CargoResult<Self> {
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
            .map(Manifests)
    }

    fn get_pkgid(manifest_path: Option<&Path>, pkgid: &str) -> CargoResult<Self> {
        let package = manifest_from_pkgid(manifest_path, pkgid)?;
        let manifest = LocalManifest::try_new(Path::new(&package.manifest_path))?;
        Ok(Manifests(vec![(manifest, package)]))
    }

    /// Get the manifest specified by the manifest path. Try to make an educated guess if no path is
    /// provided.
    fn get_local_one(manifest_path: Option<&Path>) -> CargoResult<Self> {
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

        Ok(Manifests(vec![(manifest, package.to_owned())]))
    }
}

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
