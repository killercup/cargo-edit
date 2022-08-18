use cargo::{core::dependency::DepKind, util::command_prelude::*};
use cargo_rm::ops::cargo_rm::{rm, DepTable, RmOptions};

pub fn cli() -> clap::Command<'static> {
    clap::Command::new("rm")
        .setting(clap::AppSettings::DeriveDisplayOrder)
        .about("Remove dependencies from a Cargo.toml manifest file")
        .args([
            clap::Arg::new("dependencies")
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
                .value_name("PKG_ID"),
            clap::Arg::new("offline")
                .long("offline")
                .help("Run without accessing the network"),
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
    let dry_run = args.dry_run();
    let section = parse_section(args);

    let ws = args.workspace(config)?;
    let packages = args.packages_from_flags()?;
    let packages = packages.get_packages(&ws)?;
    let spec = match packages.len() {
        0 => {
            return Err(CliError::new(
                anyhow::format_err!("no packages selected.  Please specify one with `-p <PKGID>`"),
                101,
            ));
        }
        1 => packages[0],
        len => {
            return Err(CliError::new(
                anyhow::format_err!(
                    "{len} packages selected.  Please specify one with `-p <PKGID>`",
                ),
                101,
            ));
        }
    };

    let dependencies = args.get_many::<String>("dependencies").unwrap().collect();

    let options = RmOptions {
        config,
        spec,
        dependencies,
        section,
        dry_run,
    };
    rm(&options)?;

    Ok(())
}

fn parse_section(matches: &ArgMatches) -> DepTable {
    let kind = if matches.contains_id("dev") {
        DepKind::Development
    } else if matches.contains_id("build") {
        DepKind::Build
    } else {
        DepKind::Normal
    };

    let mut table = DepTable::new().set_kind(kind);

    if let Some(target) = matches.get_one::<String>("target") {
        assert!(!target.is_empty(), "Target specification may not be empty");
        table = table.set_target(target);
    }

    table
}
