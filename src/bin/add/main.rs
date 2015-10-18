//! `cargo add`

#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

#![deny(missing_docs)]

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

mod fetch_version;
mod args;
use args::Args;

static USAGE: &'static str = r"
Usage:
    cargo add <crate> [--dev|--build|--optional] [--ver=<semver>|--git=<uri>|--path=<uri>] [options]
    cargo add (-h|--help)
    cargo add --version

Options:
    -D --dev                Add crate as development dependency.
    -B --build              Add crate as build dependency.
    --ver=<semver>          Specify the version to grab from the registry (crates.io).
                            You can also specify versions as part of the name, e.g
                            `cargo add bitflags@0.3.2`.
    --git=<uri>             Specify a git repository to download the crate from.
    --path=<uri>            Specify the path the crate should be loaded from.
    --optional              Add as an optional dependency (for use in features.)
    --manifest-path=<path>  Path to the manifest to add a dependency to.
    -h --help               Show this help page.
    --version               Show version.

Add a dependency to a Cargo.toml manifest file.
";

fn handle_add(args: &Args) -> Result<(), Box<Error>> {
    let mut manifest = try!(Manifest::open(&args.flag_manifest_path.as_ref().map(|s| &s[..])));
    let dep = try!(args.parse_dependency());

    manifest.insert_into_table(&args.get_section(), &dep)
            .map_err(From::from)
            .and_then(|_| {
                let mut file = try!(Manifest::find_file(&args.flag_manifest_path
                                                             .as_ref()
                                                             .map(|s| &s[..])));
                manifest.write_to_file(&mut file)
            })
            .or_else(|err| {
                println!("Could not edit `Cargo.toml`.\n\nERROR: {}", err);
                Err(err)
            })
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
