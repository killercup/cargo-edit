use self::code_from_cargo::Kind;
use crate::errors::*;
use std::path::PathBuf;
use url::Url;

const CRATES_IO_INDEX: &str = "https://github.com/rust-lang/crates.io-index";

pub fn registry_path() -> Result<PathBuf> {
    Ok(index_path()?.join(short_name(&registry_url()?)))
}

fn registry_url() -> Result<Url> {
    // TODO parse cargo config
    Ok(Url::parse(CRATES_IO_INDEX).map_err(|_| "registry url is wrong")?)
}

fn index_path() -> Result<PathBuf> {
    // TODO parse cargo config
    Ok(dirs::home_dir()
        .chain_err(|| ErrorKind::ReadHomeDirFailure)?
        .join(".cargo")
        .join("registry")
        .join("index"))
}

fn short_name(registry: &Url) -> String {
    #![allow(deprecated)]
    use std::hash::{Hash, Hasher, SipHasher};

    let mut hasher = SipHasher::new_with_keys(0, 0);
    Kind::Registry.hash(&mut hasher);
    registry.as_str().hash(&mut hasher);
    let hash = hex::encode(hasher.finish().to_le_bytes());

    let ident = registry.host_str().unwrap_or("").to_string();

    format!("{}-{}", ident, hash)
}

mod code_from_cargo {
    #![allow(dead_code)]

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum Kind {
        Git(GitReference),
        Path,
        Registry,
        LocalRegistry,
        Directory,
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum GitReference {
        Tag(String),
        Branch(String),
        Rev(String),
    }
}
