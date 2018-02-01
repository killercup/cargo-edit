//! Handle `cargo add` arguments

use cargo_edit::Dependency;
use cargo_edit::{get_crate_name_from_github, get_crate_name_from_gitlab, get_crate_name_from_path,
                 get_latest_dependency};
use semver;
use std::path::PathBuf;

use errors::*;

#[derive(Debug, Deserialize)]
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
    pub flag_path: Option<PathBuf>,
    /// Crate directory path
    pub flag_target: Option<String>,
    /// Optional dependency
    pub flag_optional: bool,
    /// `Cargo.toml` path
    pub flag_manifest_path: Option<PathBuf>,
    /// `--version`
    pub flag_version: bool,
    /// `---upgrade`
    pub flag_upgrade: Option<String>,
    /// '--fetch-prereleases'
    pub flag_allow_prerelease: bool,
    /// '--quiet'
    pub flag_quiet: bool,
    /// '--features'
    pub flag_features: Option<String>,
}

impl Args {
    /// Get dependency section
    pub fn get_section(&self) -> Vec<String> {
        if self.flag_dev {
            vec!["dev-dependencies".to_owned()]
        } else if self.flag_build {
            vec!["build-dependencies".to_owned()]
        } else if let Some(ref target) = self.flag_target {
            if target.is_empty() {
                panic!("Target specification may not be empty");
            }
            vec![
                "target".to_owned(),
                target.clone(),
                "dependencies".to_owned(),
            ]
        } else {
            vec!["dependencies".to_owned()]
        }
    }

    /// Build dependencies from arguments
    pub fn parse_dependencies(&self) -> Result<Vec<Dependency>> {
        if !self.arg_crates.is_empty() {
            let mut result = Vec::new();
            for arg_crate in &self.arg_crates {
                let le_crate = if crate_name_has_version(arg_crate) {
                    parse_crate_name_with_version(arg_crate)?
                } else {
                    get_latest_dependency(arg_crate, self.flag_allow_prerelease)?
                }.set_optional(self.flag_optional).set_features(self.flag_features.clone());

                result.push(le_crate);
            }
            return Ok(result);
        }

        if crate_name_has_version(&self.arg_crate) {
            return Ok(vec![
                parse_crate_name_with_version(&self.arg_crate)?.set_optional(self.flag_optional).set_features(self.flag_features.clone()),
            ]);
        }


        let dependency = if !crate_name_is_url_or_path(&self.arg_crate) {
            let dependency = Dependency::new(&self.arg_crate);
            if let Some(ref version) = self.flag_vers {
                semver::VersionReq::parse(version)
                    .chain_err(|| "Invalid dependency version requirement")?;
                dependency.set_version(version)
            } else if let Some(ref repo) = self.flag_git {
                dependency.set_git(repo)
            } else if let Some(ref path) = self.flag_path {
                dependency.set_path(path.to_str().unwrap())
            } else {
                let dep = get_latest_dependency(&self.arg_crate, self.flag_allow_prerelease)?;
                let v = format!(
                    "{prefix}{version}",
                    prefix = self.get_upgrade_prefix().unwrap_or(""),
                    // If version is unavailable `get_latest_dependency` must have
                    // returned `Err(FetchVersionError::GetVersion)`
                    version = dep.version().unwrap_or_else(|| unreachable!())
                );
                dep.set_version(&v)
            }
        } else {
            parse_crate_name_from_uri(&self.arg_crate)?
        }.set_optional(self.flag_optional).set_features(self.flag_features.clone());

        Ok(vec![dependency])
    }

    fn get_upgrade_prefix(&self) -> Option<&'static str> {
        self.flag_upgrade
            .clone()
            .and_then(|flag| match flag.to_uppercase().as_ref() {
                "NONE" => Some("="),
                "PATCH" => Some("~"),
                "MINOR" => Some("^"),
                "ALL" => Some(">="),
                _ => {
                    println!(
                        "WARN: cannot understand upgrade option \"{}\", using default",
                        flag
                    );
                    None
                }
            })
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
            flag_target: None,
            flag_optional: false,
            flag_manifest_path: None,
            flag_version: false,
            flag_upgrade: None,
            flag_allow_prerelease: false,
            flag_quiet: false,
            flag_features: None,
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
    // FIXME: how else can we check if the name is a (possibly invalid) path?
    name.contains('.') || name.contains('/') || name.contains('\\')
}

fn parse_crate_name_with_version(name: &str) -> Result<Dependency> {
    assert!(crate_name_has_version(name));

    let xs: Vec<_> = name.splitn(2, '@').collect();
    let (name, version) = (xs[0], xs[1]);
    semver::VersionReq::parse(version).chain_err(|| "Invalid crate version requirement")?;

    Ok(Dependency::new(name).set_version(version))
}

fn parse_crate_name_from_uri(name: &str) -> Result<Dependency> {
    if crate_name_is_github_url(name) {
        if let Ok(ref crate_name) = get_crate_name_from_github(name) {
            return Ok(Dependency::new(crate_name).set_git(name));
        }
    } else if crate_name_is_gitlab_url(name) {
        if let Ok(ref crate_name) = get_crate_name_from_gitlab(name) {
            return Ok(Dependency::new(crate_name).set_git(name));
        }
    } else if crate_name_is_path(name) {
        if let Ok(ref crate_name) = get_crate_name_from_path(name) {
            return Ok(Dependency::new(crate_name).set_path(name));
        }
    }

    Err(From::from(format!(
        "Unable to obtain crate informations from `{}`.\n",
        name
    )))
}

#[cfg(test)]
mod tests {
    use cargo_edit::Dependency;
    use super::*;

    #[test]
    fn test_dependency_parsing() {
        let args = Args {
            arg_crate: "demo".to_owned(),
            flag_vers: Some("0.4.2".to_owned()),
            ..Args::default()
        };

        assert_eq!(
            args.parse_dependencies().unwrap(),
            vec![Dependency::new("demo").set_version("0.4.2")]
        );
    }

    #[test]
    #[cfg(feature = "test-external-apis")]
    fn test_repo_as_arg_parsing() {
        let github_url = "https://github.com/killercup/cargo-edit/";
        let args_github = Args {
            arg_crate: github_url.to_owned(),
            ..Args::default()
        };
        assert_eq!(
            args_github.parse_dependencies().unwrap(),
            vec![Dependency::new("cargo-edit").set_git(github_url)]
        );

        let gitlab_url = "https://gitlab.com/Polly-lang/Polly.git";
        let args_gitlab = Args {
            arg_crate: gitlab_url.to_owned(),
            ..Args::default()
        };
        assert_eq!(
            args_gitlab.parse_dependencies().unwrap(),
            vec![Dependency::new("polly").set_git(gitlab_url)]
        );
    }

    #[test]
    fn test_path_as_arg_parsing() {
        let self_path = ".";
        let args_path = Args {
            arg_crate: self_path.to_owned(),
            ..Args::default()
        };
        assert_eq!(
            args_path.parse_dependencies().unwrap(),
            vec![Dependency::new("cargo-edit").set_path(self_path)]
        );
    }

}
