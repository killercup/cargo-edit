use cargo::util::command_prelude::*;
use cargo::CargoResult;
use cargo_rm::shell_status;
use cargo_rm::shell_warn;
use cargo_rm::{manifest_from_pkgid, LocalManifest};

use std::borrow::Cow;
use std::path::PathBuf;

/// Remove a dependency from a Cargo.toml manifest file.
#[derive(Debug)]
pub struct RmOptions {
    /// Dependencies to be removed
    crates: Vec<String>,
    /// Remove as development dependency
    dev: bool,
    /// Remove as build dependency
    build: bool,
    /// Remove as dependency from the given target platform
    target: Option<String>,
    /// Path to the manifest to remove a dependency from
    manifest_path: Option<PathBuf>,
    /// Package to remove from
    pkg_id: Option<String>,
    /// Don't actually write the manifest
    dry_run: bool,
    /// Do not print any output in case of success
    quiet: bool,
}

impl RmOptions {
    /// Get dependency section
    pub fn get_section(&self) -> Vec<String> {
        let section_name = if self.dev {
            "dev-dependencies"
        } else if self.build {
            "build-dependencies"
        } else {
            "dependencies"
        };

        if let Some(ref target) = self.target {
            assert!(!target.is_empty(), "Target specification may not be empty");

            vec!["target".to_owned(), target.clone(), section_name.to_owned()]
        } else {
            vec![section_name.to_owned()]
        }
    }
}

pub fn cli() -> clap::Command<'static> {
    clap::Command::new("rm")
        .setting(clap::AppSettings::DeriveDisplayOrder)
        .about("Remove a dependency from a Cargo.toml manifest file")
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
                .value_name("PKGID")
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
                .short('D')
                .long("dev")
                .conflicts_with("build")
                .action(clap::ArgAction::SetTrue)
                .group("section")
                .help("Remove as development dependency"),
            clap::Arg::new("build")
                .short('B')
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum UnstableOptions {}

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

    rm(&options)?;

    Ok(())
}

fn rm(args: &RmOptions) -> CargoResult<()> {
    let manifest_path = if let Some(ref pkg_id) = args.pkg_id {
        let pkg = manifest_from_pkgid(args.manifest_path.as_deref(), pkg_id)?;
        Cow::Owned(Some(pkg.manifest_path.into_std_path_buf()))
    } else {
        Cow::Borrowed(&args.manifest_path)
    };
    let mut manifest = LocalManifest::find(manifest_path.as_deref())?;
    let deps = &args.crates;

    deps.iter()
        .map(|dep| {
            if !args.quiet {
                let section = args.get_section();
                let section = if section.len() >= 3 {
                    format!("{} for target `{}`", &section[2], &section[1])
                } else {
                    section[0].clone()
                };
                shell_status("Removing", &format!("{dep} from {section}",))?;
            }
            let result = manifest
                .remove_from_table(&args.get_section(), dep)
                .map_err(Into::into);

            // Now that we have removed the crate, if that was the last reference to that crate,
            // then we need to drop any explicitly activated features on that crate.
            manifest.gc_dep(dep);

            result
        })
        .collect::<CargoResult<Vec<_>>>()?;

    if args.dry_run {
        shell_warn("aborting rm due to dry run")?;
    } else {
        manifest.write()?;
    }

    Ok(())
}
