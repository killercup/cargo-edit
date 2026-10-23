use tame_index::flock::FileLock;
use tame_index::krate::IndexKrate;

use url::Url;

use super::errors::{CargoResult, Context};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertsSource {
    /// Use certs from Mozilla's root certificate store.
    #[default]
    Webpki,
    /// Use certs from the system root certificate store.
    Native,
}

pub struct IndexCache {
    certs_source: CertsSource,
    /// Keyed by registry URL *and* auth, so a token configured for one registry
    /// is never reused for (or withheld from) another that shares its index URL.
    index: std::collections::HashMap<(Url, Option<String>), AnyIndexCache>,
}

impl IndexCache {
    #[inline]
    pub fn new(certs_source: CertsSource) -> Self {
        Self {
            certs_source,
            index: Default::default(),
        }
    }

    /// Determines if the specified crate exists in the crates.io index
    #[inline]
    pub fn has_krate(
        &mut self,
        registry: &Url,
        name: &str,
        auth: Option<&str>,
    ) -> CargoResult<bool> {
        self.index(registry, auth)
            .with_context(|| format!("failed to look up {name}"))?
            .has_krate(name)
    }

    /// Determines if the specified crate version exists in the crates.io index
    #[inline]
    pub fn has_krate_version(
        &mut self,
        registry: &Url,
        name: &str,
        version: &str,
        auth: Option<&str>,
    ) -> CargoResult<Option<bool>> {
        self.index(registry, auth)
            .with_context(|| format!("failed to look up {name}@{version}"))?
            .has_krate_version(name, version)
    }

    #[inline]
    pub fn update_krate(
        &mut self,
        registry: &Url,
        name: &str,
        auth: Option<&str>,
    ) -> CargoResult<()> {
        self.index(registry, auth)
            .with_context(|| format!("failed to look up {name}"))?
            .update_krate(name);
        Ok(())
    }

    pub fn krate(
        &mut self,
        registry: &Url,
        name: &str,
        auth: Option<&str>,
    ) -> CargoResult<Option<IndexKrate>> {
        self.index(registry, auth)
            .with_context(|| format!("failed to look up {name}"))?
            .krate(name)
    }

    fn index<'s>(
        &'s mut self,
        registry: &Url,
        auth: Option<&str>,
    ) -> CargoResult<&'s mut AnyIndexCache> {
        let key = (registry.clone(), auth.map(str::to_owned));
        if !self.index.contains_key(&key) {
            let index = AnyIndex::open(registry, self.certs_source, auth)?;
            let index = AnyIndexCache::new(index);
            self.index.insert(key.clone(), index);
        }
        Ok(self.index.get_mut(&key).unwrap())
    }
}

struct AnyIndexCache {
    index: AnyIndex,
    cache: std::collections::HashMap<String, Option<IndexKrate>>,
}

impl AnyIndexCache {
    #[inline]
    fn new(index: AnyIndex) -> Self {
        Self {
            index,
            cache: std::collections::HashMap::new(),
        }
    }

    /// Determines if the specified crate exists in the crates.io index
    #[inline]
    fn has_krate(&mut self, name: &str) -> CargoResult<bool> {
        Ok(self.krate(name)?.map(|_| true).unwrap_or(false))
    }

    /// Determines if the specified crate version exists in the crates.io index
    #[inline]
    fn has_krate_version(&mut self, name: &str, version: &str) -> CargoResult<Option<bool>> {
        let krate = self.krate(name)?;
        Ok(krate.map(|ik| ik.versions.iter().any(|iv| iv.version == version)))
    }

    #[inline]
    fn update_krate(&mut self, name: &str) {
        self.cache.remove(name);
    }

    fn krate(&mut self, name: &str) -> CargoResult<Option<IndexKrate>> {
        if let Some(entry) = self.cache.get(name) {
            return Ok(entry.clone());
        }

        let entry = self.index.krate(name)?;
        self.cache.insert(name.to_owned(), entry.clone());
        Ok(entry)
    }
}

enum AnyIndex {
    Local(LocalIndex),
    Remote(RemoteIndex),
}

impl AnyIndex {
    fn open(url: &Url, certs_source: CertsSource, auth: Option<&str>) -> CargoResult<Self> {
        if url.scheme() == "file" {
            LocalIndex::open(url)
                .map(Self::Local)
                .with_context(|| format!("invalid local registry {url:?}"))
        } else {
            RemoteIndex::open(url, certs_source, auth)
                .map(Self::Remote)
                .with_context(|| format!("invalid registry {url:?}"))
        }
    }

    fn krate(&mut self, name: &str) -> CargoResult<Option<IndexKrate>> {
        match self {
            Self::Local(index) => index.krate(name),
            Self::Remote(index) => index.krate(name),
        }
    }
}

struct LocalIndex {
    index: tame_index::index::LocalIndex,
}

impl LocalIndex {
    fn open(url: &Url) -> CargoResult<Self> {
        let path = url
            .to_file_path()
            .map_err(|_err| anyhow::format_err!("invalid file path {url:?}"))?;
        let path = tame_index::PathBuf::from_path_buf(path)
            .map_err(|_err| anyhow::format_err!("invalid file path {url:?}"))?;
        Ok(Self {
            index: tame_index::index::LocalIndex::open(path)?,
        })
    }

    fn krate(&mut self, name: &str) -> CargoResult<Option<IndexKrate>> {
        let l = FileLock::unlocked();
        let Some(entry) = self.index.krate(name.try_into()?, &l)? else {
            return Ok(None);
        };

        Ok(entry.deserialize().map(Some)?)
    }
}

struct RemoteIndex {
    index: tame_index::SparseIndex,
    client: ureq::Agent,
    lock: FileLock,
    etag_cache: tame_index::EtagCache,
    /// Value for the `Authorization` header, sent verbatim (matching cargo) for
    /// registries with `auth-required = true`. `None` for unauthenticated registries.
    auth: Option<String>,
}

impl RemoteIndex {
    fn open(url: &Url, certs_source: CertsSource, auth: Option<&str>) -> CargoResult<Self> {
        log::trace!("opening index entry for {url}");
        let url = url.to_string();
        let url = tame_index::IndexUrl::NonCratesIo(std::borrow::Cow::Owned(url));
        let index = tame_index::SparseIndex::new(tame_index::IndexLocation::new(url))?;

        let client = ureq::Agent::config_builder()
            // tame_index handles HTTP status codes internally
            .http_status_as_error(false)
            .tls_config(
                ureq::tls::TlsConfig::builder()
                    .provider(ureq::tls::TlsProvider::Rustls)
                    .root_certs(match certs_source {
                        CertsSource::Webpki => ureq::tls::RootCerts::WebPki,
                        CertsSource::Native => ureq::tls::RootCerts::PlatformVerifier,
                    })
                    .build(),
            )
            .build()
            .new_agent();

        let lock = FileLock::unlocked();

        Ok(Self {
            index,
            client,
            lock,
            etag_cache: Default::default(),
            auth: auth.map(str::to_owned),
        })
    }

    fn krate(&mut self, name: &str) -> CargoResult<Option<IndexKrate>> {
        log::trace!("krate {name}");
        let krate_name = name.try_into()?;
        let req = self
            .etag_cache
            .make_remote_request(&self.index, krate_name, &self.lock)?;

        let mut req: ureq::http::Request<()> = req.into();
        // tame_index's `make_remote_request` does not attach credentials; a
        // registry with `auth-required = true` rejects the request with 401
        // otherwise. cargo sends the configured token verbatim.
        if let Some(auth) = &self.auth {
            let value = ureq::http::HeaderValue::from_str(auth)
                .context("invalid Authorization header value")?;
            req.headers_mut()
                .insert(ureq::http::header::AUTHORIZATION, value);
        }

        let (parts, mut body) = self.client.run(req)?.into_parts();
        let body = body.read_to_vec().context("failed to read response body")?;

        self.etag_cache
            .parse_remote_response(
                &self.index,
                krate_name,
                &ureq::http::Response::from_parts(parts, body),
                false,
                &self.lock,
            )
            .and_then(|ie| {
                if let Some(ie) = ie {
                    ie.deserialize().map(Some)
                } else {
                    Ok(None)
                }
            })
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// A single valid sparse-index entry line (64-hex `cksum` is required by
    /// `IndexKrate::from_slice`).
    const INDEX_LINE: &str = concat!(
        r#"{"name":"somecrate","vers":"1.0.0","deps":[],"#,
        r#""cksum":"0000000000000000000000000000000000000000000000000000000000000000","#,
        r#""features":{},"yanked":false}"#,
        "\n"
    );

    /// Spawns a one-shot HTTP/1.1 server that captures the raw request and
    /// answers with `INDEX_LINE`. Returns the base URL and a receiver for the
    /// captured request text.
    fn serve_once() -> (String, std::sync::mpsc::Receiver<String>) {
        serve(1)
    }

    /// Like [`serve_once`], but answers `connections` requests, one per connection.
    fn serve(connections: usize) -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for _ in 0..connections {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = [0_u8; 8192];
                let n = stream.read(&mut buf).unwrap();
                let request = String::from_utf8_lossy(&buf[..n]).into_owned();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
                    INDEX_LINE.len(),
                    INDEX_LINE
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
                let _ = tx.send(request);
            }
        });
        // tame_index requires the `sparse+` scheme prefix; it is stripped before
        // the actual HTTP request is issued to `http://{addr}/`.
        (format!("sparse+http://{addr}/"), rx)
    }

    /// Extract the value of the (case-insensitive) `Authorization` request header.
    fn authorization(request: &str) -> Option<String> {
        request
            .lines()
            .find(|line| line.to_ascii_lowercase().starts_with("authorization:"))
            .map(|line| line.split_once(':').unwrap().1.trim().to_owned())
    }

    // Regression test for #931: a request to an auth-required registry must
    // carry the configured token verbatim, or the server answers 401.
    #[test]
    fn sends_authorization_header_when_auth_present() {
        let (url, rx) = serve_once();
        let mut cache = IndexCache::new(CertsSource::Webpki);
        let registry = Url::parse(&url).unwrap();

        let krate = cache
            .krate(&registry, "somecrate", Some("Bearer test-token"))
            .unwrap();
        assert!(
            krate.is_some(),
            "expected the crate to be parsed from the 200 response"
        );

        let request = rx.recv().unwrap();
        assert_eq!(
            authorization(&request).as_deref(),
            Some("Bearer test-token"),
            "Authorization header missing/wrong in request:\n{request}"
        );
    }

    #[test]
    fn omits_authorization_header_when_no_auth() {
        let (url, rx) = serve_once();
        let mut cache = IndexCache::new(CertsSource::Webpki);
        let registry = Url::parse(&url).unwrap();

        let _ = cache.krate(&registry, "somecrate", None).unwrap();

        let request = rx.recv().unwrap();
        assert_eq!(
            authorization(&request),
            None,
            "unexpected Authorization header in request:\n{request}"
        );
    }

    // The same index URL looked up without and then with a token must not reuse
    // the first (unauthenticated) client, or the second request gets a 401.
    #[test]
    fn auth_is_part_of_the_cache_key() {
        let (url, rx) = serve(2);
        let mut cache = IndexCache::new(CertsSource::Webpki);
        let registry = Url::parse(&url).unwrap();

        let _ = cache.krate(&registry, "somecrate", None).unwrap();
        let _ = cache
            .krate(&registry, "somecrate", Some("Bearer test-token"))
            .unwrap();

        assert_eq!(
            authorization(&rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap()),
            None
        );
        assert_eq!(
            authorization(&rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap()).as_deref(),
            Some("Bearer test-token")
        );
    }
}
