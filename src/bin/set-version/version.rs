use std::str::FromStr;

use cargo_edit::VersionExt;

use crate::errors::{CargoResult, version_downgrade_err};

#[derive(Clone, Debug)]
pub(crate) enum TargetVersion {
    Relative(BumpLevel),
    Absolute(semver::Version),
}

impl TargetVersion {
    pub(crate) fn bump(
        &self,
        current: &semver::Version,
        metadata: Option<&str>,
    ) -> CargoResult<Option<semver::Version>> {
        match self {
            Self::Relative(bump_level) => {
                let mut potential_version = current.to_owned();
                bump_level.bump_version(&mut potential_version, metadata)?;
                if potential_version != *current {
                    let version = potential_version;
                    Ok(Some(version))
                } else {
                    Ok(None)
                }
            }
            Self::Absolute(version) => {
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
                    Err(version_downgrade_err(current, version))
                }
            }
        }
    }
}

impl Default for TargetVersion {
    fn default() -> Self {
        Self::Relative(BumpLevel::Release)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum BumpLevel {
    Major,
    Minor,
    Patch,
    /// Strip all pre-release flags
    Release,
    Rc,
    Beta,
    Alpha,
}

impl FromStr for BumpLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "major" => Ok(Self::Major),
            "minor" => Ok(Self::Minor),
            "patch" => Ok(Self::Patch),
            "release" => Ok(Self::Release),
            "rc" => Ok(Self::Rc),
            "beta" => Ok(Self::Beta),
            "alpha" => Ok(Self::Alpha),
            _ => Err(String::from(
                "[valid values: major, minor, patch, rc, beta, alpha]",
            )),
        }
    }
}

impl BumpLevel {
    pub(crate) fn bump_version(
        self,
        version: &mut semver::Version,
        metadata: Option<&str>,
    ) -> CargoResult<()> {
        match self {
            Self::Major => {
                version.increment_major();
            }
            Self::Minor => {
                version.increment_minor();
            }
            Self::Patch => {
                if !version.is_prerelease() {
                    version.increment_patch();
                } else {
                    version.pre = semver::Prerelease::EMPTY;
                }
            }
            Self::Release => {
                if version.is_prerelease() {
                    version.pre = semver::Prerelease::EMPTY;
                }
            }
            Self::Rc => {
                version.increment_rc()?;
            }
            Self::Beta => {
                version.increment_beta()?;
            }
            Self::Alpha => {
                version.increment_alpha()?;
            }
        };

        if let Some(metadata) = metadata {
            version.metadata(metadata)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn abs(version: &str) -> TargetVersion {
        let abs = semver::Version::parse(version).unwrap();
        TargetVersion::Absolute(abs)
    }

    #[test]
    fn abs_bump_from_dev() {
        let expected = "2022.3.0";
        let current = "2022.3.0-dev-12345";

        let target = abs(expected);
        let current = semver::Version::parse(current).unwrap();
        let actual = target.bump(&current, None).unwrap();
        let actual = actual.expect("Version changed").to_string();
        assert_eq!(actual, expected);
    }
}
