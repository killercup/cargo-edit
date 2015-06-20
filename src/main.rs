extern crate docopt;
extern crate rustc_serialize;
extern crate semver;
extern crate toml;

use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{OpenOptions};
use std::io::{Read, Write};
use std::process;

mod manifest;

static USAGE: &'static str = "
Usage:
    cargo add [options] <dep>...
    cargo add [options] <dep> (--version | --path | --git) <source>
    cargo add -h | --help

Options:
    --manifest-path PATH    Path to the manifest to add a dependency to.
    -h --help               Show this help page.

Add a dependency to the crate's Cargo.toml file. If no source is specified, the
source will be set to a wild-card version dependency from the source's default
crate registry.

If a version is specified, it will be validated as a valid semantic version
requirement. No other kind of source will be validated, and the registry will
not be polled to guarantee that a crate meeting that version requirement
actually exists.
";

#[derive(Debug, RustcDecodable)]
//Docopts input args.
struct Args {
    arg_dep: Vec<String>,
    arg_source: String,
    flag_manifest_path: Option<String>,
    flag_version: bool,
    flag_git: bool,
    flag_path: bool,
}

// Parse command-line input into key/value data that can be added to the TOML.
fn parse_dependency(dep: &String, args: &Args) -> Result<manifest::Dependency, Box<Error>> {
    if args.flag_version { parse_semver(&args.arg_source) }
    else if args.flag_git { parse_git(&args.arg_source) }
    else if args.flag_path { parse_path(&args.arg_source) }
    else { Ok(toml::Value::String(String::from("*"))) }
    .map(|data| (dep.clone(), data))
}

// Parse (and validate) a version requirement to the correct TOML data.
fn parse_semver(version: &String) -> Result<toml::Value, Box<Error>> {
    try!(semver::VersionReq::parse(version));
    Ok(toml::Value::String(version.clone()))
}

// Parse a git source to the correct TOML data.
fn parse_git(repo: &String) -> Result<toml::Value, Box<Error>> {
    let mut dep = BTreeMap::new();
    dep.insert(String::from("git"), toml::Value::String(repo.clone()));
    Ok(toml::Value::Table(dep))
}

// Parse a path to the correct TOML data.
fn parse_path(path: &String) -> Result<toml::Value, Box<Error>> {
    let mut dep = BTreeMap::new();
    dep.insert(String::from("path"), toml::Value::String(path.clone()));
    Ok(toml::Value::Table(dep))
}

fn main() {
    //1. Generate an Args struct from the docopts string.
    docopt::Docopt::new(USAGE).and_then(|d| d.decode::<Args>()).or_else(|err| err.exit())
    //2. Generate a list of dependencies & a manifest file handle from the Args.
    //[Args -> (File, Vec<Dependency>)]
    .and_then(|args| {
        args.arg_dep.iter()
        .map(|dep| parse_dependency(dep, &args))
        .collect::<Result<Vec<_>, _>>()
        .and_then(|deps| {
            manifest::find_manifest(args.flag_manifest_path.as_ref())
            .and_then(|path| OpenOptions::new().read(true).write(true)
                                               .open(path).map_err(From::from))
            .map(|manifest| (manifest, deps))
        })
    })
    //3. Add the dependencies to the manifest. [(File, Vec<Dependency>) -> ()]
    .and_then(|(mut manifest, deps)| {
        manifest::read_as_toml(&mut manifest)
        .and_then(|mut toml_data| deps.into_iter()
                                 .map(|dep| manifest::insert_dependency(&mut toml_data, dep))
                                 .collect::<Result<Vec<_>, _>>()
                                 .map_err(From::from)
                                 .map(|_| toml_data))
        .and_then(|toml_data| manifest::write_from_toml(&mut manifest, toml_data))
    })
    //4. Print error message and return error code on failure.
    .or_else(|err| -> Result<(), Box<Error>> {
        println!("Could not add dependency.\n\nERROR: {}", err);
        process::exit(1);
    }).ok();
}
