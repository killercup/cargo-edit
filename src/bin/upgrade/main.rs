//! `cargo upgrade`
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts,
        trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces,
        unused_qualifications)]

extern crate cargo_metadata;
extern crate docopt;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;
extern crate toml_edit;

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process;

extern crate cargo_edit;
use cargo_edit::{get_latest_dependency, Dependency, Manifest};

extern crate termcolor;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

mod errors {
    error_chain!{
        links {
            CargoEditLib(::cargo_edit::Error, ::cargo_edit::ErrorKind);
        }

        foreign_links {
            // cargo-metadata doesn't (yet) export `ErrorKind`
            Metadata(::cargo_metadata::Error);
        }
    }
}
use errors::*;

static USAGE: &'static str = r"
Upgrade all dependencies in a manifest file to the latest version.

Usage:
    cargo upgrade [--all] [--dependency <dep>...] [--manifest-path <path>] [options]
    cargo upgrade (-h | --help)
    cargo upgrade (-V | --version)

Options:
    --all                       Upgrade all packages in the workspace.
    -d --dependency <dep>       Specific dependency to upgrade. If this option is used, only the
                                specified dependencies will be upgraded.
    --manifest-path <path>      Path to the manifest to upgrade.
    --allow-prerelease          Include prerelease versions when fetching from crates.io (e.g.
                                '0.6.0-alpha'). Defaults to false.
    --dry-run                   Print changes to be made without making them. Defaults to false.
    -h --help                   Show this help page.
    -V --version                Show version.

Dev, build, and all target dependencies will also be upgraded. Only dependencies from crates.io are
supported. Git/path dependencies will be ignored.

All packages in the workspace will be upgraded if the `--all` flag is supplied. The `--all` flag may
be supplied in the presence of a virtual manifest.
";

/// Docopts input args.
#[derive(Debug, Deserialize)]
struct Args {
    /// `--dependency -d <dep>`
    flag_dependency: Vec<String>,
    /// `--manifest-path <path>`
    flag_manifest_path: Option<String>,
    /// `--version`
    flag_version: bool,
    /// `--all`
    flag_all: bool,
    /// `--allow-prerelease`
    flag_allow_prerelease: bool,
    /// `--dry-run`
    flag_dry_run: bool,
}

fn is_version_dependency(dep: &toml_edit::Item) -> bool {
    dep["git"].is_none() && dep["path"].is_none()
}

/// Upgrade the specified manifest. Use the closure provided to get the new dependency versions.
fn upgrade_manifest_using_dependencies<F>(
    manifest_path: &Option<String>,
    only_update: &[String],
    dry_run: bool,
    new_dependency: F,
) -> Result<()>
where
    F: Fn(&String) -> cargo_edit::Result<Dependency>,
{
    let manifest_path = manifest_path.as_ref().map(From::from);
    let mut manifest = Manifest::open(&manifest_path)?;

    if dry_run {
        let bufwtr = BufferWriter::stdout(ColorChoice::Always);
        let mut buffer = bufwtr.buffer();
        buffer
            .set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))
            .chain_err(|| "Failed to set output colour")?;
        write!(&mut buffer, "Starting dry run. ")
            .chain_err(|| "Failed to write dry run message")?;
        buffer
            .set_color(&ColorSpec::new())
            .chain_err(|| "Failed to clear output colour")?;
        writeln!(&mut buffer, "Changes will not be saved.")
            .chain_err(|| "Failed to write dry run message")?;
        bufwtr
            .print(&buffer)
            .chain_err(|| "Failed to print dry run message")?;
    }

    for (table_path, table) in manifest.get_sections() {
        let table_like = table.as_table_like().expect("bug in get_sections");
        for (name, old_value) in table_like.iter() {
            let owned = name.to_owned();
            if (only_update.is_empty() || only_update.contains(&owned))
                && is_version_dependency(old_value)
            {
                let latest_version = new_dependency(&owned)?;

                manifest.update_table_entry(&table_path, &latest_version, dry_run)?;
            }
        }
    }

    let mut file = Manifest::find_file(&manifest_path)?;
    manifest
        .write_to_file(&mut file)
        .chain_err(|| "Failed to write new manifest contents")
}

fn upgrade_manifest(
    manifest_path: &Option<String>,
    only_update: &[String],
    allow_prerelease: bool,
    dry_run: bool,
) -> Result<()> {
    upgrade_manifest_using_dependencies(manifest_path, only_update, dry_run, |name| {
        get_latest_dependency(name, allow_prerelease)
    })
}

fn upgrade_manifest_from_cache(
    manifest_path: &Option<String>,
    only_update: &[String],
    new_deps: &HashMap<String, Dependency>,
    dry_run: bool,
) -> Result<()> {
    upgrade_manifest_using_dependencies(
        manifest_path,
        only_update,
        dry_run,
        |name| Ok(new_deps[name].clone()),
    )
}

/// Get a list of the paths of all the (non-virtual) manifests in the workspace.
fn get_workspace_manifests(manifest_path: &Option<String>) -> Result<Vec<String>> {
    Ok(
        cargo_metadata::metadata_deps(manifest_path.as_ref().map(Path::new), true)
            .chain_err(|| "Failed to get metadata")?
            .packages
            .iter()
            .map(|p| p.manifest_path.clone())
            .collect(),
    )
}

/// Look up all current direct crates.io dependencies in the workspace. Then get the latest version
/// for each.
fn get_new_workspace_deps(
    manifest_path: &Option<String>,
    only_update: &[String],
    allow_prerelease: bool,
) -> Result<HashMap<String, Dependency>> {
    let mut new_deps = HashMap::new();

    cargo_metadata::metadata_deps(manifest_path.as_ref().map(|p| Path::new(p)), true)
        .chain_err(|| "Failed to get metadata")?
        .packages
        .iter()
        .flat_map(|package| package.dependencies.to_owned())
        .filter(|dependency| {
            only_update.is_empty() || only_update.contains(&dependency.name)
        })
        .map(|dependency| {
            if !new_deps.contains_key(&dependency.name) {
                new_deps.insert(
                    dependency.name.clone(),
                    get_latest_dependency(&dependency.name, allow_prerelease)?,
                );
            }
            Ok(())
        })
        .collect::<Result<Vec<()>>>()?;

    Ok(new_deps)
}

fn upgrade_workspace_manifests(
    manifest_path: &Option<String>,
    only_update: &[String],
    allow_prerelease: bool,
    dry_run: bool,
) -> Result<()> {
    let new_deps = get_new_workspace_deps(manifest_path, only_update, allow_prerelease)?;

    get_workspace_manifests(manifest_path).and_then(|manifests| {
        for manifest in manifests {
            upgrade_manifest_from_cache(&Some(manifest), only_update, &new_deps, dry_run)?
        }

        Ok(())
    })
}

fn main() {
    let args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.deserialize::<Args>())
        .unwrap_or_else(|err| err.exit());

    if args.flag_version {
        println!("cargo-upgrade version {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    let output = if args.flag_all {
        upgrade_workspace_manifests(
            &args.flag_manifest_path,
            &args.flag_dependency,
            args.flag_allow_prerelease,
            args.flag_dry_run,
        )
    } else {
        upgrade_manifest(
            &args.flag_manifest_path,
            &args.flag_dependency,
            args.flag_allow_prerelease,
            args.flag_dry_run,
        )
    };

    if let Err(err) = output {
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
