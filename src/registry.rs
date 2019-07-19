use self::code_from_cargo::Kind;
use crate::errors::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::env::current_dir;
use std::path::{Path, PathBuf};
use url::Url;

const CRATES_IO_INDEX: &str = "https://github.com/rust-lang/crates.io-index";
const CRATES_IO_REGISTRY: &str = "crates-io";

pub fn registry_path() -> Result<PathBuf> {
    Ok(cargo_home()?
        .join("registry")
        .join("index")
        .join(short_name(&registry_url()?)))
}

#[derive(Debug, Deserialize)]
struct Source {
    #[serde(rename = "replace-with")]
    replace_with: Option<String>,
    registry: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CargoConfig {
    #[serde(default)]
    sources: HashMap<String, Source>,
}

fn cargo_home() -> Result<PathBuf> {
    let default_cargo_home = dirs::home_dir()
        .map(|x| x.join(".cargo"))
        .chain_err(|| ErrorKind::ReadHomeDirFailure)?;
    let cargo_home = std::env::var("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or(default_cargo_home);
    Ok(cargo_home)
}

pub fn registry_url() -> Result<Url> {
    // TODO support local registry sources, directory sources, git sources: https://doc.rust-lang.org/cargo/reference/source-replacement.html?highlight=replace-with#source-replacement
    fn read_config(registries: &mut HashMap<String, Source>, path: impl AsRef<Path>) -> Result<()> {
        let content = std::fs::read(path)?;
        let config =
            toml::from_slice::<CargoConfig>(&content).map_err(|_| ErrorKind::InvalidCargoConfig)?;
        for (key, value) in config.sources {
            registries.entry(key).or_insert(value);
        }
        Ok(())
    }
    // registry might be replaced with another source
    // it's looks like a singly linked list
    // put relations in this map.
    let mut registries: HashMap<String, Source> = HashMap::new();

    // ref: https://doc.rust-lang.org/cargo/reference/config.html#hierarchical-structure
    for work_dir in current_dir()?.ancestors() {
        let config_path = work_dir.join(".cargo").join("config");
        if config_path.is_file() {
            read_config(&mut registries, config_path)?;
        }
    }

    let default_config_path = cargo_home()?.join("config");
    if default_config_path.is_file() {
        read_config(&mut registries, default_config_path)?;
    }

    // find head of the linked list
    let mut source = registries
        .remove(CRATES_IO_REGISTRY)
        .unwrap_or_else(|| Source {
            replace_with: None,
            registry: Some(CRATES_IO_INDEX.to_string()),
        });

    // search this linked list and find the tail
    while let Some(replace_with) = &source.replace_with {
        source = registries
            .remove(replace_with)
            .chain_err(|| ErrorKind::NoSuchSourceFound(replace_with.to_string()))?;
    }

    let registry_url = source
        .registry
        .and_then(|x| Url::parse(&x).ok())
        .chain_err(|| ErrorKind::InvalidCargoConfig)?;

    Ok(registry_url)
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
