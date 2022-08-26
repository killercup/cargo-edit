use cargo_rm::CargoResult;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[clap(bin_name = "cargo")]
pub struct Cli {
    /// Unstable (nightly-only) flags
    #[clap(short = 'Z', value_name = "FLAG", global = true, arg_enum)]
    unstable_features: Vec<UnstableOptions>,

    #[clap(subcommand)]
    subcommand: Command,
}

impl Cli {
    pub fn exec(self) -> CargoResult<()> {
        self.subcommand.exec()
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Rm(crate::commands::rm::RmArgs),
}

impl Command {
    pub fn exec(self) -> CargoResult<()> {
        match self {
            Self::Rm(rm) => rm.exec(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ArgEnum)]
enum UnstableOptions {}

#[test]
fn verify_app() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}
