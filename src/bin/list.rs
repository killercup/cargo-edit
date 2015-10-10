#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate docopt;
extern crate rustc_serialize;

use std::error::Error;
use std::process;

extern crate cargo_edit;
use cargo_edit::{Manifest, list_section, list_tree};

static USAGE: &'static str = r"
Usage:
    cargo list [<section>] [options]
    cargo list (-h|--help)
    cargo list --version

Options:
    --manifest-path=<path>  Path to the manifest to add a dependency to.
    --tree                  List dependencies recursively as tree.
    -h --help               Show this help page.

Display a crate's dependencies using its Cargo.toml file.
";

#[derive(Debug, RustcDecodable)]
/// Docopts input args.
struct Args {
    arg_section: Option<String>,
    flag_manifest_path: Option<String>,
    flag_tree: bool,
    flag_version: bool,
}

impl Args {
    pub fn get_section(&self) -> String {
        let section = self.arg_section.clone().unwrap_or("dependencies".to_owned());

        let toml_field = match &(section[..]) {
            // Handle shortcuts
            "deps" => "dependencies",
            "dev-deps" => "dev-dependencies",
            "build-deps" => "build-dependencies",
            // No shortcut
            field => field
        };

        String::from(toml_field)
    }
}

fn handle_list(args: &Args) -> Result<(), Box<Error>> {
    let listing = if args.flag_tree {
        let manifest = try!(Manifest::open_lock_file(&args.flag_manifest_path.as_ref()));
        list_tree(&manifest)
    } else {
        let manifest = try!(Manifest::open(&args.flag_manifest_path.as_ref()));
        list_section(&manifest, &args.get_section())
    };

    listing
    .map(|listing| println!("{}", listing) )
    .or_else(|err| {
        println!("Could not list your stuff.\n\nERROR: {}", err);
        Err(err)
    })
}

fn main() {
    let args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.decode::<Args>())
        .unwrap_or_else(|err| err.exit());

    if args.flag_version {
        println!("cargo-list version {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    if let Err(err) = handle_list(&args) {
        println!("{}", err);
        process::exit(1);
    }
}
