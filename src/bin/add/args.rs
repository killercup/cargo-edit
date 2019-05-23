//! Handle `cargo add` arguments

use cargo_edit::Dependency;
use cargo_edit::{get_latest_dependency, CrateName};
use semver;
use std::path::PathBuf;
use structopt::StructOpt;

use crate::errors::*;

#[derive(Debug, StructOpt)]
#[structopt(bin_name = "cargo")]
pub enum ArgsWrap {
    #[structopt(name = "add", author = "")]
    #[structopt(
        after_help = "This command allows you to add a dependency to a Cargo.toml manifest file. If <crate> is a github
or gitlab repository URL, or a local path, `cargo add` will try to automatically get the crate name
and set the appropriate `--git` or `--path` value.

Please note that Cargo treats versions like '1.2.3' as '^1.2.3' (and that '^1.2.3' is specified
as '>=1.2.3 and <2.0.0'). By default, `cargo add` will use this format, as it is the one that the
crates.io registry suggests. One goal of `cargo add` is to prevent you from using wildcard
dependencies (version set to '*')."
    )]
    /// Add dependency to a Cargo.toml manifest file.
    Add(Args),
}

#[derive(Debug, StructOpt)]
pub struct Args {
    #[structopt(name = "crate", raw(required = "true"))]
    /// Crates to be added
    pub crates: Vec<String>,

    #[structopt(long = "dev", short = "D", conflicts_with = "build")]
    /// Add crate as development dependency
    pub dev: bool,

    #[structopt(long = "build", short = "B", conflicts_with = "dev")]
    /// Add crate as build dependency
    pub build: bool,

    #[structopt(
        long = "vers",
        value_name = "uri",
        conflicts_with = "git",
        conflicts_with = "path"
    )]
    /// Specify the version to grab from the registry(crates.io).
    /// You can also specify version as part of name, e.g
    /// `cargo add bitflags@0.3.2`.
    pub vers: Option<String>,

    #[structopt(
        long = "git",
        value_name = "uri",
        conflicts_with = "vers",
        conflicts_with = "path"
    )]
    /// Specify a git repository to download the crate from.
    pub git: Option<String>,

    #[structopt(long = "path", conflicts_with = "git", conflicts_with = "vers")]
    /// Specify the path the crate should be loaded from.
    pub path: Option<PathBuf>,

    #[structopt(long = "target", conflicts_with = "dev", conflicts_with = "build")]
    /// Add as dependency to the given target platform.
    pub target: Option<String>,

    #[structopt(long = "optional", conflicts_with = "dev", conflicts_with = "build")]
    /// Add as an optional dependency (for use in features).
    pub optional: bool,

    #[structopt(long = "manifest-path", value_name = "path")]
    /// Path to the manifest to add a dependency to.
    pub manifest_path: Option<PathBuf>,

    #[structopt(
        long = "upgrade",
        value_name = "method",
        possible_value = "none",
        possible_value = "patch",
        possible_value = "minor",
        possible_value = "all",
        possible_value = "default",
        default_value = "default"
    )]
    /// Choose method of semantic version upgrade.
    pub upgrade: String,

    #[structopt(long = "allow-prerelease")]
    /// Include prerelease versions when fetching from crates.io (e.g.
    /// '0.6.0-alpha').
    pub allow_prerelease: bool,

    #[structopt(long = "no-default-features")]
    /// Set `default-features = false` for the added dependency.
    pub no_default_features: bool,

    #[structopt(long = "quiet", short = "q")]
    /// Do not print any output in case of success.
    pub quiet: bool,
}

fn parse_version_req(s: &str) -> Result<&str> {
    semver::VersionReq::parse(s).chain_err(|| "Invalid dependency version requirement")?;
    Ok(s)
}

impl Args {
    /// Get dependency section
    pub fn get_section(&self) -> Vec<String> {
        if self.dev {
            vec!["dev-dependencies".to_owned()]
        } else if self.build {
            vec!["build-dependencies".to_owned()]
        } else if let Some(ref target) = self.target {
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

    fn parse_single_dependency(&self, crate_name: &str) -> Result<Dependency> {
        let crate_name = CrateName::new(crate_name);

        if let Some(mut dependency) = crate_name.parse_as_version()? {
            // crate specifier includes a version (e.g. `docopt@0.8`)
            if let Some(ref url) = self.git {
                let url = url.clone();
                let version = dependency.version().unwrap().to_string();
                return Err(ErrorKind::GitUrlWithVersion(url, version).into());
            }

            if let Some(ref path) = self.path {
                dependency = dependency.set_path(path.to_str().unwrap());
            }

            Ok(dependency)
        } else if crate_name.is_url_or_path() {
            Ok(crate_name.parse_crate_name_from_uri()?)
        } else {
            assert_eq!(self.git.is_some() && self.vers.is_some(), false);
            assert_eq!(self.git.is_some() && self.path.is_some(), false);

            let mut dependency = Dependency::new(crate_name.name());

            if let Some(repo) = &self.git {
                dependency = dependency.set_git(repo);
            }
            if let Some(path) = &self.path {
                dependency = dependency.set_path(path.to_str().unwrap());
            }
            if let Some(version) = &self.vers {
                dependency = dependency.set_version(parse_version_req(version)?);
            }

            if self.git.is_none() && self.path.is_none() && self.vers.is_none() {
                let dep = get_latest_dependency(crate_name.name(), self.allow_prerelease)?;
                let v = format!(
                    "{prefix}{version}",
                    prefix = self.get_upgrade_prefix(),
                    // If version is unavailable `get_latest_dependency` must have
                    // returned `Err(FetchVersionError::GetVersion)`
                    version = dep.version().unwrap_or_else(|| unreachable!())
                );
                dependency = dep.set_version(&v);
            }

            Ok(dependency)
        }
    }

    /// Build dependencies from arguments
    pub fn parse_dependencies(&self) -> Result<Vec<Dependency>> {
        if self.crates.len() > 1 && (self.git.is_some() || self.path.is_some()) {
            return Err(ErrorKind::MutiCrateWithGitOrPath.into());
        }

        self.crates
            .iter()
            .map(|crate_name| {
                self.parse_single_dependency(crate_name).map(|x| {
                    x.set_optional(self.optional)
                        .set_default_features(!self.no_default_features)
                })
            })
            .collect()
    }

    fn get_upgrade_prefix(&self) -> &'static str {
        match self.upgrade.as_ref() {
            "default" => "",
            "none" => "=",
            "patch" => "~",
            "minor" => "^",
            "all" => ">=",
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
impl Default for Args {
    fn default() -> Args {
        Args {
            crates: vec!["demo".to_owned()],
            dev: false,
            build: false,
            vers: None,
            git: None,
            path: None,
            target: None,
            optional: false,
            manifest_path: None,
            upgrade: "minor".to_string(),
            allow_prerelease: false,
            no_default_features: false,
            quiet: false,
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
            vers: Some("0.4.2".to_owned()),
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
            crates: vec![github_url.to_owned()],
            ..Args::default()
        };
        assert_eq!(
            args_github.parse_dependencies().unwrap(),
            vec![Dependency::new("cargo-edit").set_git(github_url)]
        );

        let gitlab_url = "https://gitlab.com/Polly-lang/Polly.git";
        let args_gitlab = Args {
            crates: vec![gitlab_url.to_owned()],
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
            crates: vec![self_path.to_owned()],
            ..Args::default()
        };
        assert_eq!(
            args_path.parse_dependencies().unwrap(),
            vec![Dependency::new("cargo-edit").set_path(self_path)]
        );
    }
}
