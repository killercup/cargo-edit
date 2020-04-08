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
    CargoMetadata(#[from] cargo_metadata::Error),
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
    /// An error from the serde_json crate
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// An error from converting bytes to a utf8 string
    #[error(transparent)]
    FromUtf8(#[from] std::string::FromUtf8Error),

    /// A value was expected but we got `None` instead
    #[error("Expected a value")]
    ExpectedValue,

    /// Given crate name is empty
    #[error("Found empty crate name")]
    EmptyCrateName,

    /// No versions available
    #[error("No available versions exist. Either all were yanked or only prerelease versions exist. Trying with the --allow-prerelease flag might solve the issue.")]
    NoVersionsAvailable,

    /// A resource was invalid
    #[error("Invalid: {0}")]
    Invalid(String),

    /// Found a workspace manifest when expecting a normal manifest
    #[error("Found virtual manifest, but this command requires running against an actual package in this workspace.")]
    UnexpectedRootManifest,

    /// Unable to find the specified resource
    #[error("The resource `{0}` could not be found.")]
    NotFound(String),

    /// Unable to find the specified resource
    #[error("The resource `{0}` could not be found in `{1}`.")]
    NotFoundIn(String, String),

    /// Failed to parse a version for a dependency
    #[error("The version `{0}` for the dependency `{1}` couldn't be parsed")]
    ParseReqVersion(String, String, #[source] semver::ReqParseError),

    /// Failed to parse a version for a dependency
    #[error("The version `{0}` for the dependency `{1}` couldn't be parsed")]
    ParseVersion(String, String, #[source] semver::SemVerError),

    /// Unable to get a crate from a git repository
    #[error("Failed to fetch crate from git")]
    FetchCrateFromGit(#[source] reqwest::Error),

    /// Something was wrong with the git url
    #[error("There was a problem with the git repo url '{0}'")]
    GitUrl(String),

    /// Found a virtual manifest instead of an actual manifest
    #[error("Found virtual manifest, but this command requires running against an  actual package in this workspace. Try adding `--workspace`.")]
    VirtualManifest,
}

/// Library-specific alias for `Result`
pub type Result<T> = std::result::Result<T, Error>;
