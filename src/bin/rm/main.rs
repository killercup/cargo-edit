//! `cargo rm`

#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

#![deny(missing_docs)]

extern crate docopt;
extern crate toml;
extern crate semver;
extern crate rustc_serialize;

use std::error::Error;
use std::process;

extern crate cargo_edit;
use cargo_edit::Manifest;

mod args;
use args::Args;

static USAGE: &'static str = r"
Usage:
    cargo rm <crate> [--dev|--build] [options]
    cargo rm (-h|--help)
    cargo rm --version

Options:
    -D --dev                Remove crate as development dependency.
    -B --build              Remove crate as build dependency.
    --manifest-path=<path>  Path to the manifest to remove a dependency from.
    -h --help               Show this help page.
    --version               Show version.

Remove a dependency to a Cargo.toml manifest file.
";

fn handle_rm(args: &Args) -> Result<(), Box<Error>> {
    let mut manifest = try!(Manifest::open(&args.flag_manifest_path.as_ref()));

    manifest.remove_from_table(&args.get_section(), &args.arg_crate.as_ref())
            .map_err(From::from)
            .and_then(|_| {
                let mut file = try!(Manifest::find_file(&args.flag_manifest_path.as_ref()));
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
        println!("cargo-rm version {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    if let Err(err) = handle_rm(&args) {
        println!("{}", err);
        process::exit(1);
    }
}
