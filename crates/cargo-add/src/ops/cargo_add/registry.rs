use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use cargo::CargoResult;
use url::Url;

use super::errors::*;

const CRATES_IO_INDEX: &str = "https://github.com/rust-lang/crates.io-index";
const CRATES_IO_REGISTRY: &str = "crates-io";

/// Find the URL of a registry
pub fn registry_url(work_dir: &Path, registry: Option<&str>) -> CargoResult<Url> {
    // TODO support local registry sources, directory sources, git sources: https://doc.rust-lang.org/cargo/reference/source-replacement.html?highlight=replace-with#source-replacement
    fn read_config(
        registries: &mut HashMap<String, Source>,
        path: impl AsRef<Path>,
    ) -> CargoResult<()> {
        // TODO unit test for source replacement
        let content = std::fs::read(path)?;
        let config = toml_edit::easy::from_slice::<CargoConfig>(&content)
            .map_err(|_| invalid_cargo_config())?;
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
    for work_dir in work_dir.ancestors() {
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
            let mut source = registries.remove(CRATES_IO_REGISTRY).unwrap_or_default();
            source
                .registry
                .get_or_insert_with(|| CRATES_IO_INDEX.to_string());
            source
        }
        Some(r) => registries
            .remove(r)
            .with_context(|| anyhow::format_err!("The registry '{}' could not be found", r))?,
    };

    // search this linked list and find the tail
    while let Some(replace_with) = &source.replace_with {
        let is_crates_io = replace_with == CRATES_IO_INDEX;
        source = registries.remove(replace_with).with_context(|| {
            anyhow::format_err!("The source '{}' could not be found", replace_with)
        })?;
        if is_crates_io {
            source
                .registry
                .get_or_insert_with(|| CRATES_IO_INDEX.to_string());
        }
    }

    let registry_url = source
        .registry
        .and_then(|x| Url::parse(&x).ok())
        .with_context(invalid_cargo_config)?;

    Ok(registry_url)
}

#[derive(Debug, Deserialize)]
struct CargoConfig {
    #[serde(default)]
    registries: HashMap<String, Registry>,
    #[serde(default)]
    source: HashMap<String, Source>,
}

#[derive(Default, Debug, Deserialize)]
struct Source {
    #[serde(rename = "replace-with")]
    replace_with: Option<String>,
    registry: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Registry {
    index: Option<String>,
}

fn cargo_home() -> CargoResult<PathBuf> {
    let default_cargo_home = dirs_next::home_dir()
        .map(|x| x.join(".cargo"))
        .with_context(|| anyhow::format_err!("Failed to read home directory"))?;
    let cargo_home = std::env::var("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or(default_cargo_home);
    Ok(cargo_home)
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