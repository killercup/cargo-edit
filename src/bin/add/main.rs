//! `cargo add`

#![deny(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts, trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces, unused_qualifications)]
#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate docopt;
extern crate toml;
extern crate semver;
extern crate rustc_serialize;
extern crate curl;
#[macro_use]
extern crate quick_error;

use std::error::Error;
use std::process;
use std::io::{self, Write};

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

Options:
    --manifest-path=<path>  Path to the manifest to add a dependency to.
    -h --help               Show this help page.
    --version               Show version.

This command allows you to add a dependency to a Cargo.toml manifest file.

Please note that Cargo treats versions like "1.2.3" as "^1.2.3" (and that "^1.2.3" is specified
as ">=1.2.3 and <2.0.0"). By default, `cargo add` will use this format, as it is the one that the
crates.io registry suggests. One goal of `cargo add` is to prevent you from using wildcard
dependencies (version set to "*").
"#;

fn handle_add(args: &Args) -> Result<(), Box<Error>> {
    let mut manifest = try!(Manifest::open(&args.flag_manifest_path.as_ref().map(|s| &s[..])));
    let deps = try!(args.parse_dependencies());

    for dep in deps {
        if let Err(err) = manifest.insert_into_table(&args.get_section(), &dep) {
            println!("Could not edit `Cargo.toml`.\n\nERROR: {}", err);
            return Err(From::from(err));
        }
    }

    let mut file = try!(Manifest::find_file(&args.flag_manifest_path.as_ref().map(|s| &s[..])));
    manifest.write_to_file(&mut file)
}

fn main() {
    let args = docopt::Docopt::new(USAGE)
                   .and_then(|d| d.decode::<Args>())
                   .unwrap_or_else(|err| err.exit());

    if args.flag_version {
        println!("cargo-add version {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    if let Err(err) = handle_add(&args) {
        write!(io::stderr(),
               "Command failed due to unhandled error: {}",
               err)
            .unwrap();
        process::exit(1);
    }
}
