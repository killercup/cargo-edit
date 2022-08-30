use cargo::util::command_prelude::*;

use cargo_remove::ops::cargo_remove::remove;
use cargo_remove::ops::cargo_remove::RmOptions;

pub fn cli() -> clap::Command<'static> {
    clap::Command::new("remove")
        .setting(clap::AppSettings::DeriveDisplayOrder)
        .about("Remove dependencies from a Cargo.toml manifest file")
        .args([
            clap::Arg::new("dependencies")
                .action(clap::ArgAction::Append)
                .required(true)
                .multiple_values(true)
                .takes_value(true)
                .value_name("DEP_ID")
                .help("Dependencies to be removed"),
            clap::Arg::new("manifest_path")
                .long("manifest-path")
                .takes_value(true)
                .value_name("PATH")
                .value_parser(clap::builder::PathBufValueParser::new())
                .help("Path to the manifest to remove a dependency from"),
            clap::Arg::new("pkg_id")
                .short('p')
                .long("package")
                .takes_value(true)
                .value_name("PKG_ID")
                .help("Package to remove from"),
            clap::Arg::new("dry_run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Don't actually write the manifest"),
            clap::Arg::new("quiet")
                .short('q')
                .long("quiet")
                .action(clap::ArgAction::SetTrue)
                .help("Do not print any output in case of success"),
        ])
        .next_help_heading("SECTION")
        .args([
            clap::Arg::new("dev")
                .long("dev")
                .conflicts_with("build")
                .action(clap::ArgAction::SetTrue)
                .group("section")
                .help("Remove as development dependency"),
            clap::Arg::new("build")
                .long("build")
                .conflicts_with("dev")
                .action(clap::ArgAction::SetTrue)
                .group("section")
                .help("Remove as build dependency"),
            clap::Arg::new("target")
                .long("target")
                .takes_value(true)
                .value_name("TARGET")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .help("Remove as dependency from the given target platform"),
        ])
}

pub fn exec(_config: &mut Config, args: &ArgMatches) -> CliResult {
    let crates = args
        .get_many("dependencies")
        .expect("required(true)")
        .cloned()
        .collect();
    let dev = args
        .get_one::<bool>("dev")
        .copied()
        .expect("action(ArgAction::SetTrue)");
    let build = args
        .get_one::<bool>("build")
        .copied()
        .expect("action(ArgAction::SetTrue)");
    let target = args.get_one("target").cloned();
    let manifest_path = args.get_one("manifest_path").cloned();
    let pkg_id = args.get_one("pkg_id").cloned();
    let dry_run = args
        .get_one::<bool>("dry_run")
        .copied()
        .expect("action(ArgAction::SetTrue)");
    let quiet = args
        .get_one::<bool>("quiet")
        .copied()
        .expect("action(ArgAction::SetTrue)");

    let options = RmOptions {
        crates,
        dev,
        build,
        target,
        manifest_path,
        pkg_id,
        dry_run,
        quiet,
    };

    remove(&options)?;

    Ok(())
}
