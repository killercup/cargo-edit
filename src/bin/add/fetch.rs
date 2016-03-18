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


/// Query crate name by accessing a github repo Cargo.toml
///
/// The name will be returned as a string. This will fail, when
///
/// - there is no Internet connection,
/// - Cargo.toml is not present in the root of the master branch,
/// - the response from github is an error or in an incorrect format.
pub fn get_crate_name_from_github(repo: &str) -> Option<String> {
    let re = Regex::new(r"^https://github.com/([-_0-9a-zA-Z]+)/([-_0-9a-zA-Z]+)/?$").unwrap();

    re.captures(repo).and_then(|cap| {
        match (cap.at(1), cap.at(2)) {
            (Some(ref user), Some(ref repo)) => {
                let url = format!("https://raw.githubusercontent.com/{}/{}/master/Cargo.toml",
                                  user,
                                  repo);

                // FIXME: use Result or modify get_cargo_toml_from_git_url to return Option
                let data: Option<Manifest> = get_cargo_toml_from_git_url(&url)
                                                 .ok()
                                                 .and_then(|m| m.parse().ok());
                data.and_then(|ref manifest| get_name_from_manifest(manifest))
            }
            _ => None,
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
pub fn get_crate_name_from_gitlab(repo: &str) -> Option<String> {
    let re = Regex::new(r"^https://gitlab.com/([-_0-9a-zA-Z]+)/([-_0-9a-zA-Z]+)(/|.git)?$")
                 .unwrap();

    re.captures(repo).and_then(|cap| {
        match (cap.at(1), cap.at(2)) {
            (Some(ref user), Some(ref repo)) => {
                let url = format!("https://gitlab.com/{}/{}/raw/master/Cargo.toml", user, repo);

                // FIXME: use Result or modify get_cargo_toml_from_git_url to return Option
                let data: Option<Manifest> = get_cargo_toml_from_git_url(&url)
                                                 .ok()
                                                 .and_then(|m| m.parse().ok());
                data.and_then(|ref manifest| get_name_from_manifest(manifest))
            }
            _ => None,
        }
    })
}

/// Query crate name by accessing Cargo.toml in a local path
///
/// The name will be returned as a string. This will fail, when
/// Cargo.toml is not present in the root of the path.
pub fn get_crate_name_from_path(path: &str) -> Option<String> {
    let cargo_file = Path::new(path).join("Cargo.toml");
    Manifest::open(&cargo_file.to_str())
        .ok()
        .and_then(|ref manifest| get_name_from_manifest(manifest))
}

fn get_name_from_manifest(manifest: &Manifest) -> Option<String> {
    manifest.data
            .get("package")
            .and_then(|m| m.lookup("name"))
            .and_then(|name| name.as_str().map(|s| s.to_string()))
}

// FIXME: between the code above and below there is a lot of duplication.
// FIXME: make a generic version of CratesIOError to use in all cases.
// I am ignoring these FIXME waiting for the new fetch implementation using hyper to be merged
fn get_cargo_toml_from_git_url(url: &str) -> Result<String, FetchGitError> {
    if env::var("CARGO_IS_TEST").is_ok() {
        // We are in a simulated reality. Nothing is real here.
        // FIXME: Use actual test handling code.
        return Ok("CURRENT_VERSION_TEST".into());
    }
    fetch_git_url(url).or(Err(FetchGitError::GitIoError))
}

quick_error! {
    #[derive(Debug)]
    pub enum FetchGitError {
        FetchGit(err: CratesIoError) {
            from()
            description("git fetch Error")
            display("git fetch Error: {}", err)
            cause(err)
        }
        GitIoError { description("unable to download Cargo.toml from the provided git repo") }
    }
}

fn fetch_git_url(path: &str) -> Result<String, FetchGitError> {
    let mut http_handle = http::Handle::new();
    let req = Request::new(&mut http_handle, Method::Get)
                  .uri(path)
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