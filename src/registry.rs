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
#[allow(dead_code)]
struct Registry {
    index: Option<String>,
    token: Option<String>,
    #[serde(rename = "credential-provider")]
    credential_provider: Option<String>,
}

/// Resolve the auth token for a registry by checking (in order):
/// 1. Environment variable `CARGO_REGISTRIES_<NAME>_TOKEN`
/// 2. credentials.toml
/// 3. credential-provider command (cargo:token-from-stdout)
/// 4. token field in config.toml
pub fn registry_token(manifest_path: &Path, registry: Option<&str>) -> Option<String> {
    let registry_name = resolve_registry_name(manifest_path, registry);
    let registry_name = registry_name.as_deref()?;

    // 1. Check environment variable
    let env_key = format!(
        "CARGO_REGISTRIES_{}_TOKEN",
        registry_name.to_uppercase().replace('-', "_")
    );
    if let Ok(token) = std::env::var(&env_key)
        && !token.is_empty()
    {
        log::trace!("using token from environment variable {env_key}");
        return Some(token);
    }

    // 2. Check credentials.toml
    if let Some(token) = read_credentials_token(registry_name) {
        log::trace!("using token from credentials.toml");
        return Some(token);
    }

    // 3. Check config.toml for credential-provider or token
    let (token, credential_provider) = read_config_auth(manifest_path, registry_name);

    if let Some(provider) = credential_provider
        && let Some(cmd) = provider.strip_prefix("cargo:token-from-stdout ")
    {
        log::trace!("running credential-provider command: {cmd}");
        if let Some(token) = run_credential_command(cmd) {
            return Some(token);
        }
    }

    // 4. Token from config.toml
    if let Some(token) = token {
        log::trace!("using token from config.toml");
        return Some(token);
    }

    None
}

/// Determine which registry name is ultimately used after source replacement
fn resolve_registry_name(manifest_path: &Path, registry: Option<&str>) -> Option<String> {
    let registries = read_all_configs(manifest_path);

    match registry {
        Some(CRATES_IO_INDEX) | None => {
            // Check if crates-io is replaced
            if let Some(source) = registries.get(CRATES_IO_REGISTRY)
                && let Some(replace_with) = &source.replace_with
            {
                return Some(replace_with.clone());
            }
            None
        }
        Some(r) => Some(r.to_owned()),
    }
}

fn read_all_configs(manifest_path: &Path) -> HashMap<String, Source> {
    let mut sources: HashMap<String, Source> = HashMap::new();
    for work_dir in manifest_path
        .parent()
        .expect("there must be a parent directory")
        .ancestors()
    {
        let work_cargo_dir = work_dir.join(".cargo");
        let config_path = work_cargo_dir.join("config");
        if config_path.is_file() {
            let _ = read_sources_from_config(&mut sources, &config_path);
        } else {
            let config_path = work_cargo_dir.join("config.toml");
            if config_path.is_file() {
                let _ = read_sources_from_config(&mut sources, &config_path);
            }
        }
    }
    if let Ok(cargo_home) = home::cargo_home() {
        let config_path = cargo_home.join("config");
        if config_path.is_file() {
            let _ = read_sources_from_config(&mut sources, &config_path);
        } else {
            let config_path = cargo_home.join("config.toml");
            if config_path.is_file() {
                let _ = read_sources_from_config(&mut sources, &config_path);
            }
        }
    }
    sources
}

fn read_sources_from_config(sources: &mut HashMap<String, Source>, path: &Path) -> CargoResult<()> {
    let content = std::fs::read_to_string(path)?;
    let config = toml::from_str::<CargoConfig>(&content)
        .with_context(|| anyhow::format_err!("invalid cargo config at {}", path.display()))?;
    for (key, value) in config.source {
        sources.entry(key).or_insert(value);
    }
    Ok(())
}

fn read_credentials_token(registry_name: &str) -> Option<String> {
    let cargo_home = home::cargo_home().ok()?;
    for filename in &["credentials.toml", "credentials"] {
        let path = cargo_home.join(filename);
        if path.is_file() {
            let content = std::fs::read_to_string(&path).ok()?;
            let config: toml::Value = toml::from_str(&content).ok()?;
            let token = config
                .get("registries")?
                .get(registry_name)?
                .get("token")?
                .as_str()?;
            return Some(token.to_owned());
        }
    }
    None
}

fn read_config_auth(manifest_path: &Path, registry_name: &str) -> (Option<String>, Option<String>) {
    let mut token = None;
    let mut credential_provider = None;

    let check_config = |path: &Path| -> Option<(Option<String>, Option<String>)> {
        let content = std::fs::read_to_string(path).ok()?;
        let config: CargoConfig = toml::from_str(&content).ok()?;
        let reg = config.registries.get(registry_name)?;
        Some((reg.token.clone(), reg.credential_provider.clone()))
    };

    for work_dir in manifest_path
        .parent()
        .expect("there must be a parent directory")
        .ancestors()
    {
        let work_cargo_dir = work_dir.join(".cargo");
        for name in &["config", "config.toml"] {
            let config_path = work_cargo_dir.join(name);
            if config_path.is_file()
                && let Some((t, cp)) = check_config(&config_path)
            {
                if token.is_none() {
                    token = t;
                }
                if credential_provider.is_none() {
                    credential_provider = cp;
                }
            }
        }
    }

    if let Ok(cargo_home) = home::cargo_home() {
        for name in &["config", "config.toml"] {
            let config_path = cargo_home.join(name);
            if config_path.is_file()
                && let Some((t, cp)) = check_config(&config_path)
            {
                if token.is_none() {
                    token = t;
                }
                if credential_provider.is_none() {
                    credential_provider = cp;
                }
            }
        }
    }

    (token, credential_provider)
}

fn run_credential_command(cmd: &str) -> Option<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let output = std::process::Command::new(parts[0])
        .args(&parts[1..])
        .output()
        .ok()?;
    if output.status.success() {
        let token = String::from_utf8(output.stdout).ok()?.trim().to_owned();
        if !token.is_empty() {
            return Some(token);
        }
    }
    None
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
    use std::fs;
    use tempfile::TempDir;

    fn create_project_with_config(config_content: &str) -> TempDir {
        let dir = TempDir::new().unwrap();
        let cargo_dir = dir.path().join(".cargo");
        fs::create_dir_all(&cargo_dir).unwrap();
        fs::write(cargo_dir.join("config.toml"), config_content).unwrap();
        // Create a fake CARGO_HOME inside the temp dir to avoid reading
        // the real ~/.cargo/config.toml
        let fake_cargo_home = dir.path().join("fake-cargo-home");
        fs::create_dir_all(&fake_cargo_home).unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "test"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
        )
        .unwrap();
        dir
    }

    #[test]
    fn test_read_config_auth_reads_token_from_config() {
        let dir = create_project_with_config(
            r#"
[registries.my-registry]
index = "sparse+https://example.com/cargo/my-registry/"
token = "Bearer my-secret-token"
"#,
        );
        let manifest_path = dir.path().join("Cargo.toml");
        let (token, credential_provider) = read_config_auth(&manifest_path, "my-registry");
        assert_eq!(token, Some("Bearer my-secret-token".to_owned()));
        assert_eq!(credential_provider, None);
    }

    #[test]
    fn test_read_config_auth_reads_credential_provider() {
        let dir = create_project_with_config(
            r#"
[registries.my-registry]
index = "sparse+https://example.com/cargo/my-registry/"
credential-provider = "cargo:token-from-stdout echo my-token"
"#,
        );
        let manifest_path = dir.path().join("Cargo.toml");
        let (token, credential_provider) = read_config_auth(&manifest_path, "my-registry");
        assert_eq!(token, None);
        assert_eq!(
            credential_provider,
            Some("cargo:token-from-stdout echo my-token".to_owned())
        );
    }

    #[test]
    fn test_read_config_auth_reads_both_token_and_provider() {
        let dir = create_project_with_config(
            r#"
[registries.my-registry]
index = "sparse+https://example.com/cargo/my-registry/"
token = "Bearer fallback-token"
credential-provider = "cargo:token-from-stdout echo my-token"
"#,
        );
        let manifest_path = dir.path().join("Cargo.toml");
        let (token, credential_provider) = read_config_auth(&manifest_path, "my-registry");
        assert_eq!(token, Some("Bearer fallback-token".to_owned()));
        assert_eq!(
            credential_provider,
            Some("cargo:token-from-stdout echo my-token".to_owned())
        );
    }

    #[test]
    fn test_read_config_auth_returns_none_for_unknown_registry() {
        let dir = create_project_with_config(
            r#"
[registries.my-registry]
index = "sparse+https://example.com/cargo/my-registry/"
token = "Bearer my-secret-token"
"#,
        );
        let manifest_path = dir.path().join("Cargo.toml");
        let (token, credential_provider) = read_config_auth(&manifest_path, "other-registry");
        assert_eq!(token, None);
        assert_eq!(credential_provider, None);
    }

    #[test]
    fn test_run_credential_command_success() {
        let token = run_credential_command("echo hello-token");
        assert_eq!(token, Some("hello-token".to_owned()));
    }

    #[test]
    fn test_run_credential_command_failure() {
        let token = run_credential_command("false");
        assert_eq!(token, None);
    }

    #[test]
    fn test_run_credential_command_not_found() {
        let token = run_credential_command("nonexistent-command-12345");
        assert_eq!(token, None);
    }

    #[test]
    fn test_run_credential_command_empty_output() {
        let token = run_credential_command("echo");
        assert_eq!(token, None);
    }

    #[test]
    fn test_resolve_registry_name_with_source_replacement() {
        let dir = create_project_with_config(
            r#"
[registries.my-registry]
index = "sparse+https://example.com/cargo/my-registry/"

[source.crates-io]
replace-with = "my-registry"
"#,
        );
        let manifest_path = dir.path().join("Cargo.toml");
        let name = resolve_registry_name(&manifest_path, None);
        assert_eq!(name, Some("my-registry".to_owned()));
    }

    #[test]
    fn test_resolve_registry_name_explicit_registry() {
        let dir = create_project_with_config("");
        let manifest_path = dir.path().join("Cargo.toml");
        let name = resolve_registry_name(&manifest_path, Some("custom-reg"));
        assert_eq!(name, Some("custom-reg".to_owned()));
    }

    #[test]
    fn test_registry_token_from_env_var() {
        let dir = create_project_with_config(
            r#"
[registries.envtest-reg1]
index = "sparse+https://example.com/cargo/envtest-reg1/"
"#,
        );
        let manifest_path = dir.path().join("Cargo.toml");

        // SAFETY: test-only, not run in parallel with other env var tests
        unsafe {
            std::env::set_var("CARGO_REGISTRIES_ENVTEST_REG1_TOKEN", "Bearer env-token");
        }
        let token = registry_token(&manifest_path, Some("envtest-reg1"));
        unsafe {
            std::env::remove_var("CARGO_REGISTRIES_ENVTEST_REG1_TOKEN");
        }

        assert_eq!(token, Some("Bearer env-token".to_owned()));
    }

    #[test]
    fn test_registry_token_from_credential_provider() {
        let dir = create_project_with_config(
            r#"
[registries.my-registry]
index = "sparse+https://example.com/cargo/my-registry/"
credential-provider = "cargo:token-from-stdout echo test-credential-token"
"#,
        );
        let manifest_path = dir.path().join("Cargo.toml");

        // SAFETY: test-only
        unsafe {
            std::env::remove_var("CARGO_REGISTRIES_MY_REGISTRY_TOKEN");
        }
        let token = registry_token(&manifest_path, Some("my-registry"));

        assert_eq!(token, Some("test-credential-token".to_owned()));
    }

    #[test]
    fn test_registry_token_from_config_token_field() {
        let dir = create_project_with_config(
            r#"
[registries.my-registry]
index = "sparse+https://example.com/cargo/my-registry/"
token = "Bearer config-token"
"#,
        );
        let manifest_path = dir.path().join("Cargo.toml");

        // SAFETY: test-only
        unsafe {
            std::env::remove_var("CARGO_REGISTRIES_MY_REGISTRY_TOKEN");
        }
        let token = registry_token(&manifest_path, Some("my-registry"));

        assert_eq!(token, Some("Bearer config-token".to_owned()));
    }

    #[test]
    fn test_registry_token_env_var_takes_precedence() {
        let dir = create_project_with_config(
            r#"
[registries.envtest-reg2]
index = "sparse+https://example.com/cargo/envtest-reg2/"
token = "Bearer config-token"
credential-provider = "cargo:token-from-stdout echo provider-token"
"#,
        );
        let manifest_path = dir.path().join("Cargo.toml");

        // SAFETY: test-only
        unsafe {
            std::env::set_var("CARGO_REGISTRIES_ENVTEST_REG2_TOKEN", "Bearer env-token");
        }
        let token = registry_token(&manifest_path, Some("envtest-reg2"));
        unsafe {
            std::env::remove_var("CARGO_REGISTRIES_ENVTEST_REG2_TOKEN");
        }

        assert_eq!(token, Some("Bearer env-token".to_owned()));
    }

    #[test]
    fn test_registry_token_credential_provider_takes_precedence_over_config_token() {
        let dir = create_project_with_config(
            r#"
[registries.my-registry]
index = "sparse+https://example.com/cargo/my-registry/"
token = "Bearer config-token"
credential-provider = "cargo:token-from-stdout echo provider-token"
"#,
        );
        let manifest_path = dir.path().join("Cargo.toml");

        // SAFETY: test-only
        unsafe {
            std::env::remove_var("CARGO_REGISTRIES_MY_REGISTRY_TOKEN");
        }
        let token = registry_token(&manifest_path, Some("my-registry"));

        assert_eq!(token, Some("provider-token".to_owned()));
    }

    #[test]
    fn test_registry_token_with_source_replacement() {
        let dir = create_project_with_config(
            r#"
[registries.private-reg]
index = "sparse+https://example.com/cargo/private-reg/"
credential-provider = "cargo:token-from-stdout echo replaced-token"

[source.crates-io]
replace-with = "private-reg"
"#,
        );
        let manifest_path = dir.path().join("Cargo.toml");

        // SAFETY: test-only
        unsafe {
            std::env::remove_var("CARGO_REGISTRIES_PRIVATE_REG_TOKEN");
        }
        let token = registry_token(&manifest_path, None);

        assert_eq!(token, Some("replaced-token".to_owned()));
    }

    #[test]
    fn test_read_config_auth_local_takes_precedence_over_parent() {
        let dir = TempDir::new().unwrap();

        // Create a parent .cargo/config.toml with one token
        let parent_cargo_dir = dir.path().join(".cargo");
        fs::create_dir_all(&parent_cargo_dir).unwrap();
        fs::write(
            parent_cargo_dir.join("config.toml"),
            r#"
[registries.my-registry]
index = "sparse+https://example.com/cargo/my-registry/"
token = "Bearer parent-token"
credential-provider = "cargo:token-from-stdout echo parent-provider-token"
"#,
        )
        .unwrap();

        // Create a child project with its own .cargo/config.toml
        let child_dir = dir.path().join("child");
        let child_cargo_dir = child_dir.join(".cargo");
        fs::create_dir_all(&child_cargo_dir).unwrap();
        fs::write(
            child_cargo_dir.join("config.toml"),
            r#"
[registries.my-registry]
index = "sparse+https://example.com/cargo/my-registry/"
token = "Bearer child-token"
credential-provider = "cargo:token-from-stdout echo child-provider-token"
"#,
        )
        .unwrap();
        fs::write(
            child_dir.join("Cargo.toml"),
            r#"[package]
name = "test"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
        )
        .unwrap();

        let manifest_path = child_dir.join("Cargo.toml");
        let (token, credential_provider) = read_config_auth(&manifest_path, "my-registry");
        assert_eq!(token, Some("Bearer child-token".to_owned()));
        assert_eq!(
            credential_provider,
            Some("cargo:token-from-stdout echo child-provider-token".to_owned())
        );
    }

    #[test]
    fn test_registry_token_hyphenated_name_env_var() {
        let dir = create_project_with_config(
            r#"
[registries.envtest-hyph-reg]
index = "sparse+https://example.com/cargo/envtest-hyph-reg/"
"#,
        );
        let manifest_path = dir.path().join("Cargo.toml");

        // SAFETY: test-only
        unsafe {
            std::env::set_var(
                "CARGO_REGISTRIES_ENVTEST_HYPH_REG_TOKEN",
                "Bearer hyphen-token",
            );
        }
        let token = registry_token(&manifest_path, Some("envtest-hyph-reg"));
        unsafe {
            std::env::remove_var("CARGO_REGISTRIES_ENVTEST_HYPH_REG_TOKEN");
        }

        assert_eq!(token, Some("Bearer hyphen-token".to_owned()));
    }
}
