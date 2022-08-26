use cargo::util::command_prelude::*;
use cargo::CargoResult;
use clap::{Parser, Subcommand};

pub fn main(config: &mut Config) -> CliResult {
    let args = Cli::try_parse()?;
    execute_subcommand(config, &args)?;
    Ok(())
}

#[derive(Debug, Parser)]
#[clap(bin_name = "cargo")]
pub struct Cli {
    /// Unstable (nightly-only) flags
    #[clap(short = 'Z', value_name = "FLAG", global = true, arg_enum)]
    unstable_features: Vec<UnstableOptions>,

    #[clap(subcommand)]
    subcommand: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Rm(crate::commands::rm::RmArgs),
}

impl Command {
    pub fn exec(&self) -> CargoResult<()> {
        match self {
            Self::Rm(rm) => rm.exec(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ArgEnum)]
enum UnstableOptions {}

fn execute_subcommand(_config: &mut Config, args: &Cli) -> CliResult {
    args.subcommand.exec()?;
    Ok(())
}

#[test]
fn verify_app() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}
