use cargo::CargoResult;
use cargo::CliResult;
use clap::Arg;
use clap::ArgAction;
use clap::ArgMatches;
use clap::Command;

pub fn main(config: &mut cargo::util::Config) -> CliResult {
    let args = match cli().try_get_matches() {
        Ok(args) => args,
        Err(e) => {
            return Err(e.into());
        }
    };
    let (cmd, subcommand_args) = args.subcommand().expect("subcommand_required(true)");
    execute_subcommand(config, cmd, subcommand_args)
}

fn cli() -> Command<'static> {
    Command::new("cargo")
        .arg(
            Arg::new("unstable-features")
                .help("Unstable (nightly-only) flags to Cargo, see 'cargo -Z help' for details")
                .short('Z')
                .value_name("FLAG")
                .action(ArgAction::Append)
                .global(true),
        )
        .subcommands(crate::commands::builtin())
        .subcommand_required(true)
}

fn execute_subcommand(
    config: &mut cargo::Config,
    cmd: &str,
    subcommand_args: &ArgMatches,
) -> CliResult {
    config_configure(config, subcommand_args)?;
    let exec = crate::commands::builtin_exec(cmd).expect("all of `builtin` supported");
    exec(config, subcommand_args)
}

#[test]
fn verify_app() {
    cli().debug_assert()
}

fn config_configure(config: &mut cargo::Config, subcommand_args: &ArgMatches) -> CargoResult<()> {
    let arg_target_dir = &None;
    let verbose = 0;
    // quiet is unusual because it is redefined in some subcommands in order
    // to provide custom help text.
    let quiet = subcommand_args.contains_id("quiet");
    let color = None;
    let frozen = false;
    let locked = false;
    let offline = subcommand_args.contains_id("offline");
    let mut unstable_flags = vec![];
    if let Some(values) = subcommand_args.get_many::<String>("unstable-features") {
        unstable_flags.extend(values.map(|s| s.to_string()));
    }
    let config_args = [];
    config.configure(
        verbose,
        quiet,
        color,
        frozen,
        locked,
        offline,
        arg_target_dir,
        &unstable_flags,
        &config_args,
    )?;
    Ok(())
}
