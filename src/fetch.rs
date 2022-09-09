use std::collections::BTreeMap;
use std::env;
use std::path::Path;
use std::time::Duration;

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
    crate_name: &str,
    flag_allow_prerelease: bool,
    manifest_path: &Path,
    registry: Option<&Url>,
) -> CargoResult<Dependency> {
    if env::var("CARGO_IS_TEST").is_ok() {
        // We are in a simulated reality. Nothing is real here.
        // FIXME: Use actual test handling code.
        let new_version = if flag_allow_prerelease {
            format!("99999.0.0-alpha.1+{}", crate_name)
        } else {
            match crate_name {
                "test_breaking" => "0.2.0".to_string(),
                "test_nonbreaking" => "0.1.1".to_string(),
                other => format!("99999.0.0+{}", other),
            }
        };

        let features = if crate_name == "your-face" {
            [
                ("nose".to_string(), vec![]),
                ("mouth".to_string(), vec![]),
                ("eyes".to_string(), vec![]),
                ("ears".to_string(), vec![]),
            ]
            .into_iter()
            .collect::<BTreeMap<_, _>>()
        } else {
            BTreeMap::default()
        };

        return Ok(Dependency::new(crate_name)
            .set_source(RegistrySource::new(&new_version))
            .set_available_features(features));
    }

    if crate_name.is_empty() {
        anyhow::bail!("Found empty crate name");
    }

    let registry = match registry {
        Some(url) => url.clone(),
        None => registry_url(manifest_path, None)?,
    };

    let crate_versions = fuzzy_query_registry_index(crate_name, &registry)?;

    let dep = read_latest_version(&crate_versions, flag_allow_prerelease)?;

    if dep.name != crate_name {
        eprintln!("WARN: Added `{}` instead of `{}`", dep.name, crate_name);
    }

    Ok(dep)
}

/// Find the highest version compatible with a version req
pub fn get_compatible_dependency(
    crate_name: &str,
    version_req: &semver::VersionReq,
    manifest_path: &Path,
    registry: Option<&Url>,
) -> CargoResult<Dependency> {
    if crate_name.is_empty() {
        anyhow::bail!("Found empty crate name");
    }

    let registry = match registry {
        Some(url) => url.clone(),
        None => registry_url(manifest_path, None)?,
    };

    let crate_versions = fuzzy_query_registry_index(crate_name, &registry)?;

    let dep = read_compatible_version(&crate_versions, version_req)?;

    if dep.name != crate_name {
        eprintln!("WARN: Added `{}` instead of `{}`", dep.name, crate_name);
    }

    Ok(dep)
}

#[derive(Debug)]
struct CrateVersion {
    name: String,
    version: semver::Version,
    yanked: bool,
    available_features: BTreeMap<String, Vec<String>>,
}

/// Fuzzy query crate from registry index
fn fuzzy_query_registry_index(
    crate_name: impl Into<String>,
    registry: &Url,
) -> CargoResult<Vec<CrateVersion>> {
    let index = crates_index::Index::from_url(registry.as_str())?;

    let crate_name = crate_name.into();
    let mut names = gen_fuzzy_crate_names(crate_name.clone())?;
    if let Some(index) = names.iter().position(|x| *x == crate_name) {
        // ref: https://github.com/killercup/cargo-edit/pull/317#discussion_r307365704
        names.swap(index, 0);
    }

    for the_name in names {
        let crate_ = match index.crate_(&the_name) {
            Some(crate_) => crate_,
            None => continue,
        };
        return crate_
            .versions()
            .iter()
            .map(|v| {
                Ok(CrateVersion {
                    name: v.name().to_owned(),
                    version: v.version().parse()?,
                    yanked: v.is_yanked(),
                    available_features: registry_features(v),
                })
            })
            .collect();
    }
    Err(no_crate_err(crate_name))
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
) -> CargoResult<Dependency> {
    let latest = versions
        .iter()
        .filter(|&v| flag_allow_prerelease || version_is_stable(v))
        .filter(|&v| !v.yanked)
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
        .set_source(RegistrySource::new(&version))
        .set_available_features(latest.available_features.clone()))
}

fn read_compatible_version(
    versions: &[CrateVersion],
    version_req: &semver::VersionReq,
) -> CargoResult<Dependency> {
    let latest = versions
        .iter()
        .filter(|&v| version_req.matches(&v.version))
        .filter(|&v| !v.yanked)
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
        .set_source(RegistrySource::new(&version))
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

/// update registry index for given project
pub fn update_registry_index(registry: &Url, quiet: bool) -> CargoResult<()> {
    let mut index = crates_index::Index::from_url(registry.as_str())?;
    if !quiet {
        shell_status("Updating", &format!("'{}' index", registry))?;
    }

    while need_retry(index.update())? {
        shell_status("Blocking", "waiting for lock on registry index")?;
        std::thread::sleep(REGISTRY_BACKOFF);
    }

    Ok(())
}

/// Time between retries for retrieving the registry.
const REGISTRY_BACKOFF: Duration = Duration::from_secs(1);

/// Check if we need to retry retrieving the Index.
fn need_retry(res: Result<(), crates_index::Error>) -> CargoResult<bool> {
    match res {
        Ok(()) => Ok(false),
        Err(crates_index::Error::Git(err)) => {
            if err.class() == git2::ErrorClass::Index && err.code() == git2::ErrorCode::Locked {
                Ok(true)
            } else {
                Err(crates_index::Error::Git(err).into())
            }
        }
        Err(err) => Err(err.into()),
    }
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
            yanked: false,
            available_features: BTreeMap::new(),
        },
        CrateVersion {
            name: "foo".into(),
            version: "0.5.0".parse().unwrap(),
            yanked: false,
            available_features: BTreeMap::new(),
        },
    ];
    assert_eq!(
        read_latest_version(&versions, false)
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
            yanked: false,
            available_features: BTreeMap::new(),
        },
        CrateVersion {
            name: "foo".into(),
            version: "0.5.0".parse().unwrap(),
            yanked: false,
            available_features: BTreeMap::new(),
        },
    ];
    assert_eq!(
        read_latest_version(&versions, true)
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
            yanked: true,
            available_features: BTreeMap::new(),
        },
        CrateVersion {
            name: "true".into(),
            version: "0.3.0".parse().unwrap(),
            yanked: false,
            available_features: BTreeMap::new(),
        },
    ];
    assert_eq!(
        read_latest_version(&versions, false)
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
            yanked: true,
            available_features: BTreeMap::new(),
        },
        CrateVersion {
            name: "true".into(),
            version: "0.3.0".parse().unwrap(),
            yanked: true,
            available_features: BTreeMap::new(),
        },
    ];
    assert!(read_latest_version(&versions, false).is_err());
}
