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
    // ref: https://github.com/rust-lang/cargo/blob/4c1fa54d10f58d69ac9ff55be68e1b1c25ecb816/src/cargo/sources/registry/mod.rs#L386-L390
    #![allow(deprecated)]
    use std::hash::{Hash, Hasher, SipHasher};

    let mut hasher = SipHasher::new_with_keys(0, 0);
    Kind::Registry.hash(&mut hasher);
    registry.as_str().hash(&mut hasher);
    let hash = hex::encode(hasher.finish().to_le_bytes());

    let ident = registry.host_str().unwrap_or("").to_string();

    format!("{}-{}", ident, hash)
}

#[test]
fn test_short_name() {
    fn test_helper(url: &str, name: &str) {
        let url = Url::parse(url).unwrap();
        assert_eq!(short_name(&url), name);
    }
    test_helper(
        "https://github.com/rust-lang/crates.io-index",
        "github.com-1ecc6299db9ec823",
    );
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
