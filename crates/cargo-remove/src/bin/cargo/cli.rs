use cargo::util::command_prelude::*;
use cargo::CargoResult;
use clap::Command;

pub fn main(config: &mut Config) -> CliResult {
    let args = cli().try_get_matches()?;
    let (cmd, subcommand_args) = args.subcommand().expect("subcommand_required(true)");
    execute_subcommand(config, cmd, &args, subcommand_args)?;
    Ok(())
}

fn cli() -> Command<'static> {
    Command::new("cargo")
        .bin_name("cargo")
        .arg(
            opt(
                "verbose",
                "Use verbose output (-vv very verbose/build.rs output)",
            )
            .short('v')
            .action(ArgAction::Count)
            .global(true),
        )
        .arg_quiet()
        .arg(flag("offline", "Run without accessing the network").global(true))
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
    config: &mut Config,
    cmd: &str,
    args: &ArgMatches,
    subcommand_args: &ArgMatches,
) -> CliResult {
    config_configure(config, args, subcommand_args)?;
    let exec = crate::commands::builtin_exec(cmd).expect("all of `builtin` supported");
    exec(config, subcommand_args)
}

fn config_configure(
    config: &mut Config,
    args: &ArgMatches,
    subcommand_args: &ArgMatches,
) -> CargoResult<()> {
    let arg_target_dir = &None;
    let verbose = args.verbose();
    // quiet is unusual because it is redefined in some subcommands in order
    // to provide custom help text.
    let quiet = args.flag("quiet") || subcommand_args.flag("quiet");
    let color = None;
    let frozen = false;
    let locked = false;
    let offline = subcommand_args.flag("offline");
    let mut unstable_flags = vec![];
    if let Some(values) = args.get_many::<String>("unstable-features") {
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

#[test]
fn verify_app() {
    cli().debug_assert()
}
