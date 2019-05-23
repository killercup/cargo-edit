//! `cargo rm`
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

use cargo_edit::Manifest;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use structopt::StructOpt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

mod errors {
    error_chain! {
        links {
            CargoEditLib(::cargo_edit::Error, ::cargo_edit::ErrorKind);
        }
        foreign_links {
            Io(::std::io::Error);
        }
    }
}
use crate::errors::*;

#[derive(Debug, StructOpt)]
#[structopt(bin_name = "cargo")]
enum ArgsWrap {
    #[structopt(name = "rm", author = "")]
    /// Remove a dependency from a Cargo.toml manifest file.
    Rm(Args),
}

#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(name = "crates", raw(required = "true"))]
    /// Crates to be removed
    crates: Vec<String>,

    #[structopt(long = "dev", short = "D", conflicts_with = "build")]
    /// Remove crate as development dependency.
    dev: bool,

    #[structopt(long = "build", short = "B", conflicts_with = "dev")]
    /// Remove crate as build dependency.
    build: bool,

    #[structopt(long = "manifest-path", value_name = "path")]
    /// Path to the manifest to remove a dependency from.
    manifest_path: Option<PathBuf>,

    #[structopt(long = "quiet", short = "q")]
    /// Do not print any output in case of success.
    quiet: bool,
}

impl Args {
    /// Get depenency section
    pub fn get_section(&self) -> &'static str {
        if self.dev {
            "dev-dependencies"
        } else if self.build {
            "build-dependencies"
        } else {
            "dependencies"
        }
    }
}

fn print_msg(name: &str, section: &str) -> Result<()> {
    let colorchoice = if atty::is(atty::Stream::Stdout) {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };
    let mut output = StandardStream::stdout(colorchoice);
    output.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    write!(output, "{:>12}", "Removing")?;
    output.reset()?;
    writeln!(output, " {} from {}", name, section)?;
    Ok(())
}

fn handle_rm(args: &Args) -> Result<()> {
    let manifest_path = &args.manifest_path;
    let mut manifest = Manifest::open(manifest_path)?;
    let deps = &args.crates;

    deps.iter()
        .map(|dep| {
            if !args.quiet {
                print_msg(&dep, args.get_section())?;
            }
            manifest
                .remove_from_table(args.get_section(), dep)
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
    let args: ArgsWrap = ArgsWrap::from_args();
    let ArgsWrap::Rm(args) = args;

    if let Err(err) = handle_rm(&args) {
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
