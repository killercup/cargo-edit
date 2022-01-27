//! Handle `cargo add` arguments

#![allow(clippy::bool_assert_comparison)]

use cargo_edit::{
    find, get_features_from_registry, get_manifest_from_url, registry_url, workspace_members,
    Dependency,
};
use cargo_edit::{get_latest_dependency, CrateSpec};
use cargo_metadata::Package;
use clap::Parser;
use std::path::PathBuf;

use crate::errors::*;

#[derive(Debug, Parser)]
#[clap(bin_name = "cargo")]
pub enum Command {
    /// Add dependency to a Cargo.toml manifest file.
    #[clap(name = "add")]
    #[clap(after_help = "\
This command allows you to add a dependency to a Cargo.toml manifest file. If <crate> is a github \
or gitlab repository URL, or a local path, `cargo add` will try to automatically get the crate \
name and set the appropriate `--git` or `--path` value.

Please note that Cargo treats versions like '1.2.3' as '^1.2.3' (and that '^1.2.3' is specified \
as '>=1.2.3 and <2.0.0'). By default, `cargo add` will use this format, as it is the one that the \
crates.io registry suggests. One goal of `cargo add` is to prevent you from using wildcard \
dependencies (version set to '*').")]
    Add(Args),
}

#[derive(Debug, Parser)]
#[clap(about, version)]
pub struct Args {
    /// Crates to be added.
    #[clap(value_name = "CRATE", required = true)]
    pub crates: Vec<String>,

    /// Rename a dependency in Cargo.toml,
    /// https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml.
    /// Only works when specifying a single dependency.
    #[clap(long, short)]
    pub rename: Option<String>,

    /// Add crate as development dependency.
    #[clap(long, short = 'D', group = "section")]
    pub dev: bool,

    /// Add crate as build dependency.
    #[clap(long, short = 'B', group = "section")]
    pub build: bool,

    /// Add as dependency to the given target platform.
    #[clap(long, forbid_empty_values = true, group = "section")]
    pub target: Option<String>,

    /// Specify the version to grab from the registry(crates.io).
    /// You can also specify version as part of name, e.g
    /// `cargo add bitflags:0.3.2`.
    #[clap(long, value_name = "URI", conflicts_with = "git")]
    pub vers: Option<String>,

    /// Specify a git repository to download the crate from.
    #[clap(long, value_name = "URI", conflicts_with = "vers")]
    pub git: Option<String>,

    /// Specify a git branch to download the crate from.
    #[clap(long, value_name = "BRANCH", requires = "git", group = "git-ref")]
    pub branch: Option<String>,

    /// Specify a git branch to download the crate from.
    #[clap(long, value_name = "TAG", requires = "git", group = "git-ref")]
    pub tag: Option<String>,

    /// Specify a git branch to download the crate from.
    #[clap(long, value_name = "REV", requires = "git", group = "git-ref")]
    pub rev: Option<String>,

    /// Add as an optional dependency (for use in features).
    #[clap(long, conflicts_with = "dev")]
    pub optional: bool,

    /// Path to the manifest to add a dependency to.
    #[clap(
        long,
        value_name = "PATH",
        parse(from_os_str),
        conflicts_with = "pkgid"
    )]
    pub manifest_path: Option<PathBuf>,

    /// Package id of the crate to add this dependency to.
    #[clap(
        long = "package",
        short = 'p',
        value_name = "PKGID",
        conflicts_with = "manifest-path"
    )]
    pub pkgid: Option<String>,

    /// Space-separated list of features to add. For an alternative approach to
    /// enabling features, consider installing the `cargo-feature` utility.
    #[clap(long)]
    pub features: Option<Vec<String>>,

    /// Set `default-features = false` for the added dependency.
    #[clap(long)]
    pub no_default_features: bool,

    /// Do not print any output in case of success.
    #[clap(long)]
    pub quiet: bool,

    /// Run without accessing the network
    #[clap(long)]
    pub offline: bool,

    /// Registry to use
    #[clap(long, conflicts_with = "git")]
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

    /// Build dependencies from arguments
    pub fn parse_dependencies(
        &self,
        requested_features: Option<Vec<String>>,
    ) -> Result<Vec<Dependency>> {
        let workspace_members = workspace_members(self.manifest_path.as_deref())?;

        if self.crates.len() > 1 && (self.git.is_some() || self.vers.is_some()) {
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
            .map(|crate_spec| {
                self.parse_single_dependency(crate_spec, &workspace_members)
                    .map(|x| {
                        let mut x = x
                            .set_optional(self.optional)
                            .set_features(requested_features.to_owned())
                            .set_default_features(!self.no_default_features);
                        if let Some(ref rename) = self.rename {
                            x = x.set_rename(rename);
                        }
                        x
                    })
            })
            .collect()
    }

    fn parse_single_dependency(
        &self,
        crate_spec: &str,
        workspace_members: &[Package],
    ) -> Result<Dependency> {
        let crate_spec = CrateSpec::resolve(crate_spec)?;
        let manifest_path = find(&self.manifest_path)?;
        let registry_url = registry_url(&manifest_path, self.registry.as_deref())?;

        let mut dependency = match &crate_spec {
            CrateSpec::PkgId {
                name: _,
                version_req: Some(_),
            } => {
                let mut dependency = crate_spec.to_dependency()?;
                // crate specifier includes a version (e.g. `docopt:0.8`)
                if let Some(ref url) = self.git {
                    let url = url.clone();
                    let version = dependency.version().unwrap().to_string();
                    return Err(ErrorKind::GitUrlWithVersion(url, version).into());
                }

                let features = get_features_from_registry(
                    &dependency.name,
                    dependency
                        .version()
                        .expect("version populated by `parse_as_version`"),
                    &registry_url,
                )?;
                dependency = dependency.set_available_features(features);

                dependency
            }
            CrateSpec::PkgId {
                name,
                version_req: None,
            } => {
                let mut dependency = crate_spec.to_dependency()?;

                if let Some(repo) = &self.git {
                    assert!(self.vers.is_none());
                    assert!(self.registry.is_none());
                    let features = get_manifest_from_url(repo)?
                        .map(|m| m.features())
                        .transpose()?
                        .unwrap_or_else(Vec::new);

                    dependency = dependency
                        .set_git(
                            repo,
                            self.branch.clone(),
                            self.tag.clone(),
                            self.rev.clone(),
                        )
                        .set_available_features(features);
                } else {
                    if let Some(version) = &self.vers {
                        dependency = dependency.set_version(parse_version_req(version)?);
                    }

                    if self.git.is_none() && self.vers.is_none() {
                        // Only special-case workspaces when the user doesn't provide any extra
                        // information, otherwise, trust the user.
                        if let Some(package) = workspace_members.iter().find(|p| p.name == *name) {
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
                                let op = "";
                                let v =
                                    format!("{op}{version}", op = op, version = package.version);
                                dependency = dependency.set_version(&v);
                            }
                        } else {
                            dependency = get_latest_dependency(
                                name,
                                false,
                                &manifest_path,
                                Some(&registry_url),
                            )?;
                            let op = "";
                            let v = format!(
                                "{op}{version}",
                                op = op,
                                // If version is unavailable `get_latest_dependency` must have
                                // returned `Err(FetchVersionError::GetVersion)`
                                version = dependency.version().unwrap_or_else(|| unreachable!())
                            );
                            dependency = dependency.set_version(&v);
                        }
                    }
                }

                dependency
            }
            CrateSpec::Path(_) => {
                let mut dependency = crate_spec.to_dependency()?;
                // dev-dependencies do not need the version populated
                if !self.dev {
                    let dep_path = dependency.path().map(ToOwned::to_owned);
                    if let Some(dep_path) = dep_path {
                        if let Some(package) = workspace_members.iter().find(|p| {
                            p.manifest_path.parent().map(|p| p.as_std_path())
                                == Some(dep_path.as_path())
                        }) {
                            let op = "";
                            let v = format!("{op}{version}", op = op, version = package.version);

                            dependency = dependency.set_version(&v);
                        }
                    }
                }
                dependency
            }
        };

        if let Some(registry) = &self.registry {
            dependency = dependency.set_registry(registry);
        }

        Ok(dependency)
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
            tag: None,
            rev: None,
            target: None,
            optional: false,
            manifest_path: None,
            pkgid: None,
            features: None,
            no_default_features: false,
            quiet: false,
            offline: true,
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
            args.parse_dependencies(None).unwrap(),
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
            args_github.parse_dependencies(None).unwrap(),
            vec![Dependency::new("cargo-edit").set_git(github_url, None)]
        );

        let gitlab_url = "https://gitlab.com/Polly-lang/Polly.git";
        let args_gitlab = Args {
            crates: vec![gitlab_url.to_owned()],
            ..Args::default()
        };
        assert_eq!(
            args_gitlab.parse_dependencies(None).unwrap(),
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
            args_path.parse_dependencies(None).unwrap()[0]
                .path()
                .unwrap(),
            self_path
        );
    }

    #[test]
    fn verify_app() {
        use clap::IntoApp;
        Command::into_app().debug_assert()
    }
}
