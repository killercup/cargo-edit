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
    fn increment_alpha(&mut self) -> CargoResult<()>;
    /// Increment the beta pre-release number for this Version.
    ///
    /// If this isn't beta, switch to it.
    ///
    /// Errors if this would decrement the pre-release phase.
    fn increment_beta(&mut self) -> CargoResult<()>;
    /// Increment the rc pre-release number for this Version.
    ///
    /// If this isn't rc, switch to it.
    ///
    /// Errors if this would decrement the pre-release phase.
    fn increment_rc(&mut self) -> CargoResult<()>;
    /// Append informational-only metadata.
    fn metadata(&mut self, metadata: &str) -> CargoResult<()>;
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

    fn increment_alpha(&mut self) -> CargoResult<()> {
        if let Some((pre_ext, pre_ext_ver)) = prerelease_id_version(self)? {
            if pre_ext == VERSION_BETA || pre_ext == VERSION_RC {
                Err(invalid_release_level(VERSION_ALPHA, self.clone()))
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

    fn increment_beta(&mut self) -> CargoResult<()> {
        if let Some((pre_ext, pre_ext_ver)) = prerelease_id_version(self)? {
            if pre_ext == VERSION_RC {
                Err(invalid_release_level(VERSION_BETA, self.clone()))
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

    fn increment_rc(&mut self) -> CargoResult<()> {
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

    fn metadata(&mut self, build: &str) -> CargoResult<()> {
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

fn prerelease_id_version(version: &semver::Version) -> CargoResult<Option<(String, Option<u64>)>> {
    if !version.pre.is_empty() {
        if let Some((alpha, numeric)) = version.pre.as_str().split_once(".") {
            let alpha = alpha.to_owned();
            let numeric = u64::from_str(numeric)
                .map_err(|_| anyhow::format_err!("This version scheme is not supported. Use format like `pre`, `dev` or `alpha.1` for prerelease symbol"))?;
            Ok(Some((alpha, Some(numeric))))
        } else {
            Ok(Some((version.pre.as_str().to_owned(), None)))
        }
    } else {
        Ok(None)
    }
}

/// Upgrade an existing requirement to a new version
pub fn upgrade_requirement(req: &str, version: &semver::Version) -> CargoResult<Option<String>> {
    let req_text = req.to_string();
    let raw_req = semver::VersionReq::parse(&req_text)
        .expect("semver to generate valid version requirements");
    if raw_req.comparators.is_empty() {
        // Empty matches everything, no-change.
        Ok(None)
    } else {
        let comparators: CargoResult<Vec<_>> = raw_req
            .comparators
            .into_iter()
            .map(|p| set_comparator(p, version))
            .collect();
        let comparators = comparators?;
        let new_req = semver::VersionReq { comparators };
        let mut new_req_text = new_req.to_string();
        if new_req_text.starts_with('^') && !req.starts_with('^') {
            new_req_text.remove(0);
        }
        // Validate contract
        #[cfg(debug_assert)]
        {
            assert!(
                new_req.matches(version),
                "Invalid req created: {}",
                new_req_text
            )
        }
        if new_req_text == req_text {
            Ok(None)
        } else {
            Ok(Some(new_req_text))
        }
    }
}

fn set_comparator(
    mut pred: semver::Comparator,
    version: &semver::Version,
) -> CargoResult<semver::Comparator> {
    match pred.op {
        semver::Op::Wildcard => {
            pred.major = version.major;
            if pred.minor.is_some() {
                pred.minor = Some(version.minor);
            }
            if pred.patch.is_some() {
                pred.patch = Some(version.patch);
            }
            Ok(pred)
        }
        semver::Op::Exact => Ok(assign_partial_req(version, pred)),
        semver::Op::Greater | semver::Op::GreaterEq | semver::Op::Less | semver::Op::LessEq => {
            let user_pred = pred.to_string();
            Err(unsupported_version_req(user_pred))
        }
        semver::Op::Tilde => Ok(assign_partial_req(version, pred)),
        semver::Op::Caret => Ok(assign_partial_req(version, pred)),
        _ => {
            let user_pred = pred.to_string();
            Err(unsupported_version_req(user_pred))
        }
    }
}

fn assign_partial_req(
    version: &semver::Version,
    mut pred: semver::Comparator,
) -> semver::Comparator {
    pred.major = version.major;
    if pred.minor.is_some() {
        pred.minor = Some(version.minor);
    }
    if pred.patch.is_some() {
        pred.patch = Some(version.patch);
    }
    pred.pre = version.pre.clone();
    pred
}

#[cfg(test)]
mod test {
    use super::*;

    mod increment {
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

    mod upgrade_requirement {
        use super::*;

        #[track_caller]
        fn assert_req_bump<'a, O: Into<Option<&'a str>>>(version: &str, req: &str, expected: O) {
            let version = semver::Version::parse(version).unwrap();
            let actual = upgrade_requirement(req, &version).unwrap();
            let expected = expected.into();
            assert_eq!(actual.as_deref(), expected);
        }

        #[test]
        fn wildcard_major() {
            assert_req_bump("1.0.0", "*", None);
        }

        #[test]
        fn wildcard_minor() {
            assert_req_bump("1.0.0", "1.*", None);
            assert_req_bump("1.1.0", "1.*", None);
            assert_req_bump("2.0.0", "1.*", "2.*");
        }

        #[test]
        fn wildcard_patch() {
            assert_req_bump("1.0.0", "1.0.*", None);
            assert_req_bump("1.1.0", "1.0.*", "1.1.*");
            assert_req_bump("1.1.1", "1.0.*", "1.1.*");
            assert_req_bump("2.0.0", "1.0.*", "2.0.*");
        }

        #[test]
        fn caret_major() {
            assert_req_bump("1.0.0", "1", None);
            assert_req_bump("1.0.0", "^1", None);

            assert_req_bump("1.1.0", "1", None);
            assert_req_bump("1.1.0", "^1", None);

            assert_req_bump("2.0.0", "1", "2");
            assert_req_bump("2.0.0", "^1", "^2");
        }

        #[test]
        fn caret_minor() {
            assert_req_bump("1.0.0", "1.0", None);
            assert_req_bump("1.0.0", "^1.0", None);

            assert_req_bump("1.1.0", "1.0", "1.1");
            assert_req_bump("1.1.0", "^1.0", "^1.1");

            assert_req_bump("1.1.1", "1.0", "1.1");
            assert_req_bump("1.1.1", "^1.0", "^1.1");

            assert_req_bump("2.0.0", "1.0", "2.0");
            assert_req_bump("2.0.0", "^1.0", "^2.0");
        }

        #[test]
        fn caret_patch() {
            assert_req_bump("1.0.0", "1.0.0", None);
            assert_req_bump("1.0.0", "^1.0.0", None);

            assert_req_bump("1.1.0", "1.0.0", "1.1.0");
            assert_req_bump("1.1.0", "^1.0.0", "^1.1.0");

            assert_req_bump("1.1.1", "1.0.0", "1.1.1");
            assert_req_bump("1.1.1", "^1.0.0", "^1.1.1");

            assert_req_bump("2.0.0", "1.0.0", "2.0.0");
            assert_req_bump("2.0.0", "^1.0.0", "^2.0.0");
        }

        #[test]
        fn tilde_major() {
            assert_req_bump("1.0.0", "~1", None);
            assert_req_bump("1.1.0", "~1", None);
            assert_req_bump("2.0.0", "~1", "~2");
        }

        #[test]
        fn tilde_minor() {
            assert_req_bump("1.0.0", "~1.0", None);
            assert_req_bump("1.1.0", "~1.0", "~1.1");
            assert_req_bump("1.1.1", "~1.0", "~1.1");
            assert_req_bump("2.0.0", "~1.0", "~2.0");
        }

        #[test]
        fn tilde_patch() {
            assert_req_bump("1.0.0", "~1.0.0", None);
            assert_req_bump("1.1.0", "~1.0.0", "~1.1.0");
            assert_req_bump("1.1.1", "~1.0.0", "~1.1.1");
            assert_req_bump("2.0.0", "~1.0.0", "~2.0.0");
        }

        #[test]
        fn equal_major() {
            assert_req_bump("1.0.0", "=1", None);
            assert_req_bump("1.1.0", "=1", None);
            assert_req_bump("2.0.0", "=1", "=2");
        }

        #[test]
        fn equal_minor() {
            assert_req_bump("1.0.0", "=1.0", None);
            assert_req_bump("1.1.0", "=1.0", "=1.1");
            assert_req_bump("1.1.1", "=1.0", "=1.1");
            assert_req_bump("2.0.0", "=1.0", "=2.0");
        }

        #[test]
        fn equal_patch() {
            assert_req_bump("1.0.0", "=1.0.0", None);
            assert_req_bump("1.1.0", "=1.0.0", "=1.1.0");
            assert_req_bump("1.1.1", "=1.0.0", "=1.1.1");
            assert_req_bump("2.0.0", "=1.0.0", "=2.0.0");
        }
    }
}
