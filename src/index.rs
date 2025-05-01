use tame_index::krate::IndexKrate;
use tame_index::utils::flock::FileLock;

use url::Url;

use super::errors::*;

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
    index: tame_index::index::LocalRegistry,
    root: tame_index::PathBuf,
}

impl LocalIndex {
    fn open(url: &Url) -> CargoResult<Self> {
        let path = url
            .to_file_path()
            .map_err(|_err| anyhow::format_err!("invalid file path {url:?}"))?;
        let path = tame_index::PathBuf::from_path_buf(path)
            .map_err(|_err| anyhow::format_err!("invalid file path {url:?}"))?;
        let index = tame_index::index::LocalRegistry::open(path.clone(), false)?;
        Ok(Self { index, root: path })
    }

    fn krate(&mut self, name: &str) -> CargoResult<Option<IndexKrate>> {
        let name = tame_index::KrateName::cargo(name)?;
        // HACK: for some reason, `tame_index` puts `index` in the middle
        let entry_path = self.index.krate_path(name);
        let rel_path = entry_path
            .strip_prefix(&self.root)
            .map_err(|_err| anyhow::format_err!("invalid index path {entry_path:?}"))?;
        let rel_path = rel_path
            .strip_prefix("index")
            .map_err(|_err| anyhow::format_err!("invalid index path {entry_path:?}"))?;
        let entry_path = self.root.join(rel_path);
        let Ok(entry) = std::fs::read(&entry_path) else {
            return Ok(None);
        };
        let results = IndexKrate::from_slice(&entry)?;
        Ok(Some(results))
    }
}

struct RemoteIndex {
    index: tame_index::SparseIndex,
    client: tame_index::external::reqwest::blocking::Client,
    lock: FileLock,
    etags: Vec<(String, String)>,
}

impl RemoteIndex {
    fn open(url: &Url, certs_source: CertsSource) -> CargoResult<Self> {
        let url = url.to_string();
        let url = tame_index::IndexUrl::NonCratesIo(std::borrow::Cow::Owned(url));
        let index = tame_index::SparseIndex::new(tame_index::IndexLocation::new(url))?;

        let client = {
            let builder = tame_index::external::reqwest::blocking::ClientBuilder::new();

            let builder = match certs_source {
                CertsSource::Webpki => builder.tls_built_in_webpki_certs(true),
                CertsSource::Native => builder.tls_built_in_native_certs(true),
            };

            builder.build()?
        };

        let lock = FileLock::unlocked();

        Ok(Self {
            index,
            client,
            lock,
            etags: Vec::new(),
        })
    }

    fn krate(&mut self, name: &str) -> CargoResult<Option<IndexKrate>> {
        let etag = self
            .etags
            .iter()
            .find_map(|(krate, etag)| (krate == name).then_some(etag.as_str()))
            .unwrap_or("");

        let krate_name = name.try_into()?;
        let req = self
            .index
            .make_remote_request(krate_name, Some(etag), &self.lock)?;
        let (
            tame_index::external::http::request::Parts {
                method,
                uri,
                version,
                headers,
                ..
            },
            _,
        ) = req.into_parts();
        let mut req = self.client.request(method, uri.to_string());
        req = req.version(version);
        req = req.headers(headers);
        let res = self.client.execute(req.build()?)?;

        // Grab the etag if it exists for future requests
        if let Some(etag) = res
            .headers()
            .get(tame_index::external::reqwest::header::ETAG)
        {
            if let Ok(etag) = etag.to_str() {
                if let Some(i) = self.etags.iter().position(|(krate, _)| krate == name) {
                    etag.clone_into(&mut self.etags[i].1);
                } else {
                    self.etags.push((name.to_owned(), etag.to_owned()));
                }
            }
        }

        let mut builder = tame_index::external::http::Response::builder()
            .status(res.status())
            .version(res.version());

        builder
            .headers_mut()
            .unwrap()
            .extend(res.headers().iter().map(|(k, v)| (k.clone(), v.clone())));

        let body = res.bytes()?;
        let response = builder
            .body(body.to_vec())
            .map_err(|e| tame_index::Error::from(tame_index::error::HttpError::from(e)))?;

        self.index
            .parse_remote_response(krate_name, response, false, &self.lock)
            .map_err(Into::into)
    }
}
