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
// if the user has compiled with the `backtrace` feature, enable the stdlib `backtrace` feature
#![cfg_attr(feature = "backtrace", feature(backtrace))]

use anyhow::Result;
use cargo_edit::{manifest_from_pkgid, Manifest};
use std::borrow::Cow;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use structopt::StructOpt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

mod errors {
    use thiserror::Error as ThisError;

    #[derive(Debug, ThisError)]
    pub enum Error {
        /// An error originating from the cargo-edit library
        #[error(transparent)]
        CargoEditLib(#[from] cargo_edit::Error),

        /// An IO error
        #[error(transparent)]
        Io(#[from] std::io::Error),
    }
}

#[derive(Debug, StructOpt)]
#[structopt(bin_name = "cargo")]
enum Command {
    /// Remove a dependency from a Cargo.toml manifest file.
    #[structopt(name = "rm")]
    Rm(Args),
}

#[derive(Debug, StructOpt)]
struct Args {
    /// Crates to be removed.
    #[structopt(name = "crates", required = true)]
    crates: Vec<String>,

    /// Remove crate as development dependency.
    #[structopt(long = "dev", short = "D", conflicts_with = "build")]
    dev: bool,

    /// Remove crate as build dependency.
    #[structopt(long = "build", short = "B", conflicts_with = "dev")]
    build: bool,

    /// Path to the manifest to remove a dependency from.
    #[structopt(long = "manifest-path", value_name = "path", conflicts_with = "pkgid")]
    manifest_path: Option<PathBuf>,

    /// Package id of the crate to add this dependency to.
    #[structopt(
        long = "package",
        short = "p",
        value_name = "pkgid",
        conflicts_with = "path"
    )]
    pkgid: Option<String>,

    /// Do not print any output in case of success.
    #[structopt(long = "quiet", short = "q")]
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
    let manifest_path = if let Some(ref pkgid) = args.pkgid {
        let pkg = manifest_from_pkgid(pkgid)?;
        Cow::Owned(Some(pkg.manifest_path))
    } else {
        Cow::Borrowed(&args.manifest_path)
    };
    let mut manifest = Manifest::open(&manifest_path)?;
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

    let mut file = Manifest::find_file(&manifest_path)?;
    manifest.write_to_file(&mut file)?;

    Ok(())
}

fn main() {
    let args: Command = Command::from_args();
    let Command::Rm(args) = args;

    if let Err(err) = handle_rm(&args) {
        eprintln!("Command failed due to unhandled error: {}\n", err);

        for source in err.chain().skip(1) {
            eprintln!("Caused by: {}", source);
        }

        #[cfg(feature = "backtrace")]
        {
            eprintln!("Backtrace: {:?}", err.backtrace());
        }

        process::exit(1);
    }
}
