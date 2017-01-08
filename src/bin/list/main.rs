//! `cargo add`

#![deny(missing_docs, missing_debug_implementations, missing_copy_implementations, trivial_casts, trivial_numeric_casts, unsafe_code, unstable_features, unused_import_braces, unused_qualifications)]
#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate docopt;
extern crate rustc_serialize;
extern crate pad;
extern crate toml;
#[macro_use]
extern crate quick_error;

use std::error::Error;
use std::io::{self, Write};
use std::process;

extern crate cargo_edit;
use cargo_edit::Manifest;

mod list;
mod list_error;
mod tree;

use list::list_section;
use list_error::ListError;
use tree::parse_lock_file as list_tree;

static USAGE: &'static str = r"
Usage:
    cargo list [--dev|--build] [options]
    cargo list --tree
    cargo list (-h|--help)
    cargo list --version

Options:
    --manifest-path=<path>  Path to the manifest to list dependencies of.
    --tree                  List dependencies recursively as tree.
    -h --help               Show this help page.
    --version               Show version.

Display a crate's dependencies using its Cargo.toml file.
";

/// Docopts input args.
#[derive(Debug, RustcDecodable)]
struct Args {
    /// dev-dependency
    flag_dev: bool,
    /// build-dependency
    flag_build: bool,
    /// Render tree of dependencies
    flag_tree: bool,
    /// `Cargo.toml` path
    flag_manifest_path: Option<String>,
    /// `--version`
    flag_version: bool,
}

impl Args {
    /// Get dependency section
    fn get_section(&self) -> &'static str {
        if self.flag_dev {
            "dev-dependencies"
        } else if self.flag_build {
            "build-dependencies"
        } else {
            "dependencies"
        }
    }
}

fn handle_list(args: &Args) -> Result<String, Box<Error>> {
    if args.flag_tree {
            let manifest = try!(Manifest::open_lock_file(&args.flag_manifest_path
                .as_ref()
                .map(|s| &s[..])));
            list_tree(&manifest)
        } else {
            let manifest = try!(Manifest::open(&args.flag_manifest_path.as_ref().map(|s| &s[..])));
            list_section(&manifest, args.get_section()).or_else(|err| match err {
                ListError::SectionMissing(..) => Ok("".into()),
                _ => Err(err),
            })
        }
        .map_err(From::from)
}

fn main() {
    println!("\n\n    WARNING: cargo list is deprecated and slated to be removed when cargo-edit reaches 0.2\n\n");
    let args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.decode::<Args>())
        .unwrap_or_else(|err| err.exit());

    if args.flag_version {
        println!("cargo-list version {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    match handle_list(&args) {
        Ok(list) => {
            println!("{}", list);
        }
        Err(err) => {
            write!(io::stderr(), "{}\n", err).unwrap();
            process::exit(1);
        }
    }
}
