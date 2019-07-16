//! `cargo add`
#![warn(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

#[macro_use]
extern crate error_chain;

use crate::args::{Args, Command};
use cargo_edit::{Dependency, Manifest};
use std::io::Write;
use std::process;
use structopt::StructOpt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

mod args;

mod errors {
    error_chain! {
        errors {
            /// Specified a dependency with both a git URL and a version.
            GitUrlWithVersion(git: String, version: String) {
                description("Specified git URL with version")
                display("Cannot specify a git URL (`{}`) with a version (`{}`).", git, version)
            }
            /// Specified multiple crates with path or git or vers
            MultipleCratesWithGitOrPathOrVers{
                description("Specified multiple crates with path or git or vers")
                display("Cannot specify multiple crates with path or git or vers")
            }
        }
        links {
            CargoEditLib(::cargo_edit::Error, ::cargo_edit::ErrorKind);
        }
        foreign_links {
            Io(::std::io::Error);
        }
    }
}
use crate::errors::*;

fn print_msg(dep: &Dependency, section: &[String], optional: bool) -> Result<()> {
    let colorchoice = if atty::is(atty::Stream::Stdout) {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };
    let mut output = StandardStream::stdout(colorchoice);
    output.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    write!(output, "{:>12}", "Adding")?;
    output.reset()?;
    write!(output, " {}", dep.name)?;
    if let Some(version) = dep.version() {
        write!(output, " v{}", version)?;
    } else {
        write!(output, " (unknown version)")?;
    }
    write!(output, " to")?;
    if optional {
        write!(output, " optional")?;
    }
    let section = if section.len() == 1 {
        section[0].clone()
    } else {
        format!("{} for target `{}`", &section[2], &section[1])
    };
    writeln!(output, " {}", section)?;
    Ok(())
}

fn handle_add(args: &Args) -> Result<()> {
    let manifest_path = &args.manifest_path;
    let mut manifest = Manifest::open(manifest_path)?;
    let deps = &args.parse_dependencies(args.offline)?;

    deps.iter()
        .map(|dep| {
            if !args.quiet {
                print_msg(dep, &args.get_section(), args.optional)?;
            }
            manifest
                .insert_into_table(&args.get_section(), dep)
                .map_err(Into::into)
        })
        .collect::<Result<Vec<_>>>()
        .map_err(|err| {
            eprintln!("Could not edit `Cargo.toml`.\n\nERROR: {}", err);
            err
        })?;

    let mut file = Manifest::find_file(manifest_path)?;
    manifest.write_to_file(&mut file)?;

    Ok(())
}

fn main() {
    let args: Command = Command::from_args();
    let Command::Add(args) = args;

    if let Err(err) = handle_add(&args) {
        eprintln!("Command failed due to unhandled error: {}\n", err);

        for e in err.iter().skip(1) {
            eprintln!("Caused by: {}", e);
        }

        if let Some(backtrace) = err.backtrace() {
            eprintln!("Backtrace: {:?}", backtrace);
        }

        process::exit(1);
    }
}
