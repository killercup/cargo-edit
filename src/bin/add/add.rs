#![allow(clippy::bool_assert_comparison)]

use cargo_edit::CargoResult;
use clap::Args;

/// Add dependencies to a Cargo.toml manifest file.
#[derive(Debug, Args)]
#[command(version)]
#[command(after_help = "\
Examples:
  $ cargo add regex --build
  $ cargo add trycmd --dev
  $ cargo add ./crate/parser/
  $ cargo add serde +derive serde_json
")]
#[command(override_usage = "\
       cargo add [OPTIONS] <DEP>[@<VERSION>] [+<FEATURE>,...] ...
       cargo add [OPTIONS] <DEP_PATH> [+<FEATURE>,...] ...")]
pub struct AddArgs {
    /// Reference to a package to add as a dependency
    ///
    /// You can reference a packages by:{n}
    /// - `<name>`, like `cargo add serde` (latest version will be used){n}
    /// - `<name>@<version-req>`, like `cargo add serde@1` or `cargo add serde@=1.0.38`{n}
    /// - `<path>`, like `cargo add ./crates/parser/`
    ///
    /// Additionally, you can specify features for a dependency by following it with a
    /// `+<FEATURE>`.
    #[arg(value_name = "DEP_ID")]
    pub crates: Vec<String>,

    /// Disable the default features
    #[arg(long)]
    no_default_features: bool,
    /// Re-enable the default features
    #[arg(long, overrides_with = "no_default_features")]
    default_features: bool,

    /// Space-separated list of features to add
    ///
    /// Alternatively, you can specify features for a dependency by following it with a
    /// `+<FEATURE>`.
    #[arg(short = 'F', long)]
    pub features: Option<Vec<String>>,

    /// Mark the dependency as optional
    ///
    /// The package name will be exposed as feature of your crate.
    #[arg(long, conflicts_with = "dev")]
    pub optional: bool,

    /// Mark the dependency as required
    ///
    /// The package will be removed from your features.
    #[arg(long, conflicts_with = "dev", overrides_with = "optional")]
    pub no_optional: bool,

    /// Rename the dependency
    ///
    /// Example uses:{n}
    /// - Depending on multiple versions of a crate{n}
    /// - Depend on crates with the same name from different registries
    #[arg(long, short)]
    pub rename: Option<String>,

    /// Package registry for this dependency
    #[arg(long, conflicts_with = "git")]
    pub registry: Option<String>,

    /// Add as development dependency
    ///
    /// Dev-dependencies are not used when compiling a package for building, but are used for compiling tests, examples, and benchmarks.
    ///
    /// These dependencies are not propagated to other packages which depend on this package.
    #[arg(short = 'D', long, help_heading = "Section", group = "section")]
    pub dev: bool,

    /// Add as build dependency
    ///
    /// Build-dependencies are the only dependencies available for use by build scripts (`build.rs`
    /// files).
    #[arg(short = 'B', long, help_heading = "Section", group = "section")]
    pub build: bool,

    /// Add as dependency to the given target platform.
    #[arg(long, help_heading = "Section", group = "section")]
    pub target: Option<String>,

    /// Path to `Cargo.toml`
    #[arg(long, value_name = "PATH")]
    pub manifest_path: Option<std::path::PathBuf>,

    /// Package to modify
    #[arg(short = 'p', long = "package", value_name = "PKGID")]
    pub pkgid: Option<String>,

    /// Run without accessing the network
    #[arg(long)]
    pub offline: bool,

    /// Don't actually write the manifest
    #[arg(long)]
    pub dry_run: bool,

    /// Do not print any output in case of success.
    #[arg(long)]
    pub quiet: bool,

    /// Git repository location
    ///
    /// Without any other information, cargo will use latest commit on the main branch.
    #[arg(long, value_name = "URI", help_heading = "Unstable")]
    pub git: Option<String>,

    /// Git branch to download the crate from.
    #[arg(
        long,
        value_name = "BRANCH",
        help_heading = "Unstable",
        requires = "git",
        group = "git-ref"
    )]
    pub branch: Option<String>,

    /// Git tag to download the crate from.
    #[arg(
        long,
        value_name = "TAG",
        help_heading = "Unstable",
        requires = "git",
        group = "git-ref"
    )]
    pub tag: Option<String>,

    /// Git reference to download the crate from
    ///
    /// This is the catch all, handling hashes to named references in remote repositories.
    #[arg(
        long,
        value_name = "REV",
        help_heading = "Unstable",
        requires = "git",
        group = "git-ref"
    )]
    pub rev: Option<String>,
}

impl AddArgs {
    pub fn exec(self) -> CargoResult<()> {
        anyhow::bail!(
            "`cargo add` has been merged into cargo 1.62+ as of cargo-edit 0.10, either
- Upgrade cargo, like with `rustup update`
- Downgrade `cargo-edit`, like with `cargo install cargo-edit --version 0.9.1`"
        );
    }
}
