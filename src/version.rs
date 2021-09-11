use std::str::FromStr;

use crate::errors::*;

/// Additional version functionality
pub trait VersionExt {
    /// Increments the major version number for this Version.
    fn increment_major(&mut self);
    /// Increments the minor version number for this Version.
    fn increment_minor(&mut self);
    /// Increments the patch version number for this Version.
    fn increment_patch(&mut self);
    /// Increment the alpha pre-release number for this Version.
    ///
    /// If this isn't alpha, switch to it.
    ///
    /// Errors if this would decrement the pre-release phase.
    fn increment_alpha(&mut self) -> Result<()>;
    /// Increment the beta pre-release number for this Version.
    ///
    /// If this isn't beta, switch to it.
    ///
    /// Errors if this would decrement the pre-release phase.
    fn increment_beta(&mut self) -> Result<()>;
    /// Increment the rc pre-release number for this Version.
    ///
    /// If this isn't rc, switch to it.
    ///
    /// Errors if this would decrement the pre-release phase.
    fn increment_rc(&mut self) -> Result<()>;
    /// Append informational-only metadata.
    fn metadata(&mut self, metadata: &str) -> Result<()>;
    /// Checks to see if the current Version is in pre-release status
    fn is_prerelease(&self) -> bool;
}

impl VersionExt for semver::Version {
    fn increment_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
        self.pre = semver::Prerelease::EMPTY;
        self.build = semver::BuildMetadata::EMPTY;
    }

    fn increment_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
        self.pre = semver::Prerelease::EMPTY;
        self.build = semver::BuildMetadata::EMPTY;
    }

    fn increment_patch(&mut self) {
        self.patch += 1;
        self.pre = semver::Prerelease::EMPTY;
        self.build = semver::BuildMetadata::EMPTY;
    }

    fn increment_alpha(&mut self) -> Result<()> {
        if let Some((pre_ext, pre_ext_ver)) = prerelease_id_version(self)? {
            if pre_ext == VERSION_BETA || pre_ext == VERSION_RC {
                Err(ErrorKind::InvalidReleaseLevel(VERSION_ALPHA, self.clone()).into())
            } else {
                let new_ext_ver = if pre_ext == VERSION_ALPHA {
                    pre_ext_ver.unwrap_or(0) + 1
                } else {
                    1
                };
                self.pre = semver::Prerelease::new(&format!("{}.{}", VERSION_ALPHA, new_ext_ver))?;
                Ok(())
            }
        } else {
            self.increment_patch();
            self.pre = semver::Prerelease::new(&format!("{}.1", VERSION_ALPHA))?;
            Ok(())
        }
    }

    fn increment_beta(&mut self) -> Result<()> {
        if let Some((pre_ext, pre_ext_ver)) = prerelease_id_version(self)? {
            if pre_ext == VERSION_RC {
                Err(ErrorKind::InvalidReleaseLevel(VERSION_BETA, self.clone()).into())
            } else {
                let new_ext_ver = if pre_ext == VERSION_BETA {
                    pre_ext_ver.unwrap_or(0) + 1
                } else {
                    1
                };
                self.pre = semver::Prerelease::new(&format!("{}.{}", VERSION_BETA, new_ext_ver))?;
                Ok(())
            }
        } else {
            self.increment_patch();
            self.pre = semver::Prerelease::new(&format!("{}.1", VERSION_BETA))?;
            Ok(())
        }
    }

    fn increment_rc(&mut self) -> Result<()> {
        if let Some((pre_ext, pre_ext_ver)) = prerelease_id_version(self)? {
            let new_ext_ver = if pre_ext == VERSION_RC {
                pre_ext_ver.unwrap_or(0) + 1
            } else {
                1
            };
            self.pre = semver::Prerelease::new(&format!("{}.{}", VERSION_RC, new_ext_ver))?;
            Ok(())
        } else {
            self.increment_patch();
            self.pre = semver::Prerelease::new(&format!("{}.1", VERSION_RC))?;
            Ok(())
        }
    }

    fn metadata(&mut self, build: &str) -> Result<()> {
        self.build = semver::BuildMetadata::new(build)?;
        Ok(())
    }

    fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }
}

static VERSION_ALPHA: &str = "alpha";
static VERSION_BETA: &str = "beta";
static VERSION_RC: &str = "rc";

fn prerelease_id_version(version: &semver::Version) -> Result<Option<(String, Option<u64>)>> {
    if !version.pre.is_empty() {
        if let Some((alpha, numeric)) = version.pre.as_str().split_once(".") {
            let alpha = alpha.to_owned();
            let numeric = u64::from_str(numeric)
                .map_err(|_| ErrorKind::UnsupportedPrereleaseVersionScheme)?;
            Ok(Some((alpha, Some(numeric))))
        } else {
            Ok(Some((version.pre.as_str().to_owned(), None)))
        }
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn alpha() {
        let mut v = semver::Version::parse("1.0.0").unwrap();
        v.increment_alpha().unwrap();
        assert_eq!(v, semver::Version::parse("1.0.1-alpha.1").unwrap());

        let mut v2 = semver::Version::parse("1.0.1-dev").unwrap();
        v2.increment_alpha().unwrap();
        assert_eq!(v2, semver::Version::parse("1.0.1-alpha.1").unwrap());

        let mut v3 = semver::Version::parse("1.0.1-alpha.1").unwrap();
        v3.increment_alpha().unwrap();
        assert_eq!(v3, semver::Version::parse("1.0.1-alpha.2").unwrap());

        let mut v4 = semver::Version::parse("1.0.1-beta.1").unwrap();
        assert!(v4.increment_alpha().is_err());
    }

    #[test]
    fn beta() {
        let mut v = semver::Version::parse("1.0.0").unwrap();
        v.increment_beta().unwrap();
        assert_eq!(v, semver::Version::parse("1.0.1-beta.1").unwrap());

        let mut v2 = semver::Version::parse("1.0.1-dev").unwrap();
        v2.increment_beta().unwrap();
        assert_eq!(v2, semver::Version::parse("1.0.1-beta.1").unwrap());

        let mut v2 = semver::Version::parse("1.0.1-alpha.1").unwrap();
        v2.increment_beta().unwrap();
        assert_eq!(v2, semver::Version::parse("1.0.1-beta.1").unwrap());

        let mut v3 = semver::Version::parse("1.0.1-beta.1").unwrap();
        v3.increment_beta().unwrap();
        assert_eq!(v3, semver::Version::parse("1.0.1-beta.2").unwrap());

        let mut v4 = semver::Version::parse("1.0.1-rc.1").unwrap();
        assert!(v4.increment_beta().is_err());
    }

    #[test]
    fn rc() {
        let mut v = semver::Version::parse("1.0.0").unwrap();
        v.increment_rc().unwrap();
        assert_eq!(v, semver::Version::parse("1.0.1-rc.1").unwrap());

        let mut v2 = semver::Version::parse("1.0.1-dev").unwrap();
        v2.increment_rc().unwrap();
        assert_eq!(v2, semver::Version::parse("1.0.1-rc.1").unwrap());

        let mut v3 = semver::Version::parse("1.0.1-rc.1").unwrap();
        v3.increment_rc().unwrap();
        assert_eq!(v3, semver::Version::parse("1.0.1-rc.2").unwrap());
    }

    #[test]
    fn metadata() {
        let mut v = semver::Version::parse("1.0.0").unwrap();
        v.metadata("git.123456").unwrap();
        assert_eq!(v, semver::Version::parse("1.0.0+git.123456").unwrap());
    }
}
