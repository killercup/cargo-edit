use std::fmt::Display;

/// Common result type
pub type CargoResult<T> = anyhow::Result<T>;

/// Common error type
pub type Error = anyhow::Error;

pub use anyhow::Context;

/// CLI-specific result
pub type CliResult = Result<(), CliError>;

#[derive(Debug)]
/// The CLI error is the error type used at Cargo's CLI-layer.
///
/// All errors from the lib side of Cargo will get wrapped with this error.
/// Other errors (such as command-line argument validation) will create this
/// directly.
pub struct CliError {
    /// The error to display. This can be `None` in rare cases to exit with a
    /// code without displaying a message. For example `cargo run -q` where
    /// the resulting process exits with a nonzero code (on Windows), or an
    /// external subcommand that exits nonzero (we assume it printed its own
    /// message).
    pub error: Option<anyhow::Error>,
    /// The process exit code.
    pub exit_code: i32,
}

impl CliError {
    /// Attach an error code to an error
    pub fn new(error: anyhow::Error, code: i32) -> CliError {
        CliError {
            error: Some(error),
            exit_code: code,
        }
    }

    /// Silent error
    pub fn code(code: i32) -> CliError {
        CliError {
            error: None,
            exit_code: code,
        }
    }
}

impl From<anyhow::Error> for CliError {
    fn from(err: anyhow::Error) -> CliError {
        CliError::new(err, 101)
    }
}

impl From<clap::Error> for CliError {
    fn from(err: clap::Error) -> CliError {
        #[allow(clippy::bool_to_int_with_if)]
        let code = if err.use_stderr() { 1 } else { 0 };
        CliError::new(err.into(), code)
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> CliError {
        CliError::new(err.into(), 1)
    }
}

pub(crate) fn non_existent_table_err(table: impl Display) -> Error {
    anyhow::format_err!("The table `{}` could not be found.", table)
}

pub(crate) fn non_existent_dependency_err(name: impl Display, table: impl Display) -> Error {
    anyhow::format_err!(
        "The dependency `{}` could not be found in `{}`.",
        name,
        table,
    )
}

pub(crate) fn invalid_cargo_config() -> Error {
    anyhow::format_err!("Invalid cargo config")
}

pub(crate) fn unsupported_version_req(req: impl Display) -> Error {
    anyhow::format_err!("Support for modifying {} is currently unsupported", req)
}

pub(crate) fn invalid_release_level(actual: impl Display, version: impl Display) -> Error {
    anyhow::format_err!("Cannot increment the {} field for {}", actual, version)
}
