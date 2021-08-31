use crate::errors::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use url::Url;

const CRATES_IO_INDEX: &str = "https://github.com/rust-lang/crates.io-index";
const CRATES_IO_REGISTRY: &str = "crates-io";

#[derive(Debug, Deserialize)]
struct Source {
    #[serde(rename = "replace-with")]
    replace_with: Option<String>,
    registry: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Registry {
    index: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CargoConfig {
    #[serde(default)]
    registries: HashMap<String, Registry>,
    #[serde(default)]
    source: HashMap<String, Source>,
}

fn cargo_home() -> Result<PathBuf> {
    let default_cargo_home = dirs_next::home_dir()
        .map(|x| x.join(".cargo"))
        .chain_err(|| ErrorKind::ReadHomeDirFailure)?;
    let cargo_home = std::env::var("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or(default_cargo_home);
    Ok(cargo_home)
}

/// Find the URL of a registry
pub fn registry_url(manifest_path: &Path, registry: Option<&str>) -> Result<Url> {
    // TODO support local registry sources, directory sources, git sources: https://doc.rust-lang.org/cargo/reference/source-replacement.html?highlight=replace-with#source-replacement
    fn read_config(registries: &mut HashMap<String, Source>, path: impl AsRef<Path>) -> Result<()> {
        // TODO unit test for source replacement
        let content = std::fs::read(path)?;
        let config =
            toml::from_slice::<CargoConfig>(&content).map_err(|_| ErrorKind::InvalidCargoConfig)?;
        for (key, value) in config.registries {
            registries.entry(key).or_insert(Source {
                registry: value.index,
                replace_with: None,
            });
        }
        for (key, value) in config.source {
            registries.entry(key).or_insert(value);
        }
        Ok(())
    }
    // registry might be replaced with another source
    // it's looks like a singly linked list
    // put relations in this map.
    let mut registries: HashMap<String, Source> = HashMap::new();
    // ref: https://doc.rust-lang.org/cargo/reference/config.html#hierarchical-structure
    for work_dir in manifest_path
        .parent()
        .expect("there must be a parent directory")
        .ancestors()
    {
        let work_cargo_dir = work_dir.join(".cargo");
        let config_path = work_cargo_dir.join("config");
        if config_path.is_file() {
            read_config(&mut registries, config_path)?;
        } else {
            let config_path = work_cargo_dir.join("config.toml");
            if config_path.is_file() {
                read_config(&mut registries, config_path)?;
            }
        }
    }

    let default_cargo_home = cargo_home()?;
    let default_config_path = default_cargo_home.join("config");
    if default_config_path.is_file() {
        read_config(&mut registries, default_config_path)?;
    } else {
        let default_config_path = default_cargo_home.join("config.toml");
        if default_config_path.is_file() {
            read_config(&mut registries, default_config_path)?;
        }
    }

    // find head of the relevant linked list
    let mut source = match registry {
        Some(CRATES_IO_INDEX) | None => {
            registries
                .remove(CRATES_IO_REGISTRY)
                .unwrap_or_else(|| Source {
                    replace_with: None,
                    registry: Some(CRATES_IO_INDEX.to_string()),
                })
        }
        Some(r) => registries
            .remove(r)
            .chain_err(|| ErrorKind::NoSuchRegistryFound(r.to_string()))?,
    };

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
        DefaultBranch,
    }
}
