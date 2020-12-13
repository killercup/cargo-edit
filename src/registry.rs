use self::code_from_cargo::Kind;
use crate::errors::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use url::Url;

const CRATES_IO_INDEX: &str = "https://github.com/rust-lang/crates.io-index";
const CRATES_IO_REGISTRY: &str = "crates-io";

/// Returns the filesystem path containing the cache for the specified registry (or `crates.io` if `None`).
/// The provided `manifest_path` is used to resolve custom registries provided in `registry_name`
pub fn registry_path(request: &RegistryReq) -> Result<PathBuf> {
    registry_path_from_url(&registry_url(request)?)
}

pub fn registry_path_from_url(registry: &Url) -> Result<PathBuf> {
    Ok(cargo_home()?
        .join("registry")
        .join("index")
        .join(short_name(registry)))
}

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

/// A resolved index to a cargo registry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryIndex(Url);
impl RegistryIndex {
    /// Returns the internal short-name of the registry
    pub fn short_name(&self) -> String {
        short_name(&self.0)
    }
}

/// Allows definition of different cargo registries, and their source.
///
// Note: registries can be defined in cargo project tomls, global user cargo configs, by URL, or by environment variables
// ref: https://doc.rust-lang.org/cargo/reference/registries.html
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryReq<'a> {
    /// Used to specify a registry defined within a project, or a project's parent directories (as defined within a .cargo/config).
    Project {
        /// A specific cargo registry, by name. Defaults to `crates.io`
        registry_name: Option<&'a str>,
        /// A path to start searching for cargo configs from
        manifest_dir: &'a Path,
    },
    /// Uses only the user's global cargo config, if it exists. `None` defaults to `crates.io`
    User {
        /// A specific cargo registry, by name. Defaults to `crates.io`
        registry_name: Option<&'a str>,
    },
    /// An unnamed specific cargo registry, by URL
    Custom {
        /// The URL to the registry's index
        index: RegistryIndex,
    },
}
impl<'a> Default for RegistryReq<'a> {
    /// Specifies the user's default crate repository - typically `crates.io`
    fn default() -> RegistryReq<'a> {
        // Use user as default to avoid possibly surprising scenario of project overriding registries
        RegistryReq::User {
            registry_name: None,
        }
    }
}
impl<'a> RegistryReq<'a> {
    /// A registry specified by name, resolved using a user's project and global configuration. If no registry is provided, then `crates.io` is used.
    pub fn project(registry_name: Option<&'a str>, manifest_dir: &'a Path) -> RegistryReq<'a> {
        RegistryReq::Project {
            registry_name,
            manifest_dir,
        }
    }

    /// A registry specified by name, resolved using a user's global configuration. If none is provided, then `crates.io` is used.
    pub fn user(registry_name: Option<&str>) -> RegistryReq<'_> {
        RegistryReq::User { registry_name }
    }

    /// A registry specified by URL. This URL is assumed to be valid.
    pub fn custom(index: Url) -> RegistryReq<'a> {
        RegistryReq::Custom {
            index: RegistryIndex(index),
        }
    }

    /// Resolves the registry's index URL
    pub fn index(&self) -> Result<RegistryIndex> {
        // check environment variable
        // ref: https://doc.rust-lang.org/cargo/reference/registries.html#using-an-alternate-registry
        // Can't find environment variable checking in cargo?

        // check cargo configs for `registries` entries

        // check default cargo

        // fallback to cargo? None?

        registry_url(self).map(RegistryIndex)
    }
}

/// Returns the user's configured, or default, cargo home directory.
fn cargo_home() -> Result<PathBuf> {
    // use $HOME/.cargo if no $CARGO_HOME or $CARGO_HOME has bad UTF8
    match std::env::var_os("CARGO_HOME") {
        Some(cargo_home) => Ok(cargo_home.into()),
        None => {
            let userhome = dirs::home_dir().chain_err(|| ErrorKind::ReadHomeDirFailure)?;
            Ok(userhome.join(".cargo"))
        }
    }
}

/// Searches for a `(config|config.toml)` file inside the provided directory.
fn get_dir_config(dir: PathBuf) -> Option<PathBuf> {
    // prefer `config` over `config.toml` for compatibility (according to note in config reference below)
    // ref: https://doc.rust-lang.org/cargo/reference/config.html
    // note: should this method return an `impl io::Read` to prevent race conditions, if the config is removed?

    let mut target = dir.join("config");
    if target.is_file() {
        return Some(target);
    }

    target.set_extension("toml");
    if target.is_file() {
        return Some(target);
    }

    None
}

/// Searches for `.cargo/(config|config.toml)` files at or above the base directory specified.  
///
/// The closest to the base directory (ie: the configs with the preferred values) are at the beginning of the returned list.  
///
/// Note this may also yield the user's global config file (the one returned from [`user_cargo_config`]) if the project directory within the user's home directory (Desktop, etc)
///
/// See [Cargo Reference/Configuration/Hierarchical structure](https://doc.rust-lang.org/cargo/reference/config.html#hierarchical-structure) for resolution details.
fn project_cargo_configs(base: &Path) -> Result<Vec<PathBuf>> {
    // TODO: (?) if found, return the opened file.
    // This prevents a race condition involving the file disappearing before being able to read it.

    // return type: Result<Option<(PathBuf, Result<impl std::io::Read>)>> ??
    // Outer result fails if there was an error traversing the file tree
    // Option is None if none were found
    // Inner result fails if there was an error opening the file (permissions, ...)

    // go up the filesystem looking for cargo configs
    // ref: https://doc.rust-lang.org/cargo/reference/config.html#hierarchical-structure
    let cfgs = base
        .canonicalize()
        .chain_err(|| ErrorKind::DirectoryResolutionFailure(base.to_owned()))?
        .ancestors()
        .filter_map(|path| get_dir_config(path.join(".cargo")))
        .collect();

    Ok(cfgs)
}

/// Retrieves the user's global cargo config
fn user_cargo_config() -> Result<Option<PathBuf>> {
    cargo_home().map(get_dir_config)
}

/// Find the URL of a registry. Defaults to crates.io if no registry is provided.
///
/// Uses a project's/user's cargo config files to resolve the registry to a URL.
pub fn registry_url(request: &RegistryReq) -> Result<Url> {
    // TODO support local registry sources, directory sources, git sources: https://doc.rust-lang.org/cargo/reference/source-replacement.html?highlight=replace-with#source-replacement

    /// Takes the registries/sources from the specified cargo config toml file, and adds them to the HashMap
    /// If there is a conflict, then the existing value is favored.
    fn merge_cfg(registries: &mut HashMap<String, Source>, path: impl AsRef<Path>) -> Result<()> {
        // TODO unit test for source replacement
        let content = std::fs::read_to_string(path)?;
        let config =
            toml::from_str::<CargoConfig>(&content).map_err(|_| ErrorKind::InvalidCargoConfig)?;

        for (key, value) in config.registries {
            // favor previous values
            registries.entry(key).or_insert(Source {
                registry: value.index,
                replace_with: None,
            });
        }
        for (key, value) in config.source {
            // favor previous values
            registries.entry(key).or_insert(value);
        }
        Ok(())
    }

    // TODO implement support for registries specified via environment variables
    // ref: https://doc.rust-lang.org/cargo/reference/registries.html#using-an-alternate-registry

    fn merge_user_config(registries: &mut HashMap<String, Source>) -> Result<()> {
        // look at the user's global cargo config
        if let Some(user_config) = user_cargo_config()? {
            merge_cfg(registries, user_config)?;
        }

        Ok(())
    }

    fn lookup_registry(
        registries: &mut HashMap<String, Source>,
        registry_name: Option<&str>,
    ) -> Result<Url> {
        //TODO: change impl to do simple lookups instead of removes? (then we don't have to mutate the lookup table -> lookups can be tested easier)

        // find head of the relevant linked list
        let mut source =
            match registry_name {
                Some(CRATES_IO_INDEX) | None => registries
                    .remove(CRATES_IO_REGISTRY)
                    .unwrap_or_else(|| Source {
                        replace_with: None,
                        registry: Some(CRATES_IO_INDEX.to_string()),
                    }),
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

    // registry might be replaced with another source
    // it's looks like a singly linked list
    // put relations in this map.
    let mut registries: HashMap<String, Source> = HashMap::new();

    let registry_index: Url = match request {
        RegistryReq::Custom { index } => {
            // We were already given a bare index URL - return that
            index.0.clone()
        }
        RegistryReq::User { registry_name } => {
            // Lookup a URL based on global user config
            merge_user_config(&mut registries)?;

            lookup_registry(&mut registries, *registry_name)?
        }
        RegistryReq::Project {
            registry_name,
            manifest_dir,
        } => {
            // only use a project's registry configurations, if they exist
            project_cargo_configs(manifest_dir)?
                .iter()
                .try_for_each(|config_path| merge_cfg(&mut registries, config_path))?;

            merge_user_config(&mut registries)?;

            lookup_registry(&mut registries, *registry_name)?
        }
    };

    Ok(registry_index)
}

fn short_name(registry: &Url) -> String {
    // ref: https://github.com/rust-lang/cargo/blob/4c1fa54d10f58d69ac9ff55be68e1b1c25ecb816/src/cargo/sources/registry/mod.rs#L386-L390
    #![allow(deprecated)]
    use std::hash::{Hash, Hasher, SipHasher};

    let mut hasher = SipHasher::new();
    Kind::Registry.hash(&mut hasher);
    registry.as_str().hash(&mut hasher);
    let hash = hex::encode(hasher.finish().to_le_bytes());

    let ident = registry.host_str().unwrap_or("").to_string();

    format!("{}-{}", ident, hash)
}

#[cfg_attr(target_pointer_width = "64", test)]
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
        DefaultBranch,
    }
}
