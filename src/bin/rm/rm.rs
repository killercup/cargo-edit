use cargo_edit::CargoResult;
use clap::Args;
use std::path::PathBuf;

/// Remove a dependency from a Cargo.toml manifest file.
#[derive(Debug, Args)]
#[command(version)]
pub struct RmArgs {
    /// Dependencies to be removed
    #[arg(value_name = "DEP_ID", required = true)]
    crates: Vec<String>,

    /// Remove as development dependency
    #[arg(long, short = 'D', conflicts_with = "build", help_heading = "Section")]
    dev: bool,

    /// Remove as build dependency
    #[arg(long, short = 'B', conflicts_with = "dev", help_heading = "Section")]
    build: bool,

    /// Remove as dependency from the given target platform
    #[arg(long, value_parser = clap::builder::NonEmptyStringValueParser::new(), help_heading = "Section")]
    target: Option<String>,

    /// Path to the manifest to remove a dependency from
    #[arg(long, value_name = "PATH")]
    manifest_path: Option<PathBuf>,

    /// Package to remove from
    #[arg(long = "package", short = 'p', value_name = "PKGID")]
    pkgid: Option<String>,

    /// Unstable (nightly-only) flags
    #[arg(short = 'Z', value_name = "FLAG", global = true, value_enum)]
    unstable_features: Vec<UnstableOptions>,

    /// Don't actually write the manifest
    #[arg(long)]
    dry_run: bool,

    /// Do not print any output in case of success
    #[arg(long, short)]
    quiet: bool,
}

impl RmArgs {
    pub fn exec(&self) -> CargoResult<()> {
        anyhow::bail!(
            "`cargo rm` has been merged into cargo 1.66+ as of cargo-edit 0.12, either
- Upgrade cargo, like with `rustup update`
- Downgrade `cargo-edit`, like with `cargo install cargo-edit --version 0.11`"
        );
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum UnstableOptions {}
