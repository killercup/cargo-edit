use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use cargo::CargoResult;
use regex::Regex;

use super::errors::*;
use super::{LocalManifest, Manifest};

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
