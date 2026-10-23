use super::errors::{CargoResult, Context};
use std::collections::HashMap;
use std::path::Path;
use url::Url;

const CRATES_IO_INDEX: &str = tame_index::index::sparse::CRATES_IO_HTTP_INDEX;
const CRATES_IO_REGISTRY: &str = "crates-io";

// Read `[registries]` and `[source]` tables from a cargo config file into `registries`.
fn read_config(
    registries: &mut HashMap<String, Source>,
    path: impl AsRef<Path>,
) -> CargoResult<()> {
    let path = path.as_ref();
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

fn load_registries(manifest_path: &Path) -> CargoResult<HashMap<String, Source>> {
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

    Ok(registries)
}

// Resolve `registry` through any `replace-with` source replacements, defaults to crates-io.
fn resolve_registry(
    registries: &mut HashMap<String, Source>,
    registry: Option<&str>,
) -> CargoResult<(Vec<String>, Source)> {
    // find head of the relevant linked list
    let (head, mut source) = match registry {
        Some(CRATES_IO_INDEX | CRATES_IO_REGISTRY) | None => {
            let mut source = registries.remove(CRATES_IO_REGISTRY).unwrap_or_default();
            source
                .registry
                .get_or_insert_with(|| CRATES_IO_INDEX.to_owned());
            (CRATES_IO_REGISTRY.to_owned(), source)
        }
        Some(r) => {
            let source = registries
                .remove(r)
                .with_context(|| anyhow::format_err!("The registry '{r}' could not be found"))?;
            (r.to_owned(), source)
        }
    };
    let mut names = vec![head];

    // search this linked list and find the tail
    while let Some(replace_with) = source.replace_with.take() {
        // detect cycle: `a -> b -> a`
        if names.contains(&replace_with) {
            let chain = names
                .iter()
                .chain(std::iter::once(&replace_with))
                .cloned()
                .collect::<Vec<_>>()
                .join(" -> ");
            anyhow::bail!("cycle detected in source replacement: {chain}");
        }

        // crates-io is built in, so it need not be declared in any config
        source = if replace_with == CRATES_IO_REGISTRY {
            let mut source = registries.remove(&replace_with).unwrap_or_default();
            source
                .registry
                .get_or_insert_with(|| CRATES_IO_INDEX.to_owned());
            source
        } else {
            registries.remove(&replace_with).with_context(|| {
                anyhow::format_err!("The source '{replace_with}' could not be found")
            })?
        };
        names.push(replace_with);
    }

    Ok((names, source))
}

/// Find the URL of a registry
pub fn registry_url(manifest_path: &Path, registry: Option<&str>) -> CargoResult<Url> {
    // TODO support local registry sources, directory sources, git sources: https://doc.rust-lang.org/cargo/reference/source-replacement.html?highlight=replace-with#source-replacement
    let mut registries = load_registries(manifest_path)?;
    let (_names, source) = resolve_registry(&mut registries, registry)?;

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
/// The registry name is resolved through `replace-with` source replacement
/// first allowing crates.io to be served by a private registry.
///
/// Precedence mirrors cargo: the `CARGO_REGISTRIES_<NAME>_TOKEN` environment
/// variable wins, then `token` under `[registries.<name>]` in the `credentials`
/// / `config` files, walking from the manifest directory up to `$CARGO_HOME`.
pub fn registry_token(manifest_path: &Path, registry: Option<&str>) -> CargoResult<Option<String>> {
    registry_token_with_env(manifest_path, registry, |key| std::env::var_os(key))
}

// `registry_token` with the environment lookup injected, so tests need not mutate process state.
fn registry_token_with_env(
    manifest_path: &Path,
    registry: Option<&str>,
    env_var: impl Fn(&str) -> Option<std::ffi::OsString>,
) -> CargoResult<Option<String>> {
    // resolve which registry ultimately serves the request:
    // `source.crates-io.replace-with = "my-registry"` means the token should be pulled
    // from "my-registry", not "crates-io".
    let names = {
        let mut registries = load_registries(manifest_path)?;
        let (names, _source) = resolve_registry(&mut registries, registry)?;
        names
    };
    let Some(name) = names.last() else {
        return Ok(None);
    };

    // crates.io index does not require auth
    if name == CRATES_IO_REGISTRY {
        return Ok(None);
    }

    let env_key = format!(
        "CARGO_REGISTRIES_{}_TOKEN",
        name.to_uppercase().replace('-', "_")
    );
    if let Some(token) = env_var(&env_key) {
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

    /// Registry name extremely unlikely to have a real token in the ambient `$CARGO_HOME`,
    /// so the fallthrough to `$CARGO_HOME` never masks the file under test.
    const REGISTRY: &str = "cargo-edit-token-test-registry-3b0ff8d5-2d5a-4c44-9b11-1b0f5b9e8f3a";
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
            let path = self.root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
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
            ".cargo/config.toml",
            &format!("[registries.{REGISTRY}]\nindex = \"sparse+https://example.com/\"\n"),
        );
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
    fn source_replacement_is_followed() {
        let tree = TempTree::new("replace");
        tree.write(
            ".cargo/config.toml",
            &format!(
                "[source.crates-io]\nreplace-with = \"{REGISTRY}\"\n\n[registries.{REGISTRY}]\nindex = \"sparse+https://example.com/index/\"\n"
            ),
        );
        tree.write(
            ".cargo/credentials.toml",
            &format!(
                "[registry]\ntoken = \"crates-io-publish-token\"\n\n[registries.{REGISTRY}]\ntoken = \"replaced-token\"\n"
            ),
        );

        let url = registry_url(&tree.manifest(), None).unwrap();
        assert_eq!(url.as_str(), "sparse+https://example.com/index/");

        let token = registry_token(&tree.manifest(), None).unwrap();
        assert_eq!(token.as_deref(), Some("replaced-token"));
    }

    #[test]
    fn replacement_with_crates_io_uses_default_index() {
        let tree = TempTree::new("to-crates-io");
        tree.write(
            ".cargo/config.toml",
            "[source.mirror]\nreplace-with = \"crates-io\"\n",
        );
        tree.write(
            ".cargo/credentials.toml",
            "[registry]\ntoken = \"crates-io-publish-token\"\n",
        );

        let url = registry_url(&tree.manifest(), Some("mirror")).unwrap();
        assert_eq!(url.as_str(), CRATES_IO_INDEX);

        // The crates.io publish token must not be sent to the index.
        let token = registry_token(&tree.manifest(), Some("mirror")).unwrap();
        assert_eq!(token, None);
    }

    #[test]
    fn misconfigured_source_replacement_is_an_error() {
        let cycle = TempTree::new("cycle");
        cycle.write(
            ".cargo/config.toml",
            "[source.crates-io]\nreplace-with = \"a\"\n\n[source.a]\nreplace-with = \"b\"\n\n[source.b]\nreplace-with = \"a\"\n",
        );
        let err = registry_url(&cycle.manifest(), None).unwrap_err();
        assert!(err.to_string().contains("cycle"), "got: {err}");

        let missing = TempTree::new("missing");
        missing.write(
            ".cargo/config.toml",
            "[source.crates-io]\nreplace-with = \"a\"\n",
        );
        let token_err = registry_token(&missing.manifest(), None).unwrap_err();
        let url_err = registry_url(&missing.manifest(), None).unwrap_err();
        assert_eq!(token_err.to_string(), url_err.to_string());
        assert!(token_err.to_string().contains("'a'"), "got: {token_err}");
    }

    #[test]
    fn unknown_registry_is_an_error() {
        let tree = TempTree::new("unknown");
        tree.write(
            ".cargo/credentials.toml",
            &format!("[registries.{REGISTRY}]\ntoken = \"cred-token\"\n"),
        );
        let err = registry_token(&tree.manifest(), Some("cargo-edit-absent-registry")).unwrap_err();
        assert!(
            err.to_string().contains("cargo-edit-absent-registry"),
            "expected a missing-registry error, got: {err}"
        );
    }

    #[test]
    fn unauthenticated_registry_is_none() {
        let crates_io = TempTree::new("default");
        assert_eq!(registry_token(&crates_io.manifest(), None).unwrap(), None);

        let tree = TempTree::new("no-token");
        tree.write(
            ".cargo/config.toml",
            &format!("[registries.{REGISTRY}]\nindex = \"sparse+https://example.com/\"\n"),
        );
        assert_eq!(
            registry_token(&tree.manifest(), Some(REGISTRY)).unwrap(),
            None
        );
    }

    #[test]
    fn explicit_crates_io_is_builtin() {
        let tree = TempTree::new("explicit-crates-io");
        tree.write(
            ".cargo/credentials.toml",
            "[registry]\ntoken = \"crates-io-publish-token\"\n",
        );

        let url = registry_url(&tree.manifest(), Some(CRATES_IO_REGISTRY)).unwrap();
        assert_eq!(url.as_str(), CRATES_IO_INDEX);

        let token = registry_token(&tree.manifest(), Some(CRATES_IO_REGISTRY)).unwrap();
        assert_eq!(token, None);
    }

    #[test]
    fn env_token_wins_over_files() {
        let tree = TempTree::new("env");
        tree.write(
            ".cargo/config.toml",
            "[registries.my-registry]\nindex = \"sparse+https://example.com/\"\ntoken = \"config-token\"\n",
        );
        tree.write(
            ".cargo/credentials.toml",
            "[registries.my-registry]\ntoken = \"cred-token\"\n",
        );

        // Dashes in the registry name become underscores in the variable name.
        let env =
            |key: &str| (key == "CARGO_REGISTRIES_MY_REGISTRY_TOKEN").then(|| "env-token".into());
        let token = registry_token_with_env(&tree.manifest(), Some("my-registry"), env).unwrap();
        assert_eq!(token.as_deref(), Some("env-token"));
    }

    #[test]
    fn env_token_follows_source_replacement() {
        let tree = TempTree::new("env-replace");
        tree.write(
            ".cargo/config.toml",
            "[source.crates-io]\nreplace-with = \"my-registry\"\n\n[registries.my-registry]\nindex = \"sparse+https://example.com/\"\n",
        );

        // The token is keyed by the registry that serves the index, never by crates-io.
        let env = |key: &str| match key {
            "CARGO_REGISTRIES_MY_REGISTRY_TOKEN" => Some("replaced-token".into()),
            "CARGO_REGISTRIES_CRATES_IO_TOKEN" => Some("crates-io-token".into()),
            _ => None,
        };
        let token = registry_token_with_env(&tree.manifest(), None, env).unwrap();
        assert_eq!(token.as_deref(), Some("replaced-token"));
    }

    #[test]
    fn multi_hop_replacement_uses_tail() {
        let tree = TempTree::new("multi-hop");
        tree.write(
            ".cargo/config.toml",
            &format!(
                "[source.crates-io]\nreplace-with = \"middle\"\n\n[source.middle]\nreplace-with = \"{REGISTRY}\"\n\n[registries.{REGISTRY}]\nindex = \"sparse+https://example.com/tail/\"\n"
            ),
        );
        tree.write(
            ".cargo/credentials.toml",
            &format!(
                "[registries.middle]\ntoken = \"middle-token\"\n\n[registries.{REGISTRY}]\ntoken = \"tail-token\"\n"
            ),
        );

        let url = registry_url(&tree.manifest(), None).unwrap();
        assert_eq!(url.as_str(), "sparse+https://example.com/tail/");

        let token = registry_token(&tree.manifest(), None).unwrap();
        assert_eq!(token.as_deref(), Some("tail-token"));
    }

    #[test]
    fn nearest_config_wins() {
        let tree = TempTree::new("nearest");
        tree.write(
            ".cargo/config.toml",
            &format!(
                "[registries.{REGISTRY}]\nindex = \"sparse+https://example.com/parent/\"\ntoken = \"parent-token\"\n"
            ),
        );
        tree.write(
            "member/.cargo/config.toml",
            &format!(
                "[registries.{REGISTRY}]\nindex = \"sparse+https://example.com/member/\"\ntoken = \"member-token\"\n"
            ),
        );
        let manifest = tree.root.join("member/Cargo.toml");

        let url = registry_url(&manifest, Some(REGISTRY)).unwrap();
        assert_eq!(url.as_str(), "sparse+https://example.com/member/");

        let token = registry_token(&manifest, Some(REGISTRY)).unwrap();
        assert_eq!(token.as_deref(), Some("member-token"));
    }
}
