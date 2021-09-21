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

#[macro_use]
extern crate error_chain;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

use cargo_edit::{find, manifest_from_pkgid, LocalManifest};
use structopt::StructOpt;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

mod args;
mod errors;
mod version;
use crate::args::*;
use crate::errors::*;

fn main() {
    let args = Command::from_args();
    let Command::Version(args) = args;

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

/// Main processing function. Allows us to return a `Result` so that `main` can print pretty error
/// messages.
fn process(args: Args) -> Result<()> {
    let Args {
        target,
        bump,
        metadata,
        manifest_path,
        pkgid,
        all,
        dry_run,
        workspace,
        exclude,
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
        Manifests::get_all(&manifest_path)
    } else if let Some(ref pkgid) = pkgid {
        Manifests::get_pkgid(pkgid)
    } else {
        Manifests::get_local_one(&manifest_path)
    }?;

    if dry_run {
        dry_run_message()?;
    }

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
        }
    }

    Ok(())
}

/// A collection of manifests.
struct Manifests(Vec<(LocalManifest, cargo_metadata::Package)>);

impl Manifests {
    /// Get all manifests in the workspace.
    fn get_all(manifest_path: &Option<PathBuf>) -> Result<Self> {
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
            .map(Manifests)
    }

    fn get_pkgid(pkgid: &str) -> Result<Self> {
        let package = manifest_from_pkgid(pkgid)?;
        let manifest = LocalManifest::try_new(Path::new(&package.manifest_path))?;
        Ok(Manifests(vec![(manifest, package)]))
    }

    /// Get the manifest specified by the manifest path. Try to make an educated guess if no path is
    /// provided.
    fn get_local_one(manifest_path: &Option<PathBuf>) -> Result<Self> {
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

        Ok(Manifests(vec![(manifest, package.to_owned())]))
    }
}

fn dry_run_message() -> Result<()> {
    let bufwtr = BufferWriter::stdout(ColorChoice::Always);
    let mut buffer = bufwtr.buffer();
    buffer
        .set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))
        .chain_err(|| "Failed to set output colour")?;
    write!(&mut buffer, "Starting dry run. ").chain_err(|| "Failed to write dry run message")?;
    buffer
        .set_color(&ColorSpec::new())
        .chain_err(|| "Failed to clear output colour")?;
    writeln!(&mut buffer, "Changes will not be saved.")
        .chain_err(|| "Failed to write dry run message")?;
    bufwtr
        .print(&buffer)
        .chain_err(|| "Failed to print dry run message")
}

fn deprecated_message(message: &str) -> Result<()> {
    let bufwtr = BufferWriter::stderr(ColorChoice::Always);
    let mut buffer = bufwtr.buffer();
    buffer
        .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))
        .chain_err(|| "Failed to set output colour")?;
    writeln!(&mut buffer, "{}", message).chain_err(|| "Failed to write dry run message")?;
    buffer
        .set_color(&ColorSpec::new())
        .chain_err(|| "Failed to clear output colour")?;
    bufwtr
        .print(&buffer)
        .chain_err(|| "Failed to print dry run message")
}

fn upgrade_message(name: &str, from: &semver::Version, to: &semver::Version) -> Result<()> {
    let bufwtr = BufferWriter::stdout(ColorChoice::Always);
    let mut buffer = bufwtr.buffer();
    buffer
        .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
        .chain_err(|| "Failed to print dry run message")?;
    write!(&mut buffer, "{:>12}", "Upgraded").chain_err(|| "Failed to print dry run message")?;
    buffer
        .reset()
        .chain_err(|| "Failed to print dry run message")?;
    writeln!(&mut buffer, " {} from {} to {}", name, from, to)
        .chain_err(|| "Failed to print dry run message")?;
    bufwtr
        .print(&buffer)
        .chain_err(|| "Failed to print dry run message")
}
