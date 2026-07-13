use super::errors::{CargoResult, Context};
use std::collections::HashMap;
use std::path::Path;
use url::Url;

const CRATES_IO_INDEX: &str = tame_index::index::sparse::CRATES_IO_HTTP_INDEX;
const CRATES_IO_REGISTRY: &str = "crates-io";

/// Find the URL of a registry
pub fn registry_url(manifest_path: &Path, registry: Option<&str>) -> CargoResult<Url> {
    // TODO support local registry sources, directory sources, git sources: https://doc.rust-lang.org/cargo/reference/source-replacement.html?highlight=replace-with#source-replacement
    fn read_config(
        registries: &mut HashMap<String, Source>,
        path: impl AsRef<Path>,
    ) -> CargoResult<()> {
        let path = path.as_ref();
        // TODO unit test for source replacement
        let content = std::fs::read_to_string(path)?;
        let config = toml::from_str::<CargoConfig>(&content)
            .with_context(|| anyhow::format_err!("invalid cargo config at {}", path.display()))?;
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

    let default_cargo_home = home::cargo_home()?;
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
                .get_or_insert_with(|| CRATES_IO_INDEX.to_owned());
            source
        }
        Some(r) => registries
            .remove(r)
            .with_context(|| anyhow::format_err!("The registry '{r}' could not be found"))?,
    };

    // search this linked list and find the tail
    while let Some(replace_with) = &source.replace_with {
        let is_crates_io = replace_with == CRATES_IO_INDEX;
        source = registries.remove(replace_with).with_context(|| {
            anyhow::format_err!("The source '{replace_with}' could not be found")
        })?;
        if is_crates_io {
            source
                .registry
                .get_or_insert_with(|| CRATES_IO_INDEX.to_owned());
        }
    }

    let registry_url = source
        .registry
        .ok_or_else(|| anyhow::format_err!("missing `registry`"))?;
    let registry_url = Url::parse(&registry_url)
        .with_context(|| anyhow::format_err!("invalid `registry` field"))?;

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
    #[serde(default)]
    token: Option<String>,
}

/// Find the authentication token for a registry, if one is configured.
///
/// Sparse registries with `auth-required = true` reject unauthenticated index
/// requests with `401 Unauthorized`. cargo stores the token out-of-band, so we
/// resolve it here and let the caller attach it to the request.
///
/// Precedence mirrors cargo: the `CARGO_REGISTRIES_<NAME>_TOKEN` environment
/// variable wins, then `token` under `[registries.<name>]` in the `credentials`
/// / `config` files, walking from the manifest directory up to `$CARGO_HOME`.
pub fn registry_token(manifest_path: &Path, registry: Option<&str>) -> CargoResult<Option<String>> {
    // The crates.io index does not require authentication, and its publish
    // token must not be leaked to the (CDN-fronted) sparse index, so only named
    // alternative registries are considered here.
    let Some(name) = registry else {
        return Ok(None);
    };

    let env_key = format!(
        "CARGO_REGISTRIES_{}_TOKEN",
        name.to_uppercase().replace('-', "_")
    );
    if let Some(token) = std::env::var_os(&env_key) {
        let token = token
            .into_string()
            .map_err(|_err| anyhow::format_err!("`{env_key}` was not valid unicode"))?;
        return Ok(Some(token));
    }

    fn read_token(path: impl AsRef<Path>, name: &str) -> CargoResult<Option<String>> {
        let path = path.as_ref();
        if !path.is_file() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        let config = toml::from_str::<CargoConfig>(&content)
            .with_context(|| anyhow::format_err!("invalid cargo config at {}", path.display()))?;
        Ok(config
            .registries
            .get(name)
            .and_then(|registry| registry.token.clone()))
    }

    // Walk the config hierarchy nearest-first. Within a directory, `credentials`
    // takes precedence over `config`, matching cargo.
    const CONFIG_FILES: [&str; 4] = ["credentials.toml", "credentials", "config.toml", "config"];
    for work_dir in manifest_path
        .parent()
        .expect("there must be a parent directory")
        .ancestors()
    {
        let cargo_dir = work_dir.join(".cargo");
        for file in CONFIG_FILES {
            if let Some(token) = read_token(cargo_dir.join(file), name)? {
                return Ok(Some(token));
            }
        }
    }

    let cargo_home = home::cargo_home()?;
    for file in CONFIG_FILES {
        if let Some(token) = read_token(cargo_home.join(file), name)? {
            return Ok(Some(token));
        }
    }

    Ok(None)
}

mod code_from_cargo {
    #![allow(dead_code)]

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub(super) enum Kind {
        Git(GitReference),
        Path,
        Registry,
        LocalRegistry,
        Directory,
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub(super) enum GitReference {
        Tag(String),
        Branch(String),
        Rev(String),
        DefaultBranch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Registry name unlikely to have a real token in the ambient `$CARGO_HOME`,
    /// so the fallthrough to `$CARGO_HOME` never masks the file under test.
    const REGISTRY: &str = "cargo-edit-token-test-registry";

    /// A throwaway `<tmp>/.cargo/` tree, cleaned up on drop.
    struct TempTree {
        root: std::path::PathBuf,
    }

    impl TempTree {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "cargo-edit-registry-test-{}-{tag}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join(".cargo")).unwrap();
            Self { root }
        }

        fn write(&self, rel: &str, contents: &str) {
            std::fs::write(self.root.join(rel), contents).unwrap();
        }

        fn manifest(&self) -> std::path::PathBuf {
            self.root.join("Cargo.toml")
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn token_from_credentials_toml() {
        let tree = TempTree::new("credentials");
        tree.write(
            ".cargo/credentials.toml",
            &format!("[registries.{REGISTRY}]\ntoken = \"cred-token\"\n"),
        );
        let token = registry_token(&tree.manifest(), Some(REGISTRY)).unwrap();
        assert_eq!(token.as_deref(), Some("cred-token"));
    }

    #[test]
    fn credentials_wins_over_config() {
        let tree = TempTree::new("precedence");
        // With only a config file, its token is used.
        tree.write(
            ".cargo/config.toml",
            &format!(
                "[registries.{REGISTRY}]\nindex = \"sparse+https://example.com/\"\ntoken = \"config-token\"\n"
            ),
        );
        assert_eq!(
            registry_token(&tree.manifest(), Some(REGISTRY))
                .unwrap()
                .as_deref(),
            Some("config-token")
        );
        // credentials.toml in the same directory takes precedence.
        tree.write(
            ".cargo/credentials.toml",
            &format!("[registries.{REGISTRY}]\ntoken = \"cred-token\"\n"),
        );
        assert_eq!(
            registry_token(&tree.manifest(), Some(REGISTRY))
                .unwrap()
                .as_deref(),
            Some("cred-token")
        );
    }

    #[test]
    fn unknown_registry_is_none() {
        let tree = TempTree::new("unknown");
        tree.write(
            ".cargo/credentials.toml",
            &format!("[registries.{REGISTRY}]\ntoken = \"cred-token\"\n"),
        );
        assert_eq!(
            registry_token(&tree.manifest(), Some("cargo-edit-absent-registry")).unwrap(),
            None
        );
    }

    #[test]
    fn default_registry_is_none() {
        // The crates.io index never requires a token; don't leak the publish token to it.
        let tree = TempTree::new("default");
        assert_eq!(registry_token(&tree.manifest(), None).unwrap(), None);
    }
}
