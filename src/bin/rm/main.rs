//! `cargo rm`
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts,
        trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces,
        unused_qualifications)]

extern crate docopt;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;

use std::io::{self, Write};
use std::process;

extern crate cargo_edit;
use cargo_edit::Manifest;

mod args;
use args::Args;

mod errors {
    error_chain!{
        links {
            CargoEditLib(::cargo_edit::Error, ::cargo_edit::ErrorKind);
        }
    }
}
use errors::*;

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
    -V --version            Show version.

Remove a dependency from a Cargo.toml manifest file.
";

fn handle_rm(args: &Args) -> Result<()> {
    let manifest_path = args.flag_manifest_path.as_ref().map(From::from);
    let mut manifest = Manifest::open(&manifest_path)?;

    manifest
        .remove_from_table(args.get_section(), args.arg_crate.as_ref())
        .map_err(From::from)
        .and_then(|_| {
            let mut file = Manifest::find_file(&manifest_path)?;
            manifest.write_to_file(&mut file)?;

            Ok(())
        })
}

fn main() {
    let args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.deserialize::<Args>())
        .unwrap_or_else(|err| err.exit());

    if args.flag_version {
        println!("cargo-rm version {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    if let Err(err) = handle_rm(&args) {
        let mut stderr = io::stderr();

        writeln!(stderr, "Command failed due to unhandled error: {}\n", err).unwrap();

        for e in err.iter().skip(1) {
            writeln!(stderr, "Caused by: {}", e).unwrap();
        }

        if let Some(backtrace) = err.backtrace() {
            writeln!(stderr, "Backtrace: {:?}", backtrace).unwrap();
        }

        process::exit(1);
    }
}
