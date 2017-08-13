//! `cargo upgrade`
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts,
       trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces,
       unused_qualifications)]

extern crate docopt;
extern crate pad;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate toml;

use std::error::Error;
use std::io::{self, Write};
use std::process::{self, Command};

extern crate cargo_edit;
use cargo_edit::{get_latest_dependency, Manifest};

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
}

fn is_version_dependency(dep: &toml::Value) -> bool {
    if let Some(table) = dep.as_table() {
        !table.contains_key("git") && !table.contains_key("path")
    } else {
        true
    }
}

fn update_manifest(
    manifest_path: &Option<String>,
    only_update: &[String],
    allow_prerelease: bool,
) -> Result<(), Box<Error>> {
    let manifest_path = manifest_path.as_ref().map(From::from);
    let mut manifest = Manifest::open(&manifest_path).unwrap();

    for (table_path, table) in manifest.get_sections() {
        for (name, old_value) in &table {
            if (only_update.is_empty() || only_update.contains(name)) &&
                is_version_dependency(old_value)
            {
                let latest_version = get_latest_dependency(name, allow_prerelease)?;

                manifest.update_table_entry(&table_path, &latest_version)?;
            }
        }
    }

    let mut file = Manifest::find_file(&manifest_path)?;
    manifest.write_to_file(&mut file)
}

/// Get a list of the paths of all the (non-virtual) manifests in the workspace.
fn get_workspace_manifests(manifest_path: &Option<String>) -> Result<Vec<String>, Box<Error>> {
    let mut metadata_gatherer = Command::new("cargo");
    metadata_gatherer.args(&["metadata", "--no-deps", "--format-version", "1", "-q"]);

    if let Some(ref manifest_path) = *manifest_path {
        metadata_gatherer.args(&["--manifest-path", manifest_path]);
    }

    let output = metadata_gatherer.output()?;

    if output.status.success() {
        let metadata: serde_json::Value =
            serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;

        let workspace_members = metadata["packages"]
            .as_array()
            .ok_or("No packages in workspace")?;

        workspace_members
            .iter()
            .map(|package| {
                package["manifest_path"]
                    .as_str()
                    .map(Into::into)
                    .ok_or_else(|| "Invalid manifest path".into())
            })
            .collect()
    } else {
        Err(
            format!(
                "Failed to get metadata: {}",
                String::from_utf8_lossy(&output.stderr)
            ).into(),
        )
    }
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
        get_workspace_manifests(&args.flag_manifest_path).and_then(|manifests| {
            for manifest in manifests {
                update_manifest(&Some(manifest), &args.flag_dependency, args.flag_allow_prerelease)?
            }

            Ok(())
        })
    } else {
        update_manifest(&args.flag_manifest_path, &args.flag_dependency, args.flag_allow_prerelease)
    };

    if let Err(err) = output {
        writeln!(
            io::stderr(),
            "Command failed due to unhandled error: {}\n",
            err
        ).unwrap();
        process::exit(1);
    }
}
