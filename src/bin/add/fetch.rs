use std::env;
use rustc_serialize::json;
use rustc_serialize::json::{BuilderError, Json};
use curl::{ErrCode, http};
use curl::http::handle::{Method, Request};
use cargo_edit::Manifest;
use regex::Regex;
use std::path::Path;

const REGISTRY_HOST: &'static str = "https://crates.io";

/// Query latest version from crates.io
///
/// The latest version will be returned as a string. This will fail, when
///
/// - there is no Internet connection,
/// - the response from crates.io is an error or in an incorrect format,
/// - or when a crate with the given name does not exist on crates.io.
pub fn get_latest_version(crate_name: &str) -> Result<String, FetchVersionError> {
    if env::var("CARGO_IS_TEST").is_ok() {
        // We are in a simulated reality. Nothing is real here.
        // FIXME: Use actual test handling code.
        return Ok("CURRENT_VERSION_TEST".into());
    }

    let crate_data = try!(fetch_cratesio(&format!("/crates/{}", crate_name)));
    let crate_json = try!(Json::from_str(&crate_data));

    crate_json.as_object()
              .and_then(|c| c.get("crate"))
              .and_then(|c| c.as_object())
              .and_then(|c| c.get("max_version"))
              .and_then(|v| v.as_string())
              .map(|v| v.to_owned())
              .ok_or(FetchVersionError::GetVersion)
}

quick_error! {
    #[derive(Debug)]
    pub enum FetchVersionError {
        CratesIo(err: CratesIoError) {
            from()
            description("crates.io Error")
            display("crates.io Error: {}", err)
            cause(err)
        }
        Json(err: BuilderError) {
            from()
            description("JSON Error")
            display("Error parsing JSON: {}", err)
            cause(err)
        }
        GetVersion { description("get version error") }
    }
}

// ---
// The following was mostly copied from [1] and is therefore
// (c) 2015 Alex Crichton <alex@alexcrichton.com>
//
// [1]: https://github.com/rust-lang/cargo/blob/bd690d8dff83c7b7714f236a08304ee20732382b/src/crates-io/lib.rs
// ---

fn fetch_cratesio(path: &str) -> Result<String, FetchVersionError> {
    let mut http_handle = http::Handle::new();
    let req = Request::new(&mut http_handle, Method::Get)
                  .uri(format!("{}/api/v1{}", REGISTRY_HOST, path))
                  .header("Accept", "application/json")
                  .content_type("application/json");
    handle_cratesio(req.exec()).map_err(From::from)
}

fn handle_cratesio(response: Result<http::Response, ErrCode>) -> Result<String, CratesIoError> {
    let response = try!(response.map_err(CratesIoError::Curl));
    match response.get_code() {
        0 | 200 => {}
        403 => return Err(CratesIoError::Unauthorized),
        404 => return Err(CratesIoError::NotFound),
        _ => return Err(CratesIoError::NotOkResponse(response)),
    }

    let body = match String::from_utf8(response.move_body()) {
        Ok(body) => body,
        Err(..) => return Err(CratesIoError::NonUtf8Body),
    };

    if let Ok(errors) = json::decode::<ApiErrorList>(&body) {
        return Err(CratesIoError::Api(errors.errors.into_iter().map(|s| s.detail).collect()));
    }

    Ok(body)
}

#[derive(RustcDecodable)]
struct ApiErrorList {
    errors: Vec<ApiError>,
}
#[derive(RustcDecodable)]
struct ApiError {
    detail: String,
}

quick_error! {
    #[derive(Debug)]
    pub enum CratesIoError {
        Curl(e: ErrCode) {}
        NotOkResponse(e: http::Response)  {}
        NonUtf8Body  {}
        Api(e: Vec<String>)  {}
        Unauthorized  {}
        NotFound {}
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum FetchGitError {
        FetchGit(err: CratesIoError) {
            from()
            description("fetch error: ")
            display("fetch error: {}", err)
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
          match (cap.at(1), cap.at(2)) {
              (Some(ref user), Some(ref repo)) => {
                  let url = format!("https://raw.githubusercontent.com/{}/{}/master/Cargo.toml",
                                    user,
                                    repo);

                  let data: Result<Manifest, _> = get_cargo_toml_from_git_url(&url).and_then(|m| {
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
          match (cap.at(1), cap.at(2)) {
              (Some(ref user), Some(ref repo)) => {
                  let url = format!("https://gitlab.com/{}/{}/raw/master/Cargo.toml", user, repo);

                  let data: Result<Manifest, _> = get_cargo_toml_from_git_url(&url).and_then(|m| {
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
            .and_then(|m| m.lookup("name"))
            .and_then(|name| name.as_str().map(|s| s.to_string()))
            .ok_or(FetchGitError::ParseCargoToml)
}

// FIXME: between the code above and below there is a lot of duplication.
fn get_cargo_toml_from_git_url(url: &str) -> Result<String, FetchGitError> {

    let mut http_handle = http::Handle::new();
    let req = Request::new(&mut http_handle, Method::Get)
                  .uri(url)
                  .header("Accept", "text/plain")
                  .content_type("text/plain");
    handle_git_url(req.exec()).map_err(From::from)
}

fn handle_git_url(response: Result<http::Response, ErrCode>) -> Result<String, CratesIoError> {
    let response = try!(response.map_err(CratesIoError::Curl));
    match response.get_code() {
        0 | 200 => {}
        403 => return Err(CratesIoError::Unauthorized),
        404 => return Err(CratesIoError::NotFound),
        _ => return Err(CratesIoError::NotOkResponse(response)),
    }

    let body = match String::from_utf8(response.move_body()) {
        Ok(body) => body,
        Err(..) => return Err(CratesIoError::NonUtf8Body),
    };

    if let Ok(errors) = json::decode::<ApiErrorList>(&body) {
        return Err(CratesIoError::Api(errors.errors.into_iter().map(|s| s.detail).collect()));
    }

    Ok(body)
}
