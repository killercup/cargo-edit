use std::env;
use std::io::Read;
use rustc_serialize::json::{BuilderError, Json};
use hyper::Client;
use hyper::header::{Connection, ContentType};
use hyper::error::Error as HyperError;
use hyper::status::StatusCode;

const REGISTRY_HOST: &'static str = "https://crates.io";

/// Query latest version fromc crates.io
///
/// The latest version will be returned as a string. This will fail, when
///
/// - there is no Internet connection,
/// - the response from crates.io was an error or in an incorrect format,
/// - or when a crate with the given name does not exist on crates.io.
#[allow(trivial_casts)]
pub fn get_latest_version(crate_name: &str) -> Result<String, FetchVersionError> {
    if env::var("CARGO_IS_TEST").is_ok() {
        // We are in a simulated reality. Nothing is real here. Wildcard dependecies are okay.
        // FIXME: Use actual test handling code.
        return Ok("*".into());
    }

    let client = Client::new();
    let mut res = try!(
        client.get(&format!("{}/api/v1/crates/{}", REGISTRY_HOST, crate_name))
        .header(Connection::close())
        .header(ContentType::json())
        .send());

    if !res.status.is_success() {
        return Err(FetchVersionError::CratesIo(res.status));
    }

    // FIXME: Trivial cast
    let json = try!(Json::from_reader(&mut res as &mut Read));

    json.find_path(&["crate", "max_version"])
        .and_then(|v| v.as_string())
        .map(|v| v.to_owned())
        .ok_or(FetchVersionError::GetVersion)
}

quick_error! {
    #[derive(Debug)]
    pub enum FetchVersionError {
        Http(err: HyperError) {
            from()
            cause(err)
        }
        CratesIo(err: StatusCode) {
            from()
            description("crates.io Error")
            display("crates.io Error: {}", err)
        }
        Json(err: BuilderError) {
            from()
            description("JSON Error")
            display("Error parsing JSON: {}", err)
            cause(err)
        }
        GetVersion {
            description("get version error")
        }
    }
}
