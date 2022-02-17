//! Crate name parsing.
use super::errors::*;
use super::get_manifest_from_path;
use super::Dependency;

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

    /// Whether the version req is known or not
    pub fn has_version(&self) -> bool {
        match self {
            Self::PkgId {
                name: _,
                version_req,
            } => version_req.is_some(),
            Self::Path(_path) => {
                // We'll get it from the manifest
                true
            }
        }
    }

    /// Generate a dependency entry for this crate specifier
    pub fn to_dependency(&self) -> CargoResult<Dependency> {
        let dep = match self {
            Self::PkgId { name, version_req } => {
                let mut dep = Dependency::new(name);
                if let Some(version_req) = version_req {
                    dep = dep.set_version(version_req);
                }
                dep
            }
            Self::Path(path) => {
                let manifest = get_manifest_from_path(path)?;
                let crate_name = manifest.package_name()?;
                let path = dunce::canonicalize(path)?;
                let available_features = manifest.features()?;
                Dependency::new(crate_name)
                    .set_path(path)
                    .set_available_features(available_features)
            }
        };

        Ok(dep)
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
