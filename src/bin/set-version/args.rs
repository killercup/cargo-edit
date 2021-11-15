use structopt::{clap::AppSettings, StructOpt};

#[derive(Debug, StructOpt)]
#[structopt(bin_name = "cargo")]
pub(crate) enum Command {
    /// Change a package's version in the local manifest file (i.e. Cargo.toml).
    #[structopt(name = "set-version")]
    Version(Args),
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

    /// Specify the version metadata field (e.g. a wrapped libraries version)
    #[structopt(short = "m", long)]
    pub metadata: Option<String>,

    /// Path to the manifest to upgrade
    #[structopt(flatten)]
    pub(crate) manifest: clap_cargo::Manifest,

    #[structopt(flatten)]
    pub(crate) workspace: clap_cargo::Workspace,

    /// Print changes to be made without making them.
    #[structopt(long = "dry-run")]
    pub(crate) dry_run: bool,
}
