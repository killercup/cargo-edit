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
    index: std::collections::HashMap<Url, AnyIndexCache>,
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
    pub fn has_krate(&mut self, registry: &Url, name: &str) -> CargoResult<bool> {
        self.index(registry)
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
    ) -> CargoResult<Option<bool>> {
        self.index(registry)
            .with_context(|| format!("failed to look up {name}@{version}"))?
            .has_krate_version(name, version)
    }

    #[inline]
    pub fn update_krate(&mut self, registry: &Url, name: &str) -> CargoResult<()> {
        self.index(registry)
            .with_context(|| format!("failed to look up {name}"))?
            .update_krate(name);
        Ok(())
    }

    pub fn krate(&mut self, registry: &Url, name: &str) -> CargoResult<Option<IndexKrate>> {
        self.index(registry)
            .with_context(|| format!("failed to look up {name}"))?
            .krate(name)
    }

    fn index<'s>(&'s mut self, registry: &Url) -> CargoResult<&'s mut AnyIndexCache> {
        if !self.index.contains_key(registry) {
            let index = AnyIndex::open(registry, self.certs_source)?;
            let index = AnyIndexCache::new(index);
            self.index.insert(registry.clone(), index);
        }
        Ok(self.index.get_mut(registry).unwrap())
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
    fn open(url: &Url, certs_source: CertsSource) -> CargoResult<Self> {
        if url.scheme() == "file" {
            LocalIndex::open(url)
                .map(Self::Local)
                .with_context(|| format!("invalid local registry {url:?}"))
        } else {
            RemoteIndex::open(url, certs_source)
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
}

impl RemoteIndex {
    fn open(url: &Url, certs_source: CertsSource) -> CargoResult<Self> {
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
        })
    }

    fn krate(&mut self, name: &str) -> CargoResult<Option<IndexKrate>> {
        log::trace!("krate {name}");
        let krate_name = name.try_into()?;
        let req = self
            .etag_cache
            .make_remote_request(&self.index, krate_name, &self.lock)?;

        let (parts, mut body) = self.client.run(req.into())?.into_parts();
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
