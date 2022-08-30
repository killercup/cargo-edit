use cargo::util::command_prelude::*;
use clap::Command;

pub fn main(config: &mut Config) -> CliResult {
    let args = cli().try_get_matches()?;
    let (cmd, subcommand_args) = args.subcommand().expect("subcommand_required(true)");
    execute_subcommand(config, cmd, subcommand_args)?;
    Ok(())
}

fn cli() -> Command<'static> {
    Command::new("cargo")
        .bin_name("cargo")
        .arg(
            Arg::new("unstable-features")
                .help("Unstable (nightly-only) flags")
                .short('Z')
                .value_name("FLAG")
                .action(ArgAction::Append)
                .global(true),
        )
        .subcommands(crate::commands::builtin())
        .subcommand_required(true)
}

fn execute_subcommand(config: &mut Config, cmd: &str, subcommand_args: &ArgMatches) -> CliResult {
    let exec = crate::commands::builtin_exec(cmd).expect("all of `builtin` supported");
    exec(config, subcommand_args)
}

#[test]
fn verify_app() {
    cli().debug_assert()
}
