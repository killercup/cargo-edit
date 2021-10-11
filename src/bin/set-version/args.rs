use std::path::PathBuf;

use structopt::{clap::AppSettings, StructOpt};

#[derive(Debug, StructOpt)]
#[structopt(bin_name = "cargo")]
pub(crate) enum Command {
    /// Change a package's version in the local manifest file (i.e. Cargo.toml).
    #[structopt(name = "set-version")]
    Version(Args),
}

pub(crate) enum RawUpdateMessage {
    Semver(semver::Version),
    BumpLevel(crate::version::BumpLevel)
}

fn parse_raw_update_message(input: &str) -> Result<RawUpdateMessage, semver::Error> {
    if let Ok(bump_level) = crate::version::BumpLevel::from_str(input) {
        Ok(RawUpdateMessage::BumpLevel(bump_level))
    } else{
        semver::Version::from_str(input).map(|ver| RawUpdateMessage::Semver(ver))
    }
}

#[derive(Debug, StructOpt)]
#[structopt(setting = AppSettings::ColoredHelp)]
#[structopt(group = structopt::clap::ArgGroup::with_name("version").multiple(false))]
pub(crate) struct Args {
    /// Version to change manifests to
    #[structopt(parse(try_from_str), group = "version")]
    pub(crate) target: Option<semver::Version>,

    /// Increment manifest version
    #[structopt(long, possible_values(&crate::version::BumpLevel::variants()), group = "version")]
    pub(crate) bump: Option<crate::version::BumpLevel>,

    /// Combination of the two arguments
    #[structopt(long, parse(try_from_str = parse_raw_update_message), group = "version")]
    pub(crate) raw: Option<RawUpdateMessage>,

    /// Specify the version metadata field (e.g. a wrapped libraries version)
    #[structopt(short = "m", long)]
    pub metadata: Option<String>,

    /// Path to the manifest to upgrade
    #[structopt(long = "manifest-path", value_name = "path", conflicts_with = "pkgid")]
    pub(crate) manifest_path: Option<PathBuf>,

    /// Package id of the crate to change the version of.
    #[structopt(
        long = "package",
        short = "p",
        value_name = "pkgid",
        conflicts_with = "path",
        conflicts_with = "all",
        conflicts_with = "workspace"
    )]
    pub(crate) pkgid: Option<String>,

    /// Modify all packages in the workspace.
    #[structopt(
        long = "all",
        help = "[deprecated in favor of `--workspace`]",
        conflicts_with = "workspace",
        conflicts_with = "pkgid"
    )]
    pub(crate) all: bool,

    /// Modify all packages in the workspace.
    #[structopt(long = "workspace", conflicts_with = "all", conflicts_with = "pkgid")]
    pub(crate) workspace: bool,

    /// Print changes to be made without making them.
    #[structopt(long = "dry-run")]
    pub(crate) dry_run: bool,

    /// Crates to exclude and not modify.
    #[structopt(long)]
    pub(crate) exclude: Vec<String>,

    /// Command panics if set version does not increase version
    #[structopt(long = "panic-if-equal", default_value)]
    pub(crate) panic_if_equal: bool,

    /// Outputs only the new version
    #[structopt(long = "output-new-version", default_value)]
    pub(crate) output_new_version: bool,
}
