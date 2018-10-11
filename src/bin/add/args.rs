//! Handle `cargo add` arguments

use cargo_edit::Dependency;
use cargo_edit::{get_latest_dependency, CrateName};
use semver;
use std::path::PathBuf;
use cargo;
use git2;
use serde_json;
use tempdir;

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
    /// '--registry'
    pub flag_registry: Option<String>,
}

fn parse_version_req(s: &str) -> Result<&str> {
    semver::VersionReq::parse(s).chain_err(|| "Invalid dependency version requirement")?;
    Ok(s)
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
        let registry = self.flag_registry.clone();

        if !self.arg_crates.is_empty() {
            return self.arg_crates
                .iter()
                .map(|crate_name| {
                    Ok(
                        if let Some(krate) = CrateName::new(crate_name).parse_as_version()? {
                            krate
                        } else {
                            get_latest_dependency(crate_name, self.flag_allow_prerelease, registry.clone())?
                        }.set_optional(self.flag_optional).set_registry(registry.clone())
                    )
                })
                .collect();
        }

        let crate_name = CrateName::new(&self.arg_crate);

        let dependency = if let Some(dependency) = crate_name.parse_as_version()? {
            if let Some(ref url) = self.flag_git {
                let url = url.clone();
                let version = dependency.version().unwrap().to_string();
                Err(ErrorKind::GitUrlWithVersion(url, version))?;
            }
            if let Some(ref path) = self.flag_path {
                dependency.set_path(path.to_str().unwrap())
            } else {
                dependency
            }
        } else if !crate_name.is_url_or_path() {
            let dependency = Dependency::new(&self.arg_crate);

            match (&self.flag_git, &self.flag_vers, &self.flag_path) {
                (&Some(ref repo), &Some(ref version), _) => {
                    let repo = repo.clone();
                    let version = version.to_string();
                    return Err(ErrorKind::GitUrlWithVersion(repo, version))?;
                }
                (&Some(ref repo), &None, &Some(ref path)) => {
                    let repo = repo.clone();
                    let path = path.to_string_lossy().to_string();
                    return Err(ErrorKind::GitUrlWithPath(repo, path))?;
                }
                (&Some(ref repo), &None, &None) => dependency.set_git(repo),
                (&None, &Some(ref version), &Some(ref path)) => dependency
                    .set_version(parse_version_req(version)?)
                    .set_path(path.to_str().unwrap()),
                (&None, &Some(ref version), &None) => {
                    dependency.set_version(parse_version_req(version)?)
                }
                (&None, &None, &Some(ref path)) => dependency.set_path(path.to_str().unwrap()),
                (&None, &None, &None) => {
                    let registry = self.flag_registry.clone();
                    let dep = get_latest_dependency(&self.arg_crate, self.flag_allow_prerelease, registry.clone())?;
                    let v = format!(
                        "{prefix}{version}",
                        prefix = self.get_upgrade_prefix().unwrap_or(""),
                        // If version is unavailable `get_latest_dependency` must have
                        // returned `Err(FetchVersionError::GetVersion)`
                        version = dep.version().unwrap_or_else(|| unreachable!())
                    );
                    dep.set_version(&v).set_registry(registry)
                }
            }
        } else {
            crate_name.parse_crate_name_from_uri()?
        }.set_optional(self.flag_optional);

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
            flag_registry: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_edit::Dependency;

    #[test]
    fn test_dependency_parsing() {
        let args = Args {
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
