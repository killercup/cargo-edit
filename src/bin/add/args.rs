///! Handle `cargo add` arguments

use std::collections::BTreeMap;
use std::error::Error;
use toml;
use semver;
use cargo_edit::Dependency;

#[derive(Debug, RustcDecodable)]
/// Docopts input args.
pub struct Args {
    /// Crate name
    pub arg_crate: String,
    /// dev-dependency
    pub flag_dev: bool,
    /// build-dependency
    pub flag_build: bool,
    /// Version
    pub flag_ver: Option<String>,
    /// Git repo Path
    pub flag_git: Option<String>,
    /// Crate directory path
    pub flag_path: Option<String>,
    /// Optional dependency
    pub flag_optional: bool,
    /// `Cargo.toml` path
    pub flag_manifest_path: Option<String>,
    /// `--version`
    pub flag_version: bool,
}

impl Args {
    /// Get depenency section
    pub fn get_section(&self) -> &'static str {
        if self.flag_dev {
            "dev-dependencies"
        } else if self.flag_build {
            "build-dependencies"
        } else {
            "dependencies"
        }
    }

    /// Build depenency from arguments
    pub fn parse_dependency(&self) -> Result<Dependency, Box<Error>> {
        if crate_name_has_version(&self.arg_crate) {
            return parse_crate_name_with_version(&self.arg_crate);
        }

        let version = if let Some(ref version) = self.flag_ver {
            parse_semver(&version)
        } else if let Some(ref repo) = self.flag_git {
            parse_git(&repo)
        } else if let Some(ref path) = self.flag_path {
            parse_path(&path)
        } else {
            Ok(toml::Value::String(String::from("*")))
        };

        version.map(|data| (self.arg_crate.clone(), data))
    }
}

impl Default for Args {
    fn default() -> Args {
        Args {
            arg_crate: "demo".to_owned(),
            flag_dev: false,
            flag_build: false,
            flag_ver: None,
            flag_git: None,
            flag_path: None,
            flag_optional: false,
            flag_manifest_path: None,
            flag_version: false,
        }
    }
}

macro_rules! toml_table {
    ($key:expr => $value:expr) => {
        {
            let mut dep = BTreeMap::new();
            dep.insert(String::from($key), toml::Value::String($value.clone()));
            toml::Value::Table(dep)
        }
    }
}

fn crate_name_has_version(name: &str) -> bool {
    name.contains("@")
}

fn parse_crate_name_with_version(name: &str) -> Result<Dependency, Box<Error>> {
    // if !crate_name_has_version(name) {
    //     return Err("fuu");
    // }

    let xs: Vec<&str> = name.splitn(2, "@").collect();
    let (name, version) = (xs[0], xs[1]);
    let version = try!(parse_semver(&version.to_owned()));

    Ok((String::from(name), version))
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

#[cfg(test)]
mod tests {
    use toml;
    use super::*;

    #[test]
    fn test_dependency_parsing() {
        let args = Args {
            arg_crate: "demo".to_owned(),
            flag_ver: Some("0.4.2".to_owned()),
            ..Args::default()
        };

        assert_eq!(args.parse_dependency().unwrap(),
                   ("demo".to_owned(), toml::Value::String("0.4.2".to_owned())));
    }
}
