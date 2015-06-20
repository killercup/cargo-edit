use std::error::Error;
use std::collections::BTreeMap;

use semver;
use toml;

use manifest;

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

impl Default for Args {
    fn default() -> Args {
        Args {
            arg_section: String::from("dependencies"),
            arg_dep: vec![],
            arg_source: String::from(""),
            flag_manifest_path: None,
            flag_version: false,
            flag_git: false,
            flag_path: false,
        }
    }
}

impl Args {
    pub fn get_section(&self) -> String {
        let toml_field = match &(self.arg_section[..]) {
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
    pub fn parse_dependency(&self, dep: &String) -> Result<manifest::Dependency, Box<Error>> {
        if self.flag_version { Args::parse_semver(&self.arg_source) }
        else if self.flag_git { Args::parse_git(&self.arg_source) }
        else if self.flag_path { Args::parse_path(&self.arg_source) }
        else { Ok(toml::Value::String(String::from("*"))) }
        .map(|data| (dep.clone(), data))
    }

    pub fn get_dependencies(&self) -> Vec<manifest::Dependency> {
        self.arg_dep.iter()
            .filter_map(|dep| self.parse_dependency(dep).ok())
            .collect()
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
