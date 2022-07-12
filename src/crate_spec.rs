//! Crate name parsing.
use super::errors::*;

/// User-specified crate
///
/// This can be a
/// - Name (e.g. `docopt`)
/// - Name and a version req (e.g. `docopt@^0.8`)
/// - Path
#[derive(Debug)]
pub enum CrateSpec {
    /// Name with optional version req
    PkgId {
        /// Crate name
        name: String,
        /// Optional version requirement
        version_req: Option<String>,
    },
    /// Path to a crate root
    Path(std::path::PathBuf),
}

impl CrateSpec {
    /// Convert a string to a `Crate`
    pub fn resolve(pkg_id: &str) -> CargoResult<Self> {
        let path = std::path::Path::new(pkg_id);
        // For improved error messages, treat it like a path if it looks like one
        let id = if is_path_like(pkg_id) || path.exists() {
            Self::Path(path.to_owned())
        } else {
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

            Self::PkgId {
                name: name.to_owned(),
                version_req: version.map(|s| s.to_owned()),
            }
        };

        Ok(id)
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

fn is_path_like(s: &str) -> bool {
    s.contains('/') || s.contains('\\')
}
