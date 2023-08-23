use std::collections::BTreeMap;
use std::path::Path;

use url::Url;

use super::errors::*;
use super::registry::registry_url;
use super::shell_status;
use super::Dependency;
use super::RegistrySource;
use super::VersionExt;

/// Query latest version from a registry index
///
/// The registry argument must be specified for crates
/// from alternative registries.
///
/// The latest version will be returned as a `Dependency`. This will fail, when
///
/// - there is no Internet connection and offline is false.
/// - summaries in registry index with an incorrect format.
/// - a crate with the given name does not exist on the registry.
pub fn get_latest_dependency(
    crate_: &crates_index::Crate,
    flag_allow_prerelease: bool,
    rust_version: Option<RustVersion>,
) -> CargoResult<Dependency> {
    let crate_versions = crate_versions_from_crate(crate_)?;
    read_latest_version(&crate_versions, flag_allow_prerelease, rust_version)
}

/// Find the highest version compatible with a version req
pub fn get_compatible_dependency(
    crate_: &crates_index::Crate,
    version_req: &semver::VersionReq,
    rust_version: Option<RustVersion>,
) -> CargoResult<Dependency> {
    let crate_versions = crate_versions_from_crate(crate_)?;
    read_compatible_version(&crate_versions, version_req, rust_version)
}

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

impl TryFrom<&semver::Version> for RustVersion {
    type Error = anyhow::Error;

    fn try_from(sem_ver: &semver::Version) -> Result<Self, Self::Error> {
        anyhow::ensure!(
            sem_ver.pre.is_empty(),
            "rust-version must be a value like `1.32`"
        );
        anyhow::ensure!(
            sem_ver.build.is_empty(),
            "rust-version must be a value like `1.32`"
        );
        Ok(Self {
            major: sem_ver.major,
            minor: sem_ver.minor,
            patch: sem_ver.patch,
        })
    }
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

#[derive(Debug)]
struct CrateVersion {
    name: String,
    version: semver::Version,
    rust_version: Option<RustVersion>,
    yanked: bool,
    available_features: BTreeMap<String, Vec<String>>,
}

/// Fetch crate from sparse index, remotely or from cache
pub fn crate_from_sparse_index(
    ureq_agent: &ureq::Agent,
    crate_name: impl Into<String>,
    manifest_path: &Path,
    registry: Option<&Url>,
    use_cached: bool,
) -> CargoResult<crates_index::Crate> {
    // https://doc.rust-lang.org/cargo/reference/registry-index.html#nonexistent-crates
    const NON_EXISTENT_CRATE_STATUS_ERRORS: &[u16] = &[404, 410, 451];

    let crate_name = crate_name.into();

    if crate_name.is_empty() {
        anyhow::bail!("Found empty crate name");
    }

    let registry_url = match registry {
        Some(url) => url.clone(),
        None => registry_url(manifest_path, None)?,
    };

    let index = crates_index::SparseIndex::from_url(registry_url.as_str())?;

    let mut names = gen_fuzzy_crate_names(crate_name.clone())?;
    if let Some(index) = names.iter().position(|x| *x == crate_name) {
        // ref: https://github.com/killercup/cargo-edit/pull/317#discussion_r307365704
        names.swap(index, 0);
    }

    fn req_to_ureq(req: http::request::Builder, ureq_agent: &ureq::Agent) -> Option<ureq::Request> {
        let uri = req.uri_ref()?.to_string();
        let u_req = req.headers_ref()
            .iter()
            .map(|hm| hm.iter())
            .flatten()
            .fold(ureq_agent.get(&uri), |u_req, (h_k, h_v)| {
                match h_v.to_str() {
                    Ok(h_v) => u_req.set(&h_k.to_string(), h_v),
                    Err(_)  => u_req, // skip header if header value is not unicode
                }
            });
        Some(u_req)
    }

    for the_name in names {
        let crate_ = match index.make_cache_request(&the_name).ok() {
            Some(crate_req) => {
                if use_cached {
                    shell_status("SparseIndex", &format!("getting cached info for crate {the_name}"))?;
                    match index.crate_from_cache(&the_name) {
                        Ok(crate_) => {
                            if the_name != crate_name {
                                eprintln!("WARN: Found `{the_name}` instead of `{crate_name}`");
                            }
                            crate_
                        },
                        Err(_) => continue,
                    }
                } else {
                    shell_status("SparseIndex", &format!("getting remote info for crate {the_name}"))?;

                    let u_req = match req_to_ureq(crate_req, ureq_agent) {
                        Some(u_req) => u_req,
                        None => continue,
                    };

                    let resp = match u_req.call() {
                        Err(e) => {
                            if let ureq::Error::Status(status, _) = &e {
                                if  NON_EXISTENT_CRATE_STATUS_ERRORS.contains(status) {
                                    continue;
                                }
                            }
                            return Err(e.into());
                        },
                        Ok(resp) => resp,
                    };
                    match resp.status() {
                        200 => (),
                        304 => {
                            // cached info up to date
                            if the_name != crate_name {
                                eprintln!("WARN: Found `{the_name}` instead of `{crate_name}`");
                            }
                            return crate_from_sparse_index(ureq_agent, the_name, manifest_path, registry, true);
                        },
                        status => anyhow::bail!("getting remote info for {the_name} returned unexpected status {status}"),
                    }
                    let mut buf = Vec::with_capacity(1<<16);
                    resp.into_reader().read_to_end(&mut buf)?;
                    let crate_ = crates_index::Crate::from_slice(&*buf)?;
                    if the_name != crate_name {
                        eprintln!("WARN: Found `{the_name}` instead of `{crate_name}`");
                    }
                    crate_
                }
            },
            None => continue,
        };
        return Ok(crate_);
    }
    Err(no_crate_err(crate_name))
}

/// Get crate versions from crate info we got from the index
fn crate_versions_from_crate(crate_: &crates_index::Crate) -> CargoResult<Vec<CrateVersion>> {
    crate_
        .versions()
        .iter()
        .map(|v| {
            Ok(CrateVersion {
                name: v.name().to_owned(),
                version: v.version().parse()?,
                rust_version: v.rust_version().map(|r| r.parse()).transpose()?,
                yanked: v.is_yanked(),
                available_features: registry_features(v),
            })
        })
    .collect()
}

/// Generate all similar crate names
///
/// Examples:
///
/// | input | output |
/// | ----- | ------ |
/// | cargo | cargo  |
/// | cargo-edit | cargo-edit, cargo_edit |
/// | parking_lot_core | parking_lot_core, parking_lot-core, parking-lot_core, parking-lot-core |
fn gen_fuzzy_crate_names(crate_name: String) -> CargoResult<Vec<String>> {
    const PATTERN: [u8; 2] = [b'-', b'_'];

    let wildcard_indexs = crate_name
        .bytes()
        .enumerate()
        .filter(|(_, item)| PATTERN.contains(item))
        .map(|(index, _)| index)
        .take(10)
        .collect::<Vec<usize>>();
    if wildcard_indexs.is_empty() {
        return Ok(vec![crate_name]);
    }

    let mut result = vec![];
    let mut bytes = crate_name.into_bytes();
    for mask in 0..2u128.pow(wildcard_indexs.len() as u32) {
        for (mask_index, wildcard_index) in wildcard_indexs.iter().enumerate() {
            let mask_value = (mask >> mask_index) & 1 == 1;
            if mask_value {
                bytes[*wildcard_index] = b'-';
            } else {
                bytes[*wildcard_index] = b'_';
            }
        }
        result.push(String::from_utf8(bytes.clone()).unwrap());
    }
    Ok(result)
}

// Checks whether a version object is a stable release
fn version_is_stable(version: &CrateVersion) -> bool {
    !version.version.is_prerelease()
}

/// Read latest version from Versions structure
fn read_latest_version(
    versions: &[CrateVersion],
    flag_allow_prerelease: bool,
    rust_version: Option<RustVersion>,
) -> CargoResult<Dependency> {
    let latest = versions
        .iter()
        .filter(|&v| flag_allow_prerelease || version_is_stable(v))
        .filter(|&v| !v.yanked)
        .filter(|&v| {
            rust_version
                .and_then(|rust_version| {
                    v.rust_version
                        .map(|v_rust_version| v_rust_version <= rust_version)
                })
                .unwrap_or(true)
        })
        .max_by_key(|&v| v.version.clone())
        .ok_or_else(|| {
            anyhow::format_err!(
                "No available versions exist. Either all were yanked \
                         or only prerelease versions exist. Trying with the \
                         --allow-prerelease flag might solve the issue."
            )
        })?;

    let name = &latest.name;
    let version = latest.version.to_string();
    Ok(Dependency::new(name)
        .set_source(RegistrySource::new(version))
        .set_available_features(latest.available_features.clone()))
}

fn read_compatible_version(
    versions: &[CrateVersion],
    version_req: &semver::VersionReq,
    rust_version: Option<RustVersion>,
) -> CargoResult<Dependency> {
    let latest = versions
        .iter()
        .filter(|&v| version_req.matches(&v.version))
        .filter(|&v| !v.yanked)
        .filter(|&v| {
            rust_version
                .and_then(|rust_version| {
                    v.rust_version
                        .map(|v_rust_version| v_rust_version <= rust_version)
                })
                .unwrap_or(true)
        })
        .max_by_key(|&v| v.version.clone())
        .ok_or_else(|| {
            anyhow::format_err!(
                "No available versions exist. Either all were yanked \
                         or only prerelease versions exist. Trying with the \
                         --allow-prerelease flag might solve the issue."
            )
        })?;

    let name = &latest.name;
    let version = latest.version.to_string();
    Ok(Dependency::new(name)
        .set_source(RegistrySource::new(version))
        .set_available_features(latest.available_features.clone()))
}

fn registry_features(v: &crates_index::Version) -> BTreeMap<String, Vec<String>> {
    let mut features: BTreeMap<_, _> = v
        .features()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    features.extend(
        v.dependencies()
            .iter()
            .filter(|d| d.is_optional())
            .map(|d| (d.crate_name().to_owned(), vec![])),
    );
    features
}

#[test]
fn test_gen_fuzzy_crate_names() {
    fn test_helper(input: &str, expect: &[&str]) {
        let mut actual = gen_fuzzy_crate_names(input.to_string()).unwrap();
        actual.sort();

        let mut expect = expect.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        expect.sort();

        assert_eq!(actual, expect);
    }

    test_helper("", &[""]);
    test_helper("-", &["_", "-"]);
    test_helper("DCjanus", &["DCjanus"]);
    test_helper("DC-janus", &["DC-janus", "DC_janus"]);
    test_helper(
        "DC-_janus",
        &["DC__janus", "DC_-janus", "DC-_janus", "DC--janus"],
    );
}

#[test]
fn get_latest_stable_version() {
    let versions = vec![
        CrateVersion {
            name: "foo".into(),
            version: "0.6.0-alpha".parse().unwrap(),
            rust_version: None,
            yanked: false,
            available_features: BTreeMap::new(),
        },
        CrateVersion {
            name: "foo".into(),
            version: "0.5.0".parse().unwrap(),
            rust_version: None,
            yanked: false,
            available_features: BTreeMap::new(),
        },
    ];
    assert_eq!(
        read_latest_version(&versions, false, None)
            .unwrap()
            .version()
            .unwrap(),
        "0.5.0"
    );
}

#[test]
fn get_latest_unstable_or_stable_version() {
    let versions = vec![
        CrateVersion {
            name: "foo".into(),
            version: "0.6.0-alpha".parse().unwrap(),
            rust_version: None,
            yanked: false,
            available_features: BTreeMap::new(),
        },
        CrateVersion {
            name: "foo".into(),
            version: "0.5.0".parse().unwrap(),
            rust_version: None,
            yanked: false,
            available_features: BTreeMap::new(),
        },
    ];
    assert_eq!(
        read_latest_version(&versions, true, None)
            .unwrap()
            .version()
            .unwrap(),
        "0.6.0-alpha"
    );
}

#[test]
fn get_latest_version_with_yanked() {
    let versions = vec![
        CrateVersion {
            name: "treexml".into(),
            version: "0.3.1".parse().unwrap(),
            rust_version: None,
            yanked: true,
            available_features: BTreeMap::new(),
        },
        CrateVersion {
            name: "true".into(),
            version: "0.3.0".parse().unwrap(),
            rust_version: None,
            yanked: false,
            available_features: BTreeMap::new(),
        },
    ];
    assert_eq!(
        read_latest_version(&versions, false, None)
            .unwrap()
            .version()
            .unwrap(),
        "0.3.0"
    );
}

#[test]
fn get_no_latest_version_from_json_when_all_are_yanked() {
    let versions = vec![
        CrateVersion {
            name: "treexml".into(),
            version: "0.3.1".parse().unwrap(),
            rust_version: None,
            yanked: true,
            available_features: BTreeMap::new(),
        },
        CrateVersion {
            name: "true".into(),
            version: "0.3.0".parse().unwrap(),
            rust_version: None,
            yanked: true,
            available_features: BTreeMap::new(),
        },
    ];
    assert!(read_latest_version(&versions, false, None).is_err());
}
