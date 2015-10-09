#![cfg_attr(test, allow(dead_code))]

#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate docopt;
extern crate rustc_serialize;
extern crate semver;
extern crate toml;
extern crate pad;

use std::error::Error;
use std::process;

#[macro_use] mod utils;
mod args;
mod manifest;
mod list;
mod list_error;
mod tree;
#[cfg(test)] mod manifest_test;

use args::{Args, Command};
use manifest::Manifest;

static USAGE: &'static str = "
Usage:
    cargo edit <section> <command> [options]
    cargo edit <section> <command> [options] <dep>...
    cargo edit <section> <command> [options] <dep> (--version | --path | --git) <source>
    cargo edit -h | --help

Options:
    --manifest-path PATH    Path to the manifest to add a dependency to.
    -h --help               Show this help page.

Available commands are:
    add         Add new dependency
    list        Show a list of all dependencies
    tree        Show a tree of all dependencies and their subdependencies

Edit/display a crate's dependencies using its Cargo.toml file.

If no source is specified, the source will be set to a wild-card version
dependency from the source's default crate registry.

If a version is specified, it will be validated as a valid semantic version
requirement. No other kind of source will be validated, and the registry will
not be polled to guarantee that a crate meeting that version requirement
actually exists.
";

fn handle_add(args: &Args) -> Result<(), Box<Error>> {
    let mut manifest = try!(Manifest::open(&args.flag_manifest_path.as_ref()));

    manifest.add_deps(&args.get_section(), &args.get_dependencies())
    .and_then(|_| {
        let mut file = try!(Manifest::find_file(&args.flag_manifest_path.as_ref()));
        manifest.write_to_file(&mut file)
    })
    .or_else(|err| {
        println!("Could not edit `Cargo.toml`.\n\nERROR: {}", err);
        Err(err)
    })
}

fn handle_list(args: &Args) -> Result<(), Box<Error>> {
    let manifest = try!(Manifest::open(&args.flag_manifest_path.as_ref()));

    list::list_section(&manifest, &args.get_section())
    .map(|listing| println!("{}", listing) )
    .or_else(|err| {
        println!("Could not list your stuff.\n\nERROR: {}", err);
        Err(err)
    })
}

fn handle_tree(args: &Args) -> Result<(), Box<Error>> {
    let manifest = try!(Manifest::open_lock_file(&args.flag_manifest_path.as_ref()));

    let output = try!(tree::parse_lock_file(&manifest));
    println!("{}", output);
    Ok(())
}

fn main() {
    let args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.decode::<Args>())
        .unwrap_or_else(|err| err.exit());

    let work = match args.arg_command {
        Command::List => handle_list(&args),
        Command::Tree => handle_tree(&args),
        Command::Add  => handle_add(&args),
    };

    work
    .or_else(|_| -> Result<(), Box<Error>> {
        process::exit(1);
    }).ok();
}
