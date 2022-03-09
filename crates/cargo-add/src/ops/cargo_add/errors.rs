use std::fmt::Display;

use anyhow::Error;

pub(crate) fn parse_manifest_err() -> Error {
    anyhow::format_err!("Unable to parse external Cargo.toml")
}

pub(crate) fn non_existent_table_err(table: impl Display) -> Error {
    anyhow::format_err!("The table `{}` could not be found.", table)
}

pub(crate) fn invalid_cargo_config() -> Error {
    anyhow::format_err!("Invalid cargo config")
}
