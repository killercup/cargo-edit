//! Handle `cargo add` arguments

#![allow(clippy::bool_assert_comparison)]

use cargo_edit::{
    find, get_manifest_from_path, get_manifest_from_url, registry_url, workspace_members,
    Dependency,
};
use cargo_edit::{get_latest_dependency, CrateName};
use cargo_metadata::Package;
use std::path::PathBuf;
use structopt::{clap::AppSettings, StructOpt};

use crate::errors::*;

#[derive(Debug, StructOpt)]
#[structopt(bin_name = "cargo")]
pub enum Command {
    /// Add dependency to a Cargo.toml manifest file.
    #[structopt(name = "add")]
    #[structopt(after_help = "\
This command allows you to add a dependency to a Cargo.toml manifest file. If <crate> is a github \
or gitlab repository URL, or a local path, `cargo add` will try to automatically get the crate \
name and set the appropriate `--git` or `--path` value.

Please note that Cargo treats versions like '1.2.3' as '^1.2.3' (and that '^1.2.3' is specified \
as '>=1.2.3 and <2.0.0'). By default, `cargo add` will use this format, as it is the one that the \
crates.io registry suggests. One goal of `cargo add` is to prevent you from using wildcard \
dependencies (version set to '*').")]
    Add(Args),
}

#[derive(Debug, StructOpt)]
#[structopt(setting = AppSettings::ColoredHelp)]
pub struct Args {
    /// Crates to be added.
    #[structopt(name = "crate", required = true)]
    pub crates: Vec<String>,

    /// Rename a dependency in Cargo.toml,
    /// https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml.
    /// Only works when specifying a single dependency.
    #[structopt(long = "rename", short = "r")]
    pub rename: Option<String>,

    /// Add crate as development dependency.
    #[structopt(long = "dev", short = "D", conflicts_with = "build")]
    pub dev: bool,

    /// Add crate as build dependency.
    #[structopt(long = "build", short = "B", conflicts_with = "dev")]
    pub build: bool,

    /// Specify the version to grab from the registry(crates.io).
    /// You can also specify version as part of name, e.g
    /// `cargo add bitflags@0.3.2`.
    #[structopt(long = "vers", value_name = "uri", conflicts_with = "git")]
    pub vers: Option<String>,

    /// Specify a git repository to download the crate from.
    #[structopt(
        long = "git",
        value_name = "uri",
        conflicts_with = "vers",
        conflicts_with = "path"
    )]
    pub git: Option<String>,

    /// Specify a git branch to download the crate from.
    #[structopt(
        long = "branch",
        value_name = "branch",
        conflicts_with = "vers",
        conflicts_with = "path"
    )]
    pub branch: Option<String>,

    /// Specify the path the crate should be loaded from.
    #[structopt(long = "path", conflicts_with = "git")]
    pub path: Option<PathBuf>,

    /// Add as dependency to the given target platform.
    #[structopt(long = "target", conflicts_with = "dev", conflicts_with = "build")]
    pub target: Option<String>,

    /// Add as an optional dependency (for use in features).
    #[structopt(long = "optional", conflicts_with = "dev")]
    pub optional: bool,

    /// Path to the manifest to add a dependency to.
    #[structopt(long = "manifest-path", value_name = "path", conflicts_with = "pkgid")]
    pub manifest_path: Option<PathBuf>,

    /// Package id of the crate to add this dependency to.
    #[structopt(
        long = "package",
        short = "p",
        value_name = "pkgid",
        conflicts_with = "path"
    )]
    pub pkgid: Option<String>,

    /// Choose method of semantic version upgrade.  Must be one of "none" (exact version, `=`
    /// modifier), "patch" (`~` modifier), "minor" (`^` modifier), "all" (`>=`), or "default" (no
    /// modifier).
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
    pub upgrade: String,

    /// Include prerelease versions when fetching from crates.io (e.g.
    /// '0.6.0-alpha').
    #[structopt(long = "allow-prerelease")]
    pub allow_prerelease: bool,

    /// Space-separated list of features to add. For an alternative approach to
    /// enabling features, consider installing the `cargo-feature` utility.
    #[structopt(long = "features", number_of_values = 1)]
    pub features: Option<Vec<String>>,

    /// Set `default-features = false` for the added dependency.
    #[structopt(long = "no-default-features")]
    pub no_default_features: bool,

    /// Do not print any output in case of success.
    #[structopt(long = "quiet", short = "q")]
    pub quiet: bool,

    /// Run without accessing the network
    #[structopt(long = "offline")]
    pub offline: bool,

    /// Sort dependencies even if currently unsorted
    #[structopt(long = "sort", short = "s")]
    pub sort: bool,

    /// Registry to use
    #[structopt(long = "registry", conflicts_with = "git", conflicts_with = "path")]
    pub registry: Option<String>,
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
            assert!(!target.is_empty(), "Target specification may not be empty");

            vec![
                "target".to_owned(),
                target.clone(),
                "dependencies".to_owned(),
            ]
        } else {
            vec!["dependencies".to_owned()]
        }
    }

    fn parse_single_dependency(
        &self,
        crate_name: &str,
        workspace_members: &[Package],
    ) -> Result<Dependency> {
        let crate_name = CrateName::new(crate_name);
        let registry_url = if let Some(registry) = &self.registry {
            Some(registry_url(&find(&self.manifest_path)?, Some(registry))?)
        } else {
            None
        };

        let mut dependency = if let Some(mut dependency) = crate_name.parse_as_version()? {
            // crate specifier includes a version (e.g. `docopt@0.8`)
            if let Some(ref url) = self.git {
                let url = url.clone();
                let version = dependency.version().unwrap().to_string();
                return Err(ErrorKind::GitUrlWithVersion(url, version).into());
            }

            if let Some(ref path) = self.path {
                let manifest = get_manifest_from_path(path)?;
                let dep_path = dunce::canonicalize(path)?;

                dependency = dependency.set_available_features(manifest.features()?);
                dependency = dependency.set_path(dep_path);
            }

            dependency
        } else if let Some(mut dependency) = crate_name.parse_crate_name_from_uri()? {
            // dev-dependencies do not need the version populated
            if !self.dev {
                let dep_path = dependency.path().map(ToOwned::to_owned);
                if let Some(dep_path) = dep_path {
                    if let Some(package) = workspace_members.iter().find(|p| {
                        p.manifest_path.parent().map(|p| p.as_std_path())
                            == Some(dep_path.as_path())
                    }) {
                        let v = format!(
                            "{prefix}{version}",
                            prefix = self.get_upgrade_prefix(),
                            version = package.version
                        );

                        dependency = dependency.set_version(&v);
                    }
                }
            }
            dependency
        } else {
            let mut dependency = Dependency::new(crate_name.name());

            if let Some(repo) = &self.git {
                assert!(self.vers.is_none());
                assert!(self.path.is_none());
                assert!(self.registry.is_none());
                let features = get_manifest_from_url(repo)?
                    .map(|m| m.features())
                    .transpose()?
                    .unwrap_or_else(Vec::new);

                dependency = dependency
                    .set_git(repo, self.branch.clone())
                    .set_available_features(features);
            } else {
                if let Some(path) = &self.path {
                    assert!(self.registry.is_none());
                    let manifest = get_manifest_from_path(path)?;

                    dependency = dependency.set_available_features(manifest.features()?);
                    dependency = dependency.set_path(dunce::canonicalize(path)?);
                }
                if let Some(version) = &self.vers {
                    dependency = dependency.set_version(parse_version_req(version)?);
                }

                if self.git.is_none() && self.path.is_none() && self.vers.is_none() {
                    // Only special-case workspaces when the user doesn't provide any extra
                    // information, otherwise, trust the user.
                    if let Some(package) = workspace_members
                        .iter()
                        .find(|p| p.name == crate_name.name())
                    {
                        dependency = dependency.set_path(
                            package
                                .manifest_path
                                .parent()
                                .expect("at least parent dir")
                                .as_std_path()
                                .to_owned(),
                        );
                        // dev-dependencies do not need the version populated
                        if !self.dev {
                            let v = format!(
                                "{prefix}{version}",
                                prefix = self.get_upgrade_prefix(),
                                version = package.version
                            );
                            dependency = dependency.set_version(&v);
                        }
                    } else {
                        dependency = get_latest_dependency(
                            crate_name.name(),
                            self.allow_prerelease,
                            &find(&self.manifest_path)?,
                            &registry_url,
                        )?;
                        let v = format!(
                            "{prefix}{version}",
                            prefix = self.get_upgrade_prefix(),
                            // If version is unavailable `get_latest_dependency` must have
                            // returned `Err(FetchVersionError::GetVersion)`
                            version = dependency.version().unwrap_or_else(|| unreachable!())
                        );
                        dependency = dependency.set_version(&v);
                    }
                }
            }

            dependency
        };

        if let Some(registry) = &self.registry {
            dependency = dependency.set_registry(registry);
        }

        Ok(dependency)
    }

    /// Build dependencies from arguments
    pub fn parse_dependencies(&self) -> Result<Vec<Dependency>> {
        let workspace_members = workspace_members(self.manifest_path.as_deref())?;

        if self.crates.len() > 1
            && (self.git.is_some() || self.path.is_some() || self.vers.is_some())
        {
            return Err(ErrorKind::MultipleCratesWithGitOrPathOrVers.into());
        }

        if self.crates.len() > 1 && self.rename.is_some() {
            return Err(ErrorKind::MultipleCratesWithRename.into());
        }

        if self.crates.len() > 1 && self.features.is_some() {
            return Err(ErrorKind::MultipleCratesWithFeatures.into());
        }

        self.crates
            .iter()
            .map(|crate_name| {
                self.parse_single_dependency(crate_name, &workspace_members)
                    .map(|x| {
                        let mut x = x
                            .set_optional(self.optional)
                            .set_features(self.features.clone())
                            .set_default_features(!self.no_default_features);
                        if let Some(ref rename) = self.rename {
                            x = x.set_rename(rename);
                        }
                        x
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
            rename: None,
            dev: false,
            build: false,
            vers: None,
            git: None,
            branch: None,
            path: None,
            target: None,
            optional: false,
            manifest_path: None,
            pkgid: None,
            upgrade: "minor".to_string(),
            allow_prerelease: false,
            features: None,
            no_default_features: false,
            quiet: false,
            offline: true,
            sort: false,
            registry: None,
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
            vec![Dependency::new("cargo-edit").set_git(github_url, None)]
        );

        let gitlab_url = "https://gitlab.com/Polly-lang/Polly.git";
        let args_gitlab = Args {
            crates: vec![gitlab_url.to_owned()],
            ..Args::default()
        };
        assert_eq!(
            args_gitlab.parse_dependencies().unwrap(),
            vec![Dependency::new("polly").set_git(gitlab_url, None)]
        );
    }

    #[test]
    fn test_path_as_arg_parsing() {
        let self_path = dunce::canonicalize(std::env::current_dir().unwrap()).unwrap();
        let args_path = Args {
            // Hacky to `display` but should generally work
            crates: vec![self_path.display().to_string()],
            ..Args::default()
        };
        assert_eq!(
            args_path.parse_dependencies().unwrap()[0].path().unwrap(),
            self_path
        );
    }
}
