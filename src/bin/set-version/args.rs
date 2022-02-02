use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[clap(bin_name = "cargo")]
pub(crate) enum Command {
    /// Change a package's version in the local manifest file (i.e. Cargo.toml).
    #[clap(name = "set-version")]
    Version(Args),
}

#[derive(Debug, Parser)]
#[clap(about, version)]
#[clap(group = clap::ArgGroup::new("ver").multiple(false))]
pub(crate) struct Args {
    /// Version to change manifests to
    #[clap(parse(try_from_str), group = "ver")]
    pub(crate) target: Option<semver::Version>,

    /// Increment manifest version
    #[clap(
        long,
        possible_values(crate::version::BumpLevel::variants()),
        group = "ver"
    )]
    pub(crate) bump: Option<crate::version::BumpLevel>,

    /// Specify the version metadata field (e.g. a wrapped libraries version)
    #[clap(short, long)]
    pub metadata: Option<String>,

    /// Path to the manifest to upgrade
    #[clap(
        long,
        value_name = "PATH",
        parse(from_os_str),
        conflicts_with = "pkgid"
    )]
    pub(crate) manifest_path: Option<PathBuf>,

    /// Package id of the crate to change the version of.
    #[clap(
        long = "package",
        short = 'p',
        value_name = "PKGID",
        conflicts_with = "manifest-path",
        conflicts_with = "all",
        conflicts_with = "workspace"
    )]
    pub(crate) pkgid: Option<String>,

    /// Modify all packages in the workspace.
    #[clap(
        long,
        help = "[deprecated in favor of `--workspace`]",
        conflicts_with = "workspace",
        conflicts_with = "pkgid"
    )]
    pub(crate) all: bool,

    /// Modify all packages in the workspace.
    #[clap(long, conflicts_with = "all", conflicts_with = "pkgid")]
    pub(crate) workspace: bool,

    /// Print changes to be made without making them.
    #[clap(long)]
    pub(crate) dry_run: bool,

    /// Crates to exclude and not modify.
    #[clap(long)]
    pub(crate) exclude: Vec<String>,

    /// Unstable (nightly-only) flags
    #[clap(short = 'Z', value_name = "FLAG", global = true, arg_enum)]
    pub unstable_features: Vec<UnstableOptions>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ArgEnum)]
pub enum UnstableOptions {}

#[test]
fn verify_app() {
    use clap::IntoApp;
    Command::into_app().debug_assert()
}
