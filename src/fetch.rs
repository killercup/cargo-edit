use crate::{Dependency, Manifest};
use env_proxy;
use regex::Regex;
use reqwest;
use semver;
use serde_json as json;
use std::env;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

use crate::errors::*;

const REGISTRY_HOST: &str = "https://crates.io";

#[derive(Deserialize)]
struct Versions {
    versions: Vec<CrateVersion>,
}

#[derive(Deserialize)]
struct CrateVersion {
    #[serde(rename = "crate")]
    name: String,
    #[serde(rename = "num")]
    version: semver::Version,
    yanked: bool,
}

/// Query latest version from crates.io
///
/// The latest version will be returned as a `Dependency`. This will fail, when
///
/// - there is no Internet connection,
/// - the response from crates.io is an error or in an incorrect format,
/// - or when a crate with the given name does not exist on crates.io.
pub fn get_latest_dependency(crate_name: &str, flag_allow_prerelease: bool) -> Result<Dependency> {
    if env::var("CARGO_IS_TEST").is_ok() {
        // We are in a simulated reality. Nothing is real here.
        // FIXME: Use actual test handling code.
        let new_version = if flag_allow_prerelease {
            format!("{}--PRERELEASE_VERSION_TEST", crate_name)
        } else {
            format!("{}--CURRENT_VERSION_TEST", crate_name)
        };

        return Ok(Dependency::new(crate_name).set_version(&new_version));
    }

    let crate_versions = fetch_cratesio(crate_name)?;

    let dep = read_latest_version(&crate_versions, flag_allow_prerelease)?;

    if dep.name != crate_name {
        println!("WARN: Added `{}` instead of `{}`", dep.name, crate_name);
    }

    Ok(dep)
}

// Checks whether a version object is a stable release
fn version_is_stable(version: &CrateVersion) -> bool {
    !version.version.is_prerelease()
}

/// Read latest version from Versions structure
///
/// Assumes the version are sorted so that the first non-yanked version is the
/// latest, and thus the one we want.
fn read_latest_version(versions: &Versions, flag_allow_prerelease: bool) -> Result<Dependency> {
    let latest = versions
        .versions
        .iter()
        .filter(|&v| flag_allow_prerelease || version_is_stable(v))
        .find(|&v| !v.yanked)
        .ok_or(ErrorKind::NoVersionsAvailable)?;

    let name = &latest.name;
    let version = latest.version.to_string();
    Ok(Dependency::new(name).set_version(&version))
}

#[test]
fn get_latest_stable_version_from_json() {
    let versions: Versions = json::from_str(
        r#"{
      "versions": [
        {
          "crate": "foo",
          "num": "0.6.0-alpha",
          "yanked": false
        },
        {
          "crate": "foo",
          "num": "0.5.0",
          "yanked": false
        }
      ]
    }"#,
    )
    .expect("crate version is correctly parsed");

    assert_eq!(
        read_latest_version(&versions, false)
            .unwrap()
            .version()
            .unwrap(),
        "0.5.0"
    );
}

#[test]
fn get_latest_unstable_or_stable_version_from_json() {
    let versions: Versions = json::from_str(
        r#"{
      "versions": [
        {
          "crate": "foo",
          "num": "0.6.0-alpha",
          "yanked": false
        },
        {
          "crate": "foo",
          "num": "0.5.0",
          "yanked": false
        }
      ]
    }"#,
    )
    .expect("crate version is correctly parsed");

    assert_eq!(
        read_latest_version(&versions, true)
            .unwrap()
            .version()
            .unwrap(),
        "0.6.0-alpha"
    );
}

#[test]
fn get_latest_version_from_json_test() {
    let versions: Versions = json::from_str(
        r#"{
      "versions": [
        {
          "crate": "treexml",
          "num": "0.3.1",
          "yanked": true
        },
        {
          "crate": "treexml",
          "num": "0.3.0",
          "yanked": false
        }
      ]
    }"#,
    )
    .expect("crate version is correctly parsed");

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
    let versions: Versions = json::from_str(
        r#"{
      "versions": [
        {
          "crate": "treexml",
          "num": "0.3.1",
          "yanked": true
        },
        {
          "crate": "treexml",
          "num": "0.3.0",
          "yanked": true
        }
      ]
    }"#,
    )
    .expect("crate version is correctly parsed");

    assert!(read_latest_version(&versions, false).is_err());
}

fn fetch_cratesio(crate_name: &str) -> Result<Versions> {
    let url = format!(
        "{host}/api/v1/crates/{crate_name}",
        host = REGISTRY_HOST,
        crate_name = crate_name
    );

    match get_with_timeout(&url, get_default_timeout()) {
        Ok(response) => {
            Ok(json::from_reader(response).chain_err(|| ErrorKind::InvalidCratesIoJson)?)
        }
        Err(e) => {
            let not_found_error = e.status() == Some(reqwest::StatusCode::NOT_FOUND);

            Err(e).chain_err(|| {
                if not_found_error {
                    ErrorKind::NoCrate(crate_name.to_string())
                } else {
                    ErrorKind::FetchVersionFailure
                }
            })
        }
    }
}

fn get_crate_name_from_repository<T>(repo: &str, matcher: &Regex, url_template: T) -> Result<String>
where
    T: Fn(&str, &str) -> String,
{
    matcher
        .captures(repo)
        .ok_or_else(|| "Unable to parse git repo URL".into())
        .and_then(|cap| match (cap.get(1), cap.get(2)) {
            (Some(user), Some(repo)) => {
                let url = url_template(user.as_str(), repo.as_str());
                let data: Result<Manifest> = get_cargo_toml_from_git_url(&url)
                    .and_then(|m| m.parse().chain_err(|| ErrorKind::ParseCargoToml));
                data.and_then(|ref manifest| get_name_from_manifest(manifest))
            }
            _ => Err("Git repo url seems incomplete".into()),
        })
}

/// Query crate name by accessing a github repo Cargo.toml
///
/// The name will be returned as a string. This will fail, when
///
/// - there is no Internet connection,
/// - Cargo.toml is not present in the root of the master branch,
/// - the response from github is an error or in an incorrect format.
pub fn get_crate_name_from_github(repo: &str) -> Result<String> {
    let re =
        Regex::new(r"^https://github.com/([-_0-9a-zA-Z]+)/([-_0-9a-zA-Z]+)(/|.git)?$").unwrap();
    get_crate_name_from_repository(repo, &re, |user, repo| {
        format!(
            "https://raw.githubusercontent.com/{user}/{repo}/master/Cargo.toml",
            user = user,
            repo = repo
        )
    })
}

/// Query crate name by accessing a gitlab repo Cargo.toml
///
/// The name will be returned as a string. This will fail, when
///
/// - there is no Internet connection,
/// - Cargo.toml is not present in the root of the master branch,
/// - the response from gitlab is an error or in an incorrect format.
pub fn get_crate_name_from_gitlab(repo: &str) -> Result<String> {
    let re =
        Regex::new(r"^https://gitlab.com/([-_0-9a-zA-Z]+)/([-_0-9a-zA-Z]+)(/|.git)?$").unwrap();
    get_crate_name_from_repository(repo, &re, |user, repo| {
        format!(
            "https://gitlab.com/{user}/{repo}/raw/master/Cargo.toml",
            user = user,
            repo = repo
        )
    })
}

/// Query crate name by accessing Cargo.toml in a local path
///
/// The name will be returned as a string. This will fail, when
/// Cargo.toml is not present in the root of the path.
pub fn get_crate_name_from_path(path: &str) -> Result<String> {
    let cargo_file = Path::new(path).join("Cargo.toml");
    Manifest::open(&Some(cargo_file))
        .chain_err(|| "Unable to open local Cargo.toml")
        .and_then(|ref manifest| get_name_from_manifest(manifest))
}

fn get_name_from_manifest(manifest: &Manifest) -> Result<String> {
    manifest
        .data
        .as_table()
        .get("package")
        .and_then(|m| m["name"].as_str().map(std::string::ToString::to_string))
        .ok_or_else(|| ErrorKind::ParseCargoToml.into())
}

const fn get_default_timeout() -> Duration {
    Duration::from_secs(10)
}

fn get_with_timeout(url: &str, timeout: Duration) -> reqwest::Result<reqwest::Response> {
    let client = reqwest::ClientBuilder::new()
        .timeout(timeout)
        .proxy(reqwest::Proxy::custom(|url| {
            env_proxy::for_url(url).to_url()
        }))
        .build()?;

    client
        .get(url)
        .send()
        .and_then(reqwest::Response::error_for_status)
}

fn get_cargo_toml_from_git_url(url: &str) -> Result<String> {
    let mut res = get_with_timeout(url, get_default_timeout())
        .chain_err(|| "Failed to fetch crate from git")?;
    let mut body = String::new();
    res.read_to_string(&mut body)
        .chain_err(|| "Git response not a valid `String`")?;
    Ok(body)
}
