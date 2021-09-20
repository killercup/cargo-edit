//! Crate name parsing.
use crate::errors::*;
use crate::Dependency;
use crate::{get_crate_name_from_github, get_crate_name_from_gitlab, get_crate_name_from_path};

/// A crate specifier. This can be a plain name (e.g. `docopt`), a name and a versionreq (e.g.
/// `docopt@^0.8`), a URL, or a path.
#[derive(Debug)]
pub struct CrateName<'a>(&'a str);

impl<'a> CrateName<'a> {
    /// Create a new `CrateName`
    pub fn new(name: &'a str) -> Self {
        CrateName(name)
    }

    /// Get crate name
    pub fn name(&self) -> &str {
        self.0
    }

    /// Does this specify a versionreq?
    pub fn has_version(&self) -> bool {
        self.0.contains('@')
    }

    /// Is this a URI?
    pub fn is_url_or_path(&self) -> bool {
        self.is_github_url() || self.is_gitlab_url() || self.is_path()
    }

    fn is_github_url(&self) -> bool {
        self.0.contains("https://github.com")
    }

    fn is_gitlab_url(&self) -> bool {
        self.0.contains("https://gitlab.com")
    }

    fn is_path(&self) -> bool {
        // FIXME: how else can we check if the name is a (possibly invalid) path?
        self.0.contains('.') || self.0.contains('/') || self.0.contains('\\')
    }

    /// Checks is the specified crate name is a valid, non-empty name for a crates.io crate,
    /// meaning it contains only a-zA-Z, dashes, and underscores.
    /// expected to be usually called as validate_name()?;
    pub fn validate_name(&self) -> Result<()> {
        if self.name().is_empty() {
            return Err(ErrorKind::EmptyCrateName.into());
        }


        let contains_only_valid_characters: bool;
        let mut invalid_char: char;

        if self.name().chars().next().unwrap().is_alphabetic() {
            invalid_char = 'a'; //placeholder value. Is always a valid character in a crate name.
            contains_only_valid_characters = self.name().chars().all(|c| {
                let is_valid = (c.is_alphanumeric() || c == '-' || c == '_') && c.is_ascii();

                if !is_valid {
                    invalid_char = c;
                }
                is_valid
            });
        } else {
            invalid_char = self.name().chars().next().unwrap();
            contains_only_valid_characters = false;
        }

        if !contains_only_valid_characters {
            assert_ne!(invalid_char, 'a'); //check if invalid_char still does not contains its initial placeholder value. That should never happen
            return Err(ErrorKind::CrateNameContainsInvalidCharacter(
                self.name().to_string(),
                invalid_char,
            )
            .into());
        }

        Ok(())
    }

    /// If this crate specifier includes a version (e.g. `docopt@0.8`), extract the name and
    /// version.
    pub fn parse_as_version(&self) -> Result<Option<Dependency>> {
        if self.has_version() {
            let xs: Vec<_> = self.0.splitn(2, '@').collect();
            let (name, version) = (xs[0], xs[1]);
            semver::VersionReq::parse(version).chain_err(|| "Invalid crate version requirement")?;

            self.validate_name()?;

            Ok(Some(Dependency::new(name).set_version(version)))
        } else {
            Ok(None)
        }
    }

    /// Will parse this crate name on the assumption that it is a URI.
    pub fn parse_crate_name_from_uri(&self) -> Result<Dependency> {
        if self.is_github_url() {
            if let Ok(ref crate_name) = get_crate_name_from_github(self.0) {
                return Ok(Dependency::new(crate_name).set_git(self.0, None));
            }
        } else if self.is_gitlab_url() {
            if let Ok(ref crate_name) = get_crate_name_from_gitlab(self.0) {
                return Ok(Dependency::new(crate_name).set_git(self.0, None));
            }
        } else if self.is_path() {
            if let Ok(ref crate_name) = get_crate_name_from_path(self.0) {
                return Ok(Dependency::new(crate_name).set_path(self.0));
            }
        }

        bail!("Unable to obtain crate informations from `{}`.\n", self.0)
    }
}
