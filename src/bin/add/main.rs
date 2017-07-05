//! `cargo add`

#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts, trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces, unused_qualifications)]

extern crate reqwest;
extern crate docopt;
extern crate toml;
#[macro_use]
extern crate serde_derive;
extern crate semver;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate quick_error;

use std::error::Error;
use std::io::{self, Write};
use std::process;

extern crate cargo_edit;
use cargo_edit::Manifest;

extern crate regex;

mod fetch;
mod args;
use args::Args;

static USAGE: &'static str = r#"
Usage:
    cargo add <crate> [--dev|--build|--optional] [--vers=<ver>|--git=<uri>|--path=<uri>] [options]
    cargo add <crates>... [--dev|--build|--optional] [options]
    cargo add (-h|--help)
    cargo add --version

Specify what crate to add:
    --vers <ver>            Specify the version to grab from the registry (crates.io).
                            You can also specify versions as part of the name, e.g
                            `cargo add bitflags@0.3.2`.
    --git <uri>             Specify a git repository to download the crate from.
    --path <uri>            Specify the path the crate should be loaded from.

Specify where to add the crate:
    -D --dev                Add crate as development dependency.
    -B --build              Add crate as build dependency.
    --optional              Add as an optional dependency (for use in features). This does not work
                            for `dev-dependencies` or `build-dependencies`.
    --target <target>       Add as dependency to the given target platform. This does not work
                            for `dev-dependencies` or `build-dependencies`.

Options:
    --upgrade=<method>      Choose method of semantic version upgrade. Must be one of
                            "none" (exact version), "patch" (`~` modifier), "minor"
                            (`^` modifier, default), or "all" (`>=`).
    --update-only           If the dependency already exists, it will have its version updated,
                            preserving all other fields. The dependency will not be added if absent.
    --manifest-path=<path>  Path to the manifest to add a dependency to.
    --allow-prerelease      Include prerelease versions when fetching from crates.io (e.g.
                            '0.6.0-alpha'). Defaults to false.
    -h --help               Show this help page.
    -V --version            Show version.

This command allows you to add a dependency to a Cargo.toml manifest file. If <crate> is a github
or gitlab repository URL, or a local path, `cargo add` will try to automatically get the crate name
and set the appropriate `--git` or `--path` value.

Please note that Cargo treats versions like "1.2.3" as "^1.2.3" (and that "^1.2.3" is specified
as ">=1.2.3 and <2.0.0"). By default, `cargo add` will use this format, as it is the one that the
crates.io registry suggests. One goal of `cargo add` is to prevent you from using wildcard
dependencies (version set to "*").
"#;

fn handle_add(args: &Args) -> Result<(), Box<Error>> {
    let manifest_path = args.flag_manifest_path.as_ref().map(From::from);
    let mut manifest = Manifest::open(&manifest_path)?;
    let deps = &args.parse_dependencies()?;

    deps.iter()
        .map(|dep| if args.flag_update_only {
            manifest.update_table_entry(&args.get_section(), dep)
        } else {
            manifest.insert_into_table(&args.get_section(), dep)
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            println!("Could not edit `Cargo.toml`.\n\nERROR: {}", err);
            err
        })?;

    let mut file = Manifest::find_file(&manifest_path)?;
    manifest.write_to_file(&mut file)
}

fn main() {
    let args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.deserialize::<Args>())
        .unwrap_or_else(|err| err.exit());

    if args.flag_version {
        println!("cargo-add version {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    if let Err(err) = handle_add(&args) {
        writeln!(
            io::stderr(),
            "Command failed due to unhandled error: {}\n",
            err
        ).unwrap();
        process::exit(1);
    }
}
