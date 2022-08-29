//! Crate name parsing.
use super::errors::*;

/// User-specified crate
///
/// This can be a
/// - Name (e.g. `docopt`)
/// - Name and a version req (e.g. `docopt@^0.8`)
/// - Path
#[derive(Debug)]
pub struct CrateSpec {
    /// Crate name
    pub name: String,
    /// Optional version requirement
    pub version_req: Option<String>,
}

impl CrateSpec {
    /// Convert a string to a `Crate`
    pub fn resolve(pkg_id: &str) -> CargoResult<Self> {
        let (name, version) = pkg_id
            .split_once('@')
            .map(|(n, v)| (n, Some(v)))
            .unwrap_or((pkg_id, None));

        let invalid: Vec<_> = name
            .chars()
            .filter(|c| !is_name_char(*c))
            .map(|c| c.to_string())
            .collect();
        if !invalid.is_empty() {
            return Err(anyhow::format_err!(
                "Invalid name `{}`: {}",
                name,
                invalid.join(", ")
            ));
        }

        if let Some(version) = version {
            semver::VersionReq::parse(version)
                .with_context(|| format!("Invalid version requirement `{}`", version))?;
        }

        Ok(Self {
            name: name.to_owned(),
            version_req: version.map(|s| s.to_owned()),
        })
    }
}

impl std::str::FromStr for CrateSpec {
    type Err = Error;

    fn from_str(s: &str) -> CargoResult<Self> {
        Self::resolve(s)
    }
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || ['-', '_'].contains(&c)
}
