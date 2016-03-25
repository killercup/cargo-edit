///! Handle `cargo add` arguments
use semver;
use std::error::Error;
use std::path::Path;
use cargo_edit::Dependency;
use fetch::{get_crate_name_from_github, get_crate_name_from_gitlab, get_crate_name_from_path,
            get_latest_version};

macro_rules! toml_table {
    ($($key:expr => $value:expr),+) => {
        {
            let mut dep = BTreeMap::new();
            $(dep.insert(String::from($key), $value);)+
            toml::Value::Table(dep)
        }
    }
}

#[derive(Debug, RustcDecodable)]
/// Docopts input args.
pub struct Args {
    /// Crate name (usage 1)
    pub arg_crate: String,
    /// Crate names (usage 2)
    pub arg_crates: Vec<String>,
    /// dev-dependency
    pub flag_dev: bool,
    /// build-dependency
    pub flag_build: bool,
    /// Version
    pub flag_vers: Option<String>,
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

    /// Build dependencies from arguments
    pub fn parse_dependencies(&self) -> Result<Vec<Dependency>, Box<Error>> {
        if !self.arg_crates.is_empty() {
            let mut result = Vec::<Dependency>::new();
            for arg_crate in &self.arg_crates {
                let le_crate = if crate_name_has_version(&arg_crate) {
                                   try!(parse_crate_name_with_version(arg_crate))
                               } else {
                                   let v = try!(get_latest_version(&self.arg_crate));
                                   Dependency::new(arg_crate).set_version(&v)
                               }
                               .set_optional(self.flag_optional);

                result.push(le_crate);
            }
            return Ok(result);
        }

        if crate_name_has_version(&self.arg_crate) {
            return Ok(vec![try!(parse_crate_name_with_version(&self.arg_crate))
                               .set_optional(self.flag_optional)]);
        }


        let dependency = if !crate_name_is_url_or_path(&self.arg_crate) {
                             let dependency = Dependency::new(&self.arg_crate);

                             if let Some(ref version) = self.flag_vers {
                                 try!(semver::VersionReq::parse(&version));
                                 dependency.set_version(version)
                             } else if let Some(ref repo) = self.flag_git {
                                 dependency.set_git(repo)
                             } else if let Some(ref path) = self.flag_path {
                                 dependency.set_path(path)
                             } else {
                                 let v = try!(get_latest_version(&self.arg_crate));
                                 dependency.set_version(&v)
                             }
                         } else {
                             try!(parse_crate_name_from_uri(&self.arg_crate))
                         }
                         .set_optional(self.flag_optional);

        Ok(vec![dependency])
    }
}

impl Default for Args {
    fn default() -> Args {
        Args {
            arg_crate: "demo".to_owned(),
            arg_crates: vec![],
            flag_dev: false,
            flag_build: false,
            flag_vers: None,
            flag_git: None,
            flag_path: None,
            flag_optional: false,
            flag_manifest_path: None,
            flag_version: false,
        }
    }
}

fn crate_name_has_version(name: &str) -> bool {
    name.contains('@')
}

fn crate_name_is_url_or_path(name: &str) -> bool {
    crate_name_is_github_url(name) || crate_name_is_gitlab_url(name) || crate_name_is_path(name)
}

fn crate_name_is_github_url(name: &str) -> bool {
    name.contains("https://github.com")
}

fn crate_name_is_gitlab_url(name: &str) -> bool {
    name.contains("https://gitlab.com")
}

fn crate_name_is_path(name: &str) -> bool {
    // FIXME: how else can we check if the name is a path?
    name.contains('.') || name.contains('/') || name.contains('\\') 
}

fn parse_crate_name_with_version(name: &str) -> Result<Dependency, Box<Error>> {
    assert!(crate_name_has_version(&name));

    let xs: Vec<&str> = name.splitn(2, '@').collect();
    let (name, version) = (xs[0], xs[1]);
    try!(semver::VersionReq::parse(&version));

    Ok(Dependency::new(name).set_version(version))
}

fn parse_crate_name_from_uri(name: &str) -> Result<Dependency, Box<Error>> {
    if crate_name_is_github_url(name) {
        if let Ok(ref crate_name) = get_crate_name_from_github(name) {
            return Ok(Dependency::new(crate_name).set_git(name));
        }
    } else if crate_name_is_gitlab_url(name) {
        if let Ok(ref crate_name) = get_crate_name_from_gitlab(name) {
            return Ok(Dependency::new(crate_name).set_git(name));
        }
    } else if crate_name_is_path(name) {
        if let Ok(ref crate_name) = get_crate_name_from_path(&name) {
            return Ok(Dependency::new(crate_name).set_path(name));
        }
    }

    Err(From::from(format!("Unable to obtain crate informations from `{}`.\n", name)))
}

#[cfg(test)]
mod tests {
    use std::env;
    use cargo_edit::Dependency;
    use super::*;

    #[test]
    fn test_dependency_parsing() {
        let args = Args {
            arg_crate: "demo".to_owned(),
            flag_vers: Some("0.4.2".to_owned()),
            ..Args::default()
        };

        assert_eq!(args.parse_dependencies().unwrap(),
                   vec![Dependency::new("demo").set_version("0.4.2")]);
    }

    #[test]
    fn test_repo_as_arg_parsing() {
        // Skip remote tests if no network available
        if env::var("NO_REMOTE_CARGO_TEST").is_err() {
            let github_url = "https://github.com/killercup/cargo-edit/";
            let args_github = Args { arg_crate: github_url.to_owned(), ..Args::default() };
            assert_eq!(args_github.parse_dependencies().unwrap(),
                       vec![Dependency::new("cargo-edit").set_git(github_url)]);

            let gitlab_url = "https://gitlab.com/Polly-lang/Polly.git";
            let args_gitlab = Args { arg_crate: gitlab_url.to_owned(), ..Args::default() };
            assert_eq!(args_gitlab.parse_dependencies().unwrap(),
                       vec![Dependency::new("polly").set_git(gitlab_url)]);
        }
    }

    #[test]
    fn test_path_as_arg_parsing() {
        let self_path = ".";
        let args_path = Args { arg_crate: self_path.to_owned(), ..Args::default() };
        assert_eq!(args_path.parse_dependencies().unwrap(),
                   vec![Dependency::new("cargo-edit").set_path(self_path)]);
    }

}
