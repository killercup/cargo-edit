use std::error::Error;
use std::collections::BTreeMap;

use rustc_serialize;
use semver;
use toml;

use manifest;

macro_rules! toml_table {
    ($key:expr => $value:expr) => {
        {
            let mut dep = BTreeMap::new();
            dep.insert(String::from($key), toml::Value::String($value.clone()));
            toml::Value::Table(dep)
        }
    }
}

#[derive(Debug, RustcDecodable)]
/// Docopts input args.
pub struct Args {
    pub arg_section: String,
    pub arg_dep: Vec<String>,
    pub arg_source: String,
    pub flag_manifest_path: Option<String>,
    pub flag_version: bool,
    pub flag_git: bool,
    pub flag_path: bool,
}

impl Args {
    pub fn parse_sections(args: &Args) -> String {
        let toml_field = match &args.arg_section[..] {
            // Handle shortcuts
            "deps" => "dependencies",
            "dev-deps" => "dev-dependencies",
            "build-deps" => "build-dependencies",
            // No shortcut
            field => field
        };

        String::from(toml_field)
    }

    /// Parse command-line input into key/value data that can be added to the TOML.
    pub fn parse_dependency(dep: &String, args: &Args) -> Result<manifest::Dependency, Box<Error>> {
        if args.flag_version { Args::parse_semver(&args.arg_source) }
        else if args.flag_git { Args::parse_git(&args.arg_source) }
        else if args.flag_path { Args::parse_path(&args.arg_source) }
        else { Ok(toml::Value::String(String::from("*"))) }
        .map(|data| (dep.clone(), data))
    }

    /// Parse (and validate) a version requirement to the correct TOML data.
    fn parse_semver(version: &String) -> Result<toml::Value, Box<Error>> {
        try!(semver::VersionReq::parse(version));
        Ok(toml::Value::String(version.clone()))
    }

    /// Parse a git source to the correct TOML data.
    fn parse_git(repo: &String) -> Result<toml::Value, Box<Error>> {
        Ok(toml_table!("git" => repo))
    }

    /// Parse a path to the correct TOML data.
    fn parse_path(path: &String) -> Result<toml::Value, Box<Error>> {
        Ok(toml_table!("path" => path))
    }
}
