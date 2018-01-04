//! `cargo rm`
#![warn(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts,
        trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces,
        unused_qualifications)]

extern crate docopt;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;
extern crate termcolor;

use std::process;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

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
    -q --quiet              Do not print any output in case of success.
    -h --help               Show this help page.
    -V --version            Show version.

Remove a dependency from a Cargo.toml manifest file.
";

fn print_msg(name: &str, section: &str) {
    let mut output = StandardStream::stdout(ColorChoice::Auto);
    output
        .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
        .unwrap();
    print!("{:>12}", "Removing");
    output.reset().unwrap();
    println!(" {} from {}", name, section);
}

fn handle_rm(args: &Args) -> Result<()> {
    let manifest_path = args.flag_manifest_path.as_ref().map(From::from);
    let mut manifest = Manifest::open(&manifest_path)?;

    if !args.flag_quiet {
        print_msg(&args.arg_crate, args.get_section());
    }

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
