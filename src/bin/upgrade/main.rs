//! `cargo upgrade`
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts,
       trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces,
       unused_qualifications)]

extern crate docopt;
extern crate pad;
#[macro_use]
extern crate serde_derive;
extern crate toml;

use std::error::Error;
use std::io::{self, Write};
use std::process;

extern crate cargo_edit;
use cargo_edit::{get_latest_dependency, Manifest};

static USAGE: &'static str = r"
Upgrade all dependencies in a manifest file to the latest version.

Usage:
    cargo upgrade [--dependency <dep>...] [--manifest-path <path>]
    cargo upgrade (-h | --help)
    cargo upgrade (-V | --version)

Options:
    -d --dependency <dep>       Specific dependency to upgrade. If this option is used, only the
                                specified dependencies will be upgraded.
    --manifest-path <path>      Path to the manifest to upgrade.
    -h --help                   Show this help page.
    -V --version                Show version.

Dev, build, and all target dependencies will also be upgraded. Only dependencies from crates.io are
supported. Git/path dependencies will be ignored.
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
) -> Result<(), Box<Error>> {
    let manifest_path = manifest_path.as_ref().map(From::from);
    let mut manifest = Manifest::open(&manifest_path).unwrap();

    for (table_path, table) in manifest.get_sections() {
        for (name, old_value) in &table {
            if (only_update.is_empty() || only_update.contains(name)) &&
                is_version_dependency(old_value)
            {
                let latest_version = get_latest_dependency(name, false)?;

                manifest.update_table_entry(&table_path, &latest_version)?;
            }
        }
    }

    let mut file = Manifest::find_file(&manifest_path)?;
    manifest.write_to_file(&mut file)
}

fn main() {
    let args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.deserialize::<Args>())
        .unwrap_or_else(|err| err.exit());

    if args.flag_version {
        println!("cargo-upgrade version {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    if let Err(err) = update_manifest(&args.flag_manifest_path, &args.flag_dependency) {
        writeln!(
            io::stderr(),
            "Command failed due to unhandled error: {}\n",
            err
        ).unwrap();
        process::exit(1);
    }
}
