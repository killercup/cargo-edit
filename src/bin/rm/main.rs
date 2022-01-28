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

use cargo_edit::{colorize_stderr, manifest_from_pkgid, LocalManifest};
use clap::Parser;
use std::borrow::Cow;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

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

#[derive(Debug, Parser)]
#[clap(bin_name = "cargo")]
enum Command {
    /// Remove a dependency from a Cargo.toml manifest file.
    #[clap(name = "rm")]
    Rm(Args),
}

#[derive(Debug, Parser)]
#[clap(about, version)]
struct Args {
    /// Crates to be removed.
    #[clap(value_name = "CRATE", required = true)]
    crates: Vec<String>,

    /// Remove crate as development dependency.
    #[clap(long, short = 'D', conflicts_with = "build")]
    dev: bool,

    /// Remove crate as build dependency.
    #[clap(long, short = 'B', conflicts_with = "dev")]
    build: bool,

    /// Path to the manifest to remove a dependency from.
    #[clap(
        long,
        value_name = "PATH",
        parse(from_os_str),
        conflicts_with = "pkgid"
    )]
    manifest_path: Option<PathBuf>,

    /// Package id of the crate to remove this dependency from.
    #[clap(
        long = "package",
        short = 'p',
        value_name = "PKGID",
        conflicts_with = "manifest-path"
    )]
    pkgid: Option<String>,

    /// Do not print any output in case of success.
    #[clap(long, short)]
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
    let colorchoice = colorize_stderr();
    let mut output = StandardStream::stderr(colorchoice);
    output.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    write!(output, "{:>12}", "Removing")?;
    output.reset()?;
    writeln!(output, " {} from {}", name, section)?;
    Ok(())
}

fn handle_rm(args: &Args) -> Result<()> {
    let manifest_path = if let Some(ref pkgid) = args.pkgid {
        let pkg = manifest_from_pkgid(args.manifest_path.as_deref(), pkgid)?;
        Cow::Owned(Some(pkg.manifest_path.into_std_path_buf()))
    } else {
        Cow::Borrowed(&args.manifest_path)
    };
    let mut manifest = LocalManifest::find(&manifest_path)?;
    let deps = &args.crates;

    deps.iter()
        .map(|dep| {
            if !args.quiet {
                print_msg(dep, args.get_section())?;
            }
            let result = manifest
                .remove_from_table(args.get_section(), dep)
                .map_err(Into::into);

            // Now that we have removed the crate, if that was the last reference to that crate,
            // then we need to drop any explicitly activated features on that crate.
            manifest.gc_dep(dep);

            result
        })
        .collect::<Result<Vec<_>>>()
        .map_err(|err| {
            eprintln!("Could not edit `Cargo.toml`.\n\nERROR: {}", err);
            err
        })?;

    manifest.write()?;

    Ok(())
}

fn main() {
    let args: Command = Command::parse();
    let Command::Rm(args) = args;

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

#[test]
fn verify_app() {
    use clap::IntoApp;
    Command::into_app().debug_assert()
}
