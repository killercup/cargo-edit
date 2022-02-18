use cargo_add::ops::cargo_add::CargoResult;
use clap::ArgMatches;
use clap::Command;

pub fn main(config: &cargo::util::Config) -> CargoResult<()> {
    let args = cli().get_matches();
    let (cmd, subcommand_args) = args.subcommand().expect("subcommand_required(true)");
    execute_subcommand(config, cmd, subcommand_args)
}

fn cli() -> Command<'static> {
    Command::new("cargo")
        .subcommands(crate::commands::builtin())
        .subcommand_required(true)
}

fn execute_subcommand(
    config: &cargo::Config,
    cmd: &str,
    subcommand_args: &ArgMatches,
) -> CargoResult<()> {
    let exec = crate::commands::builtin_exec(cmd).expect("all of `builtin` supported");
    exec(config, subcommand_args)
}

#[test]
fn verify_app() {
    cli().debug_assert()
}
