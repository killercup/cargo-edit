use cargo::util::command_prelude::*;
use cargo_rm::ops::cargo_rm::{RmOptions, rm};

pub fn cli() -> clap::Command<'static> {
    clap::Command::new("rm")
        .setting(clap::AppSettings::DeriveDisplayOrder)
        .about("Remove dependencies from a Cargo.toml manifest file")
        .args([
            clap::Arg::new("crates")
                .takes_value(true)
                .value_name("DEP_ID")
                .action(clap::ArgAction::Append)
                .multiple_values(true)
                .help("Reference to a package to remove as a dependency")
                .required(true),
            clap::Arg::new("pkg_id")
                .short('p')
                .long("package")
                .help("Package ID of the crate to remove this dependency from")
                .takes_value(true)
                .value_name("PKG_ID")
        ])
        .arg_manifest_path()
        .arg_quiet()
        .arg_dry_run("Don't actually write the manifest")
        .next_help_heading("SECTION")
        .args([
            clap::Arg::new("dev")
                .long("dev")
                .help("Remove as development dependency")
                .conflicts_with("build")
                .group("section"),
            clap::Arg::new("build")
                .long("build")
                .help("Remove as build dependency")
                .conflicts_with("dev")
                .group("section"),
            clap::Arg::new("target")
                .long("target")
                .help("Remove as dependency from the given target platform")
                .takes_value(true)
                .value_parser(clap::builder::NonEmptyStringValueParser::new()),
        ])
}

pub fn exec(config: &mut Config, args: &ArgMatches) -> CliResult {
    // TODO parse options

    let options = RmOptions {
        config,
    };
    rm(&options)?;

    Ok(())
}
