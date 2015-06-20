#![cfg_attr(test, allow(dead_code))]

extern crate docopt;
extern crate rustc_serialize;
extern crate semver;
extern crate toml;

use std::error::Error;
use std::process;

#[macro_use] mod utils;
mod args;
mod manifest;

use args::Args;
use manifest::Manifest;

static USAGE: &'static str = "
Usage:
    cargo edit <section> add [options] <dep>...
    cargo edit <section> add [options] <dep> (--version | --path | --git) <source>
    cargo edit <section> add -h | --help

Options:
    --manifest-path PATH    Path to the manifest to add a dependency to.
    -h --help               Show this help page.

Edit a crate's dependencies by changing the Cargo.toml file.

If no source is specified, the source will be set to a wild-card version
dependency from the source's default crate registry.

If a version is specified, it will be validated as a valid semantic version
requirement. No other kind of source will be validated, and the registry will
not be polled to guarantee that a crate meeting that version requirement
actually exists.
";

fn main() {
    let args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.decode::<Args>())
        .unwrap_or_else(|err| err.exit());

    let deps: Vec<manifest::Dependency> = args.arg_dep.iter()
        .filter_map(|dep| Args::parse_dependency(dep, &args).ok())
        .collect();

    let mut manifest = Manifest::open(&args.flag_manifest_path.as_ref())
        .unwrap();

    let table = Args::parse_section(&args);

    manifest.add_deps(&table, &deps)
    .and_then(|_| {
        let mut file = try!(Manifest::find_file(&args.flag_manifest_path.as_ref()));
        manifest.write_to_file(&mut file)
    })
    .or_else(|err| -> Result<(), Box<Error>> {
        println!("Could not edit `Cargo.toml`.\n\nERROR: {}", err);
        process::exit(1);
    }).ok();
}
