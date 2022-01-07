use std::str::FromStr;

use cargo_edit::VersionExt;

use crate::{Error, ErrorKind};

#[derive(Clone, Debug)]
pub enum TargetVersion {
    Relative(BumpLevel),
    Absolute(semver::Version),
}

impl TargetVersion {
    pub fn bump(
        &self,
        current: &semver::Version,
        metadata: Option<&str>,
    ) -> Result<Option<semver::Version>, Error> {
        match self {
            TargetVersion::Relative(bump_level) => {
                let mut potential_version = current.to_owned();
                bump_level.bump_version(&mut potential_version, metadata)?;
                if potential_version != *current {
                    let version = potential_version;
                    Ok(Some(version))
                } else {
                    Ok(None)
                }
            }
            TargetVersion::Absolute(version) => {
                if current < version {
                    let mut version = version.clone();
                    if version.build.is_empty() {
                        if let Some(metadata) = metadata {
                            version.build = semver::BuildMetadata::new(metadata)?;
                        } else {
                            version.build = current.build.clone();
                        }
                    }

                    Ok(Some(version))
                } else if current == version {
                    Ok(None)
                } else {
                    Err(ErrorKind::VersionDowngreade(current.clone(), version.clone()).into())
                }
            }
        }
    }
}

impl Default for TargetVersion {
    fn default() -> Self {
        TargetVersion::Relative(BumpLevel::Release)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BumpLevel {
    Major,
    Minor,
    Patch,
    /// Strip all pre-release flags
    Release,
    Rc,
    Beta,
    Alpha,
}

impl BumpLevel {
    pub fn variants() -> &'static [&'static str] {
        &["major", "minor", "patch", "release", "rc", "beta", "alpha"]
    }
}

impl FromStr for BumpLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "major" => Ok(BumpLevel::Major),
            "minor" => Ok(BumpLevel::Minor),
            "patch" => Ok(BumpLevel::Patch),
            "release" => Ok(BumpLevel::Release),
            "rc" => Ok(BumpLevel::Rc),
            "beta" => Ok(BumpLevel::Beta),
            "alpha" => Ok(BumpLevel::Alpha),
            _ => Err(String::from(
                "[valid values: major, minor, patch, rc, beta, alpha]",
            )),
        }
    }
}

impl BumpLevel {
    pub fn bump_version(
        self,
        version: &mut semver::Version,
        metadata: Option<&str>,
    ) -> Result<(), Error> {
        match self {
            BumpLevel::Major => {
                version.increment_major();
            }
            BumpLevel::Minor => {
                version.increment_minor();
            }
            BumpLevel::Patch => {
                if !version.is_prerelease() {
                    version.increment_patch();
                } else {
                    version.pre = semver::Prerelease::EMPTY;
                }
            }
            BumpLevel::Release => {
                if version.is_prerelease() {
                    version.pre = semver::Prerelease::EMPTY;
                }
            }
            BumpLevel::Rc => {
                version.increment_rc()?;
            }
            BumpLevel::Beta => {
                version.increment_beta()?;
            }
            BumpLevel::Alpha => {
                version.increment_alpha()?;
            }
        };

        if let Some(metadata) = metadata {
            version.metadata(metadata)?;
        }

        Ok(())
    }
}
