use std::env;
use rustc_serialize::json;
use rustc_serialize::json::{BuilderError, Json};
use curl::{ErrCode, http};
use curl::http::handle::{Method, Request};

const REGISTRY_HOST: &'static str = "https://crates.io";

/// Query latest version fromc crates.io
///
/// The latest version will be returned as a string. This will fail, when
///
/// - there is no Internet connection,
/// - the response from crates.io was an error or in an incorrect format,
/// - or when a crate with the given name does not exist on crates.io.
pub fn get_latest_version(crate_name: &str) -> Result<String, FetchVersionError> {
    if env::var("CARGO_IS_TEST").is_ok() {
        // We are in a simulated reality. Nothing is real here.
        // FIXME: Use actual test handling code.
        return Ok("CURRENT_VERSION_TEST".into());
    }

    let crate_data = try!(fetch(&format!("/crates/{}", crate_name)));
    let crate_json = try!(Json::from_str(&crate_data));

    // issue 51
    // return error if name in crates.io is different from what we have
    let not_found = Err(FetchVersionError::CratesIo(CratesIoError::NotFound));
    match crate_json.find_path(&["crate", "name"]).and_then(|n| n.as_string()) {
        Some(name) => {
            if name != crate_name {
                return not_found;
            }
        }
        None => return not_found,
    }

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

fn fetch(path: &str) -> Result<String, FetchVersionError> {
    let mut http_handle = http::Handle::new();
    let req = Request::new(&mut http_handle, Method::Get)
                  .uri(format!("{}/api/v1{}", REGISTRY_HOST, path))
                  .header("Accept", "application/json")
                  .content_type("application/json");
    handle(req.exec()).map_err(From::from)
}

fn handle(response: Result<http::Response, ErrCode>) -> Result<String, CratesIoError> {
    let response = try!(response.map_err(CratesIoError::Curl));
    match response.get_code() {
        0 => {} // file upload url sometimes
        200 => {}
        403 => return Err(CratesIoError::Unauthorized),
        404 => return Err(CratesIoError::NotFound),
        _ => return Err(CratesIoError::NotOkResponse(response)),
    }

    let body = match String::from_utf8(response.move_body()) {
        Ok(body) => body,
        Err(..) => return Err(CratesIoError::NonUtf8Body),
    };
    match json::decode::<ApiErrorList>(&body) {
        Ok(errors) => {
            return Err(CratesIoError::Api(errors.errors.into_iter().map(|s| s.detail).collect()));
        }
        Err(..) => {}
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

#[cfg(test)]
mod tests {
    use super::get_latest_version;

    #[test]
    fn invalid_crate_name() {
        assert!(match get_latest_version("error-def") {
            Ok(_) => false,
            Err(_) => true,
        });
    }

    #[test]
    fn valid_crate_name() {
        assert!(match get_latest_version("error_def") {
            Ok(_) => true,
            Err(_) => false,
        });
    }
}
