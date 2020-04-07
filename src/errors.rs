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
    #[error("The version `{0}` for the dependency `{1}` couldn't be parsed")]
    ParseReqVersion(String, String, #[source] semver::ReqParseError),

    /// Failed to parse a version for a dependency
    #[error("The version `{0}` for the dependency `{1}` couldn't be parsed")]
    ParseVersion(String, String, #[source] semver::SemVerError),

    /// An invalid crate version requirement was encountered
    #[error("Invalid crate version requirement")]
    InvalidCrateVersionReq(#[source] semver::ReqParseError),

    /// Unable to get crate name from a URI
    #[error("Unable to obtain crate informations from `{0}`.\n")]
    ParseCrateNameFromUri(String),

    /// Unable to get a crate from a git repository
    #[error("Failed to fetch crate from git")]
    FetchCrateFromGit(#[source] reqwest::Error),

    /// Received an invalid response from a git repository
    #[error("Git response not a valid `String`")]
    InvalidGitResponse(#[source] std::io::Error),

    /// Could not read manifest contents
    #[error("Failed to read manifest contents")]
    ManifestReadError(#[source] std::io::Error),

    /// Could not parse Cargo.toml
    #[error("Unable to parse Cargo.toml")]
    ManifestParseError(#[source] Box<Error>),

    /// Cargo.toml contained invalid TOML
    #[error("Manifest not valid TOML")]
    ManifestInvalidToml(#[source] toml_edit::TomlError),

    /// Could not found Cargo.toml
    #[error("Failed to find Cargo.toml")]
    ManifestNotLocated(#[source] std::io::Error),

    /// Could not get cargo metadata
    #[error("Failed to get cargo file metadata")]
    GetCargoMetadata(#[source] std::io::Error),

    /// Could not get current directory
    #[error("Failed to get current directory")]
    GetCwd(#[source] std::io::Error),

    /// Could not set output colour
    #[error("Failed to set output colour")]
    SetOutputColour(#[source] std::io::Error),

    /// Could not write upgrade message
    #[error("Failed to write upgrade message")]
    WriteUpgradeMessage(#[source] std::io::Error),

    /// Could not clear output colour
    #[error("Failed to clear output colour")]
    ClearOutputColour(#[source] std::io::Error),

    /// Could not write upgraded versions
    #[error("Failed to write upgrade versions")]
    WriteUpgradeVersions(#[source] std::io::Error),

    /// Could not print upgrade message
    #[error("Failed to print upgrade message")]
    PrintUpgradeMessage(#[source] std::io::Error),

    /// Could not truncate Cargo.toml
    #[error("Failed to truncate Cargo.toml")]
    TruncateCargoToml(#[source] std::io::Error),

    /// Could not write updated Cargo.toml
    #[error("Failed to write updated Cargo.toml")]
    WriteUpdatedCargoToml(#[source] std::io::Error),

    /// Missing Version Field
    #[error("Missing version field")]
    MissingVersionField,

    /// Could not write new manifest contents
    #[error("Failed to write new manifest contents")]
    WriteNewManifestContents(#[source] Box<Error>),

    /// Could not open Cargo.toml
    #[error("Unable to open local Cargo.toml")]
    OpenLocalManifest(#[source] Box<Error>),

    /// Git repo URL seems incomplete
    #[error("Git repo url seems incomplete")]
    IncompleteGitUrl,

    /// Could not parse git repo URL
    #[error("Unable to parse git repo URL")]
    ParseGitUrl,

    /// Found a virtual manifest instead of an actual manifest
    #[error("Found virtual manifest, but this command requires running against an  actual package in this workspace. Try adding `--workspace`.")]
    VirtualManifest,
}

/// Library-specific alias for `Result`
pub type Result<T> = std::result::Result<T, Error>;
