use cargo_edit::{Dependency, Manifest};
use curl;
use curl::easy::Easy;
use regex::Regex;
use rustc_serialize::json::{BuilderError, Json, Object};
use semver::Version;
use std::env;
use std::path::Path;

const REGISTRY_HOST: &'static str = "https://crates.io";

/// Query latest version from crates.io
///
/// The latest version will be returned as a `Dependency`. This will fail, when
///
/// - there is no Internet connection,
/// - the response from crates.io is an error or in an incorrect format,
/// - or when a crate with the given name does not exist on crates.io.
pub fn get_latest_dependency(crate_name: &str, flag_allow_prerelease: bool) -> Result<Dependency, FetchVersionError> {
    if env::var("CARGO_IS_TEST").is_ok() {
        // We are in a simulated reality. Nothing is real here.
        // FIXME: Use actual test handling code.
        return Ok(Dependency::new(crate_name)
            .set_version(&format!("{}--CURRENT_VERSION_TEST", crate_name)));
    }

    let crate_data = try!(fetch_cratesio(&format!("/crates/{}", crate_name)));
    let crate_json = try!(Json::from_str(&crate_data));

    let dep = try!(read_latest_version(crate_json, flag_allow_prerelease));

    if dep.name != crate_name {
        println!("WARN: Added `{}` instead of `{}`", dep.name, crate_name);
    }

    Ok(dep)
}

// Checks whether a version object is a stable release
fn version_is_stable(version: &Object) -> bool {
    version.get("num")
        .and_then(Json::as_string)
        .and_then(|s| Version::parse(s).ok())
        .map(|s| !s.is_prerelease())
        .unwrap_or(false)
}

/// Read latest version from JSON structure
///
/// Assumes the version are sorted so that the first non-yanked version is the
/// latest, and thus the one we want.
fn read_latest_version(crate_json: Json, flag_allow_prerelease: bool) -> Result<Dependency, FetchVersionError> {
    let versions = try!(crate_json.as_object()
        .and_then(|c| c.get("versions"))
        .and_then(Json::as_array)
        .ok_or(FetchVersionError::GetVersion));

    let latest = try!(versions.iter()
        .filter_map(Json::as_object)
        .filter(|&v| flag_allow_prerelease || version_is_stable(v))
        .find(|&v| !v.get("yanked").and_then(Json::as_boolean).unwrap_or(true))
        .ok_or(FetchVersionError::NoneAvailable));

    let name = try!(latest.get("crate")
        .and_then(Json::as_string)
        .map(String::from)
        .ok_or(FetchVersionError::NotFound));

    let version = try!(latest.get("num")
        .and_then(Json::as_string)
        .map(String::from)
        .ok_or(FetchVersionError::GetVersion));

    Ok(Dependency::new(&name).set_version(&version))
}

#[test]
fn get_latest_stable_version_from_json() {
    let json = Json::from_str(r#"{
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
    }"#)
        .unwrap();

    assert_eq!(read_latest_version(json, false).unwrap().version().unwrap(),
               "0.5.0");
}

#[test]
fn get_latest_unstable_or_stable_version_from_json() {
    let json = Json::from_str(r#"{
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
    }"#)
        .unwrap();

    assert_eq!(read_latest_version(json, true).unwrap().version().unwrap(),
               "0.6.0-alpha");
}

#[test]
fn get_latest_version_from_json_test() {
    let json = Json::from_str(r#"{
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
    }"#)
        .unwrap();

    assert_eq!(read_latest_version(json, false).unwrap().version().unwrap(),
               "0.3.0");
}

#[test]
fn get_no_latest_version_from_json_when_all_are_yanked() {
    let json = Json::from_str(r#"{
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
    }"#)
        .unwrap();

    assert!(read_latest_version(json, false).is_err());
}

quick_error! {
    #[derive(Debug)]
    pub enum FetchVersionError {
        Curl(err: curl::Error) {
            from()
            description("Curl error")
            display("Curl error: {}", err)
            cause(err)
        }
        NonUtf8(err: ::std::string::FromUtf8Error) {
            from()
            description("Curl error")
            display("Curl error: {}", err)
            cause(err)
        }
        NotFound {}
        Json(err: BuilderError) {
            from()
            description("JSON Error")
            display("Error parsing JSON: {}", err)
            cause(err)
        }
        GetVersion { description("get version error") }
        NoneAvailable { description("No available versions exist. Either all were yanked or only \
            prerelease versions exist. Trying with the --fetch-prereleases flag might solve \
            the issue.") }
    }
}

// ---
// The following was mostly copied from [1] and is therefore
// (c) 2015 Alex Crichton <alex@alexcrichton.com>
//
// [1]: https://github.com/rust-lang/cargo/blob/bd690d8dff83c7b7714f236a08304ee20732382b/src/crates-io/lib.rs
// ---

fn fetch_cratesio(path: &str) -> Result<String, FetchVersionError> {
    let mut easy = Easy::new();
    easy.url(&format!("{}/api/v1{}", REGISTRY_HOST, path))?;
    easy.get(true)?;
    easy.accept_encoding("application/json")?;

    let mut html = Vec::new();
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|data| {
            html.extend_from_slice(data);
                    Ok(data.len())
        })?;


        transfer.perform()?;
    }

    String::from_utf8(html).map_err(FetchVersionError::NonUtf8)
}

#[cfg_attr(rustfmt, rustfmt_skip)]
quick_error! {
    #[derive(Debug)]
    pub enum FetchGitError {
        FetchGit(err: curl::Error) {
            from()
            description("fetch error: ")
            display("fetch error: {}", err)
            cause(err)
        }
        StringWrite(err: curl::easy::WriteError) {
            from()
            description("string write error: ")
            display("string write error: {:?}", err)
        }
        NonUtf8(err: ::std::string::FromUtf8Error) {
            from()
            description("Curl error")
            display("Curl error: {}", err)
            cause(err)
        }
        ParseRegex { description("parse error: unable to parse git repo url") }
        IncompleteCaptures { description("parse error: the git repo url seems incomplete") }
        LocalCargoToml { description("path error: unable to open Cargo.toml") }
        // RemoteCargoToml { description("path error: unable to open Cargo.toml from the provided repo") }
        ParseCargoToml { description("parse error: unable to parse the external Cargo.toml") }
    }
}

/// Query crate name by accessing a github repo Cargo.toml
///
/// The name will be returned as a string. This will fail, when
///
/// - there is no Internet connection,
/// - Cargo.toml is not present in the root of the master branch,
/// - the response from github is an error or in an incorrect format.
pub fn get_crate_name_from_github(repo: &str) -> Result<String, FetchGitError> {
    let re = Regex::new(r"^https://github.com/([-_0-9a-zA-Z]+)/([-_0-9a-zA-Z]+)(/|.git)?$")
        .unwrap();

    re.captures(repo)
        .ok_or(FetchGitError::ParseRegex)
        .and_then(|cap| {
            match (cap.get(1), cap.get(2)) {
                (Some(user), Some(repo)) => {
                    let url = format!("https://raw.githubusercontent.com/{}/{}/master/Cargo.toml",
                                      user.as_str(),
                                      repo.as_str());

                    let data: Result<Manifest, _> = get_cargo_toml_from_git_url(&url)
                        .and_then(|m| {
                            m.parse()
                                .map_err(|_| FetchGitError::ParseCargoToml)
                        });
                    data.and_then(|ref manifest| get_name_from_manifest(manifest))
                }
                _ => Err(FetchGitError::IncompleteCaptures),
            }
        })
}

/// Query crate name by accessing a gitlab repo Cargo.toml
///
/// The name will be returned as a string. This will fail, when
///
/// - there is no Internet connection,
/// - Cargo.toml is not present in the root of the master branch,
/// - the response from gitlab is an error or in an incorrect format.
pub fn get_crate_name_from_gitlab(repo: &str) -> Result<String, FetchGitError> {
    let re = Regex::new(r"^https://gitlab.com/([-_0-9a-zA-Z]+)/([-_0-9a-zA-Z]+)(/|.git)?$")
        .unwrap();

    re.captures(repo)
        .ok_or(FetchGitError::ParseRegex)
        .and_then(|cap| {
            match (cap.get(1), cap.get(2)) {
                (Some(user), Some(repo)) => {
                    let url = format!("https://gitlab.com/{}/{}/raw/master/Cargo.toml",
                                      user.as_str(),
                                      repo.as_str());

                    let data: Result<Manifest, _> = get_cargo_toml_from_git_url(&url)
                        .and_then(|m| {
                            m.parse()
                                .map_err(|_| FetchGitError::ParseCargoToml)
                        });
                    data.and_then(|ref manifest| get_name_from_manifest(manifest))
                }
                _ => Err(FetchGitError::IncompleteCaptures),
            }
        })
}

/// Query crate name by accessing Cargo.toml in a local path
///
/// The name will be returned as a string. This will fail, when
/// Cargo.toml is not present in the root of the path.
pub fn get_crate_name_from_path(path: &str) -> Result<String, FetchGitError> {
    let cargo_file = Path::new(path).join("Cargo.toml");
    Manifest::open(&cargo_file.to_str())
        .map_err(|_| FetchGitError::LocalCargoToml)
        .and_then(|ref manifest| get_name_from_manifest(manifest))
}

fn get_name_from_manifest(manifest: &Manifest) -> Result<String, FetchGitError> {
    manifest.data
        .get("package")
        .and_then(|m| m.get("name"))
        .and_then(|name| name.as_str().map(|s| s.to_string()))
        .ok_or(FetchGitError::ParseCargoToml)
}

fn get_cargo_toml_from_git_url(url: &str) -> Result<String, FetchGitError> {
    let mut easy = Easy::new();
    easy.url(url)?;
    easy.get(true)?;
    easy.accept_encoding("text/plain")?;

    let mut html = Vec::new();
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|data| {
            html.extend_from_slice(data);
                    Ok(data.len())
        })?;


        transfer.perform()?;
    }

    String::from_utf8(html).map_err(FetchGitError::NonUtf8)
}
