use std::fmt::Display;

use anyhow::Error;

pub(crate) fn no_crate_err(name: impl Display) -> Error {
    anyhow::format_err!("The crate `{}` could not be found in registry index.", name)
}

pub(crate) fn parse_manifest_err() -> Error {
    anyhow::format_err!("Unable to parse external Cargo.toml")
}

pub(crate) fn non_existent_table_err(table: impl Display) -> Error {
    anyhow::format_err!("The table `{}` could not be found.", table)
}

pub(crate) fn invalid_cargo_config() -> Error {
    anyhow::format_err!("Invalid cargo config")
}

pub(crate) fn parse_version_err(version: impl Display, dep: impl Display) -> Error {
    anyhow::format_err!(
        "The version `{}` for the dependency `{}` couldn't be parsed",
        version,
        dep
    )
}
