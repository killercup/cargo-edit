///! Handle `cargo add` arguments

use semver;
use std::error::Error;
use cargo_edit::Dependency;
use fetch_version::get_latest_version;

macro_rules! toml_table {
    ($($key:expr => $value:expr),+) => {
        {
            let mut dep = BTreeMap::new();
            $(dep.insert(String::from($key), $value);)+
            toml::Value::Table(dep)
        }
    }
}

/// Errors when parsing CLI arguments
quick_error! {
    #[derive(Debug)]
    pub enum ArgParseError {
        /// Internal error parsing args to TOML
        BuildingDependency {
            description("Error build a dependency description from arguments")
        }
    }
}

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

        let dependency = Dependency::new(&self.arg_crate);

        let dependency = if let Some(ref version) = self.flag_ver {
            try!(semver::VersionReq::parse(&version));
            dependency.set_version(version)
        } else if let Some(ref repo) = self.flag_git {
            dependency.set_git(repo)
        } else if let Some(ref path) = self.flag_path {
            dependency.set_path(path)
        } else {
            let v = try!(get_latest_version(&self.arg_crate));
            dependency.set_version(&v)
        };

        Ok(dependency)
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

fn crate_name_has_version(name: &str) -> bool {
    name.contains("@")
}

fn parse_crate_name_with_version(name: &str) -> Result<Dependency, Box<Error>> {
    let xs: Vec<&str> = name.splitn(2, "@").collect();
    let (name, version) = (xs[0], xs[1]);
    try!(semver::VersionReq::parse(&version));

    Ok(Dependency::new(name).set_version(version))
}

#[cfg(test)]
mod tests {
    use cargo_edit::Dependency;
    use super::*;

    #[test]
    fn test_dependency_parsing() {
        let args = Args {
            arg_crate: "demo".to_owned(),
            flag_ver: Some("0.4.2".to_owned()),
            ..Args::default()
        };

        assert_eq!(args.parse_dependency().unwrap(),
                   Dependency::new("demo").set_version("0.4.2"));
    }
}
