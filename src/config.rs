//! Module containing some helpers for accessing cargo config

use std::path::{Path, PathBuf};
use crate::errors::*;

/// Iterates over configs starting with the one closest to the work_dir and ending
/// with the cargo_home config.
pub fn configs<'a>(work_dir: &'a Path) -> impl Iterator<Item=PathBuf> + 'a {
    let home = match cargo_home() {
        Ok(home) => if home.is_file() {
            Some(home)
        } else {
            None
        },
        Err(..) => None,
    };
    work_dir.ancestors().filter_map(|dir| {
        let path = dir.join(".cargo").join("config");
        if path.is_file() {
            Some(path)
        } else {
            None
        }
    }).chain(home.map(|dir| dir.join("config")))
}

/// Returns the location of the cargo home directory, if possible
pub fn cargo_home() -> Result<PathBuf> {
    let default_cargo_home = dirs::home_dir()
        .map(|x| x.join(".cargo"))
        .chain_err(|| ErrorKind::ReadHomeDirFailure)?;
    let cargo_home = std::env::var("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or(default_cargo_home);
    Ok(cargo_home)
}

/// Takes a version string and a format string and if possible,
/// formats the version according to the format string. Currently accepts
/// 3 placeholders in the format string, `{MAJOR}`, `{MINOR}`, and `{PATCH}`
pub fn format_version(version: &str, fmt: &Option<String>) -> String {
    let fmt = if let Some(ref fmt) = fmt {
        fmt
    } else {
        return version.into();
    };
    let version = if let Ok(version) = semver::Version::parse(version) {
        version
    } else {
        return version.into();
    };
    fmt.replace("{MAJOR}", &version.major.to_string())
        .replace("{MINOR}", &version.minor.to_string())
        .replace("{PATCH}", &version.patch.to_string())
}

/// Allows a user to get a config value inside the `[cargo-edit]` config table of
/// a .cargo/config file. All keys are relative to this table, so calling this like
///
/// ```rust,ignore
/// config::get(manifest_path, "add.version_fmt")
/// ```
///
/// will look for a value that looks like this in the config file:
///
/// ```toml,ignore
/// [cargo-edit.add]
/// version_fmt = "{major}.{minor}"
/// ```
pub fn get(manifest_path: impl AsRef<Path>, key: &str) -> Option<toml::Value> {
    'outer: for config in configs(manifest_path.as_ref()) {
        let content = std::fs::read(config).ok()?;
        let config = toml::from_slice::<toml::Value>(&content).ok()?;
        let mut obj = config.get("cargo-edit")?;
        for part in key.split('.') {
            if let Some(val) = obj.get(part) {
                obj = val;
            } else {
                continue 'outer;
            };
        }
        return Some(obj.clone());
    }
    None
}
