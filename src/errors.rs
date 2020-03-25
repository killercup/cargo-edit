use thiserror::Error as ThisError;

/// Main error type
#[derive(Debug, ThisError)]
pub enum Error {
    // foreign links
    /// An error from the std::io module
    #[error(transparent)]
    Io(#[from] std::io::Error),
    ///  An error from the git2 crate
    #[error(transparent)]
    Git(#[from] git2::Error),
    /// An error from the cargo_metadata crate
    #[error(transparent)]
    CargoMetadata(#[from] failure::Compat<::cargo_metadata::Error>),
    /// An error from the toml_edit crate
    #[error(transparent)]
    TomlEditParse(#[from] toml_edit::TomlError),
    /// A ReqParseError from the semver crate
    #[error(transparent)]
    SemVerParse(#[from] semver::ReqParseError),
    /// A SemVerError from the semver crate
    #[error(transparent)]
    SemVer(#[from] semver::SemVerError),
    /// An error from the reqwest crate
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    // links
    /// Failed to read home directory
    #[error("Failed to read home directory")]
    ReadHomeDirFailure,

    /// Invalid JSON in registry index
    #[error("Invalid JSON in registry index")]
    InvalidSummaryJson,

    /// Given crate name is empty
    #[error("Found empty crate name")]
    EmptyCrateName,

    /// No crate by that name exists
    #[error("The crate `{0}` could not be found in registry index.")]
    NoCrate(String),

    /// No versions available
    #[error("No available versions exist. Either all were yanked or only prerelease versions exist. Trying with the --allow-prerelease flag might solve the issue.")]
    NoVersionsAvailable,

    /// Unable to parse external Cargo.toml
    #[error("Unable to parse external Cargo.toml")]
    ParseCargoToml,

    /// Cargo.toml could not be found.
    #[error("Unable to find Cargo.toml")]
    MissingManifest,

    /// Cargo.toml is valid toml, but doesn't contain the expected fields
    #[error("Cargo.toml missing expected `package` or `project` fields")]
    InvalidManifest,

    /// Found a workspace manifest when expecting a normal manifest
    #[error("Found virtual manifest, but this command requires running against an actual package in this workspace.")]
    UnexpectedRootManifest,

    /// The TOML table could not be found.
    #[error("The table `{0}` could not be found.")]
    NonExistentTable(String),

    /// The dependency could not be found.
    #[error("The dependency `{name}` could not be found in `{table}`.")]
    NonExistentDependency {
        /// Name of the non-existent dependency
        name: String,
        /// Table of dependencies
        table: String,
    },

    /// Config of cargo is invalid
    #[error("Invalid cargo config")]
    InvalidCargoConfig,

    /// Unable to find the source specified by 'replace-with'
    #[error("The source '{0}' could not be found")]
    NoSuchSourceFound(String),

    /// Unable to find the specified registry
    #[error("The registry '{0}' could not be found")]
    NoSuchRegistryFound(String),

    /// Failed to parse a version for a dependency
    #[error("The version `{version}` for the dependency `{dep}` couldn't be parsed")]
    ParseVersion {
        /// The invalid version
        version: String,
        /// The dependency with an invalid version
        dep: String,
    },

    /// A string error
    #[error("{0}")]
    Custom(String),

    /// Wraps another error in order to provide more context
    #[error("{error}")]
    Wrapped {
        /// Current error
        error: Box<Error>,
        /// Source error
        source: Box<Error>,
    },
}

impl Error {
    /// Transforms an error into an Error, and adds a string as context
    pub fn wrap<T, U>(error: T, source: U) -> Error
    where
        T: Into<Error>,
        U: Into<Error>,
    {
        Error::Wrapped {
            error: Box::new(error.into()),
            source: Box::new(source.into()),
        }
    }

    /// Takes an existing Error and turns it into an Error::Wrapped, with the given source error
    ///
    /// Basically equivalent to `Error::wrap(error.into(), source)` but slightly less verbose than
    /// something like `Error::wrap(Error::SomeError, source)`
    pub fn wraps<U>(self, source: U) -> Error
    where
        U: Into<Error>,
    {
        Error::wrap(self, source)
    }
}

impl From<String> for Error {
    fn from(s: String) -> Error {
        Error::Custom(s)
    }
}

impl<'a> From<&'a str> for Error {
    fn from(e: &'a str) -> Error {
        Error::Custom(e.into())
    }
}

/// Result wrapper type
pub type Result<T> = std::result::Result<T, Error>;
