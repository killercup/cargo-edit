use super::Dependency;
use super::RegistrySource;
use super::VersionExt;
use tame_index::IndexVersion;

/// Simplified represetation of `package.rust-version`
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RustVersion {
    #[allow(missing_docs)]
    pub major: u64,
    #[allow(missing_docs)]
    pub minor: u64,
    #[allow(missing_docs)]
    pub patch: u64,
}

impl RustVersion {
    /// Minimum-possible `package.rust-version`
    pub const MIN: Self = RustVersion {
        major: 1,
        minor: 0,
        patch: 0,
    };
    /// Maximum-possible `package.rust-version`
    pub const MAX: Self = RustVersion {
        major: u64::MAX,
        minor: u64::MAX,
        patch: u64::MAX,
    };
}

impl std::str::FromStr for RustVersion {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let version_req = text.parse::<semver::VersionReq>()?;
        anyhow::ensure!(
            version_req.comparators.len() == 1,
            "rust-version must be a value like `1.32`"
        );
        let comp = &version_req.comparators[0];
        anyhow::ensure!(
            comp.op == semver::Op::Caret,
            "rust-version must be a value like `1.32`"
        );
        anyhow::ensure!(
            comp.pre == semver::Prerelease::EMPTY,
            "rust-version must be a value like `1.32`"
        );
        Ok(Self {
            major: comp.major,
            minor: comp.minor.unwrap_or(0),
            patch: comp.patch.unwrap_or(0),
        })
    }
}

impl From<&'_ semver::VersionReq> for RustVersion {
    fn from(version_req: &semver::VersionReq) -> Self {
        // HACK: `rust-version` is a subset of the `VersionReq` syntax that only ever
        // has one comparator with a required minor and optional patch, and uses no
        // other features. If in the future this syntax is expanded, this code will need
        // to be updated.
        assert!(version_req.comparators.len() == 1);
        let comp = &version_req.comparators[0];
        assert_eq!(comp.op, semver::Op::Caret);
        assert_eq!(comp.pre, semver::Prerelease::EMPTY);
        Self {
            major: comp.major,
            minor: comp.minor.unwrap_or(0),
            patch: comp.patch.unwrap_or(0),
        }
    }
}

impl From<&'_ semver::Version> for RustVersion {
    fn from(version: &semver::Version) -> Self {
        Self {
            major: version.major,
            minor: version.minor,
            patch: version.patch,
        }
    }
}

// Checks whether a version object is a stable release
fn version_is_stable(version: &semver::Version) -> bool {
    !version.is_prerelease()
}

/// Read latest version from Versions structure
pub fn find_latest_version(
    versions: &[IndexVersion],
    flag_allow_prerelease: bool,
    rust_version: Option<RustVersion>,
) -> Option<Dependency> {
    let (latest, _) = versions
        .iter()
        .filter_map(|k| Some((k, k.version.parse::<semver::Version>().ok()?)))
        .filter(|(_, v)| flag_allow_prerelease || version_is_stable(v))
        .filter(|(k, _)| !k.yanked)
        .filter(|(k, _)| filter_by_rust_version(rust_version, k))
        .max_by_key(|(_, v)| v.clone())?;

    let name = &latest.name;
    let version = latest.version.to_string();
    Some(Dependency::new(name).set_source(RegistrySource::new(version)))
}

pub fn find_compatible_version(
    versions: &[IndexVersion],
    version_req: &semver::VersionReq,
    rust_version: Option<RustVersion>,
) -> Option<Dependency> {
    let (latest, _) = versions
        .iter()
        .filter_map(|k| Some((k, k.version.parse::<semver::Version>().ok()?)))
        .filter(|(_, v)| version_req.matches(v))
        .filter(|(k, _)| !k.yanked)
        .filter(|(k, _)| filter_by_rust_version(rust_version, k))
        .max_by_key(|(_, v)| v.clone())?;

    let name = &latest.name;
    let version = latest.version.to_string();
    Some(Dependency::new(name).set_source(RegistrySource::new(version)))
}

fn filter_by_rust_version(rust_version: Option<RustVersion>, k: &&IndexVersion) -> bool {
    rust_version
        .and_then(|rust_version| {
            k.rust_version
                .as_ref()
                .and_then(|k_rust_version| k_rust_version.parse::<RustVersion>().ok())
                .map(|k_rust_version| k_rust_version <= rust_version)
        })
        .unwrap_or(true)
}
