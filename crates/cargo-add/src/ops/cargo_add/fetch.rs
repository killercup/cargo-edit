use std::collections::BTreeMap;
use std::env;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use cargo::CargoResult;
use regex::Regex;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use url::Url;

use super::errors::*;
use super::registry::registry_url;
use super::RegistrySource;
use super::VersionExt;
use super::{Dependency, LocalManifest, Manifest};

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
    work_dir: &Path,
    registry: Option<&str>,
) -> CargoResult<Dependency> {
    if crate_name.is_empty() {
        anyhow::bail!("Found empty crate name");
    }

    let mut dep = if env::var("CARGO_IS_TEST").is_ok() {
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

        Dependency::new(crate_name)
            .set_source(RegistrySource::new(new_version))
            .set_available_features(features)
    } else {
        let registry_url = registry_url(work_dir, registry)?;
        let crate_versions = fuzzy_query_registry_index(crate_name, &registry_url)?;
        read_latest_version(&crate_versions, flag_allow_prerelease)?
    };

    if let Some(registry) = registry {
        dep = dep.set_registry(registry);
    }

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
/// | cargo-edit | cargo-edit, cargo_add |
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
        .set_source(RegistrySource::new(version))
        .set_available_features(latest.available_features.clone()))
}

/// Get crate features from registry
pub fn get_features_from_registry(
    crate_name: &str,
    version: &str,
    registry: &Url,
) -> CargoResult<BTreeMap<String, Vec<String>>> {
    if env::var("CARGO_IS_TEST").is_ok() {
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
        return Ok(features);
    }

    let index = crates_index::Index::from_url(registry.as_str())?;
    let version =
        semver::VersionReq::parse(version).map_err(|_| parse_version_err(version, crate_name))?;

    let crate_ = index
        .crate_(crate_name)
        .ok_or_else(|| no_crate_err(crate_name))?;
    for crate_instance in crate_.versions().iter().rev() {
        let instance_version = match semver::Version::parse(crate_instance.version()) {
            Ok(version) => version,
            Err(_) => continue,
        };
        if version.matches(&instance_version) {
            return Ok(registry_features(crate_instance));
        }
    }
    Ok(registry_features(crate_.highest_version()))
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
    let colorchoice = colorize_stderr();
    let mut output = StandardStream::stderr(colorchoice);

    let mut index = crates_index::Index::from_url(registry.as_str())?;
    if !quiet {
        output.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
        write!(output, "{:>12}", "Updating")?;
        output.reset()?;
        writeln!(output, " '{}' index", registry)?;
    }

    while need_retry(index.update())? {
        registry_blocked_message(&mut output)?;
        std::thread::sleep(REGISTRY_BACKOFF);
    }

    Ok(())
}

fn colorize_stderr() -> termcolor::ColorChoice {
    if concolor_control::get(concolor_control::Stream::Stderr).color() {
        termcolor::ColorChoice::Always
    } else {
        termcolor::ColorChoice::Never
    }
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

/// Report to user that the Registry is locked
fn registry_blocked_message(output: &mut StandardStream) -> CargoResult<()> {
    output.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    write!(output, "{:>12}", "Blocking")?;
    output.reset()?;
    writeln!(output, " waiting for lock on registry index")?;
    Ok(())
}

/// Load Cargo.toml in a local path
///
/// This will fail, when Cargo.toml is not present in the root of the path.
pub fn get_manifest_from_path(path: &Path) -> CargoResult<LocalManifest> {
    let cargo_file = path.join("Cargo.toml");
    LocalManifest::try_new(&cargo_file).with_context(|| "Unable to open local Cargo.toml")
}

/// Load Cargo.toml from  github repo Cargo.toml
///
/// This will fail when:
/// - there is no Internet connection,
/// - Cargo.toml is not present in the root of the master branch,
/// - the response from the server is an error or in an incorrect format.
pub fn get_manifest_from_url(url: &str) -> CargoResult<Option<Manifest>> {
    let manifest = if is_github_url(url) {
        Some(get_manifest_from_github(url)?)
    } else if is_gitlab_url(url) {
        Some(get_manifest_from_gitlab(url)?)
    } else {
        None
    };
    Ok(manifest)
}

fn is_github_url(url: &str) -> bool {
    url.contains("https://github.com")
}

fn is_gitlab_url(url: &str) -> bool {
    url.contains("https://gitlab.com")
}

fn get_manifest_from_github(repo: &str) -> CargoResult<Manifest> {
    let re =
        Regex::new(r"^https://github.com/([-_0-9a-zA-Z]+)/([-_0-9a-zA-Z]+)(/|.git)?$").unwrap();
    get_manifest_from_repository(repo, &re, |user, repo| {
        format!(
            "https://raw.githubusercontent.com/{user}/{repo}/master/Cargo.toml",
            user = user,
            repo = repo
        )
    })
}

fn get_manifest_from_gitlab(repo: &str) -> CargoResult<Manifest> {
    let re =
        Regex::new(r"^https://gitlab.com/([-_0-9a-zA-Z]+)/([-_0-9a-zA-Z]+)(/|.git)?$").unwrap();
    get_manifest_from_repository(repo, &re, |user, repo| {
        format!(
            "https://gitlab.com/{user}/{repo}/raw/master/Cargo.toml",
            user = user,
            repo = repo
        )
    })
}

fn get_manifest_from_repository<T>(
    repo: &str,
    matcher: &Regex,
    url_template: T,
) -> CargoResult<Manifest>
where
    T: Fn(&str, &str) -> String,
{
    matcher
        .captures(repo)
        .ok_or_else(|| anyhow::format_err!("Unable to parse git repo URL"))
        .and_then(|cap| match (cap.get(1), cap.get(2)) {
            (Some(user), Some(repo)) => {
                let url = url_template(user.as_str(), repo.as_str());
                get_cargo_toml_from_git_url(&url)
                    .and_then(|m| m.parse().with_context(parse_manifest_err))
            }
            _ => Err(anyhow::format_err!("Git repo url seems incomplete")),
        })
}

fn get_cargo_toml_from_git_url(url: &str) -> CargoResult<String> {
    let mut agent = ureq::AgentBuilder::new().timeout(get_default_timeout());
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "arm",
        target_arch = "x86",
        target_arch = "aarch64"
    )))]
    {
        use std::sync::Arc;

        let tls_connector = Arc::new(native_tls::TlsConnector::new().map_err(|e| e.to_string())?);
        agent = agent.tls_connector(tls_connector.clone());
    }
    if let Some(proxy) = env_proxy::for_url_str(url)
        .to_url()
        .and_then(|url| ureq::Proxy::new(url).ok())
    {
        agent = agent.proxy(proxy);
    }
    let req = agent.build().get(url);
    let res = req.call();
    match res {
        Ok(res) => res
            .into_string()
            .with_context(|| "Git response not a valid `String`"),
        Err(err) => Err(anyhow::format_err!(
            "HTTP request `{}` failed: {}",
            url,
            err
        )),
    }
}

const fn get_default_timeout() -> Duration {
    Duration::from_secs(10)
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
