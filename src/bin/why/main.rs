//! `cargo wny`
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

use std::path::PathBuf;
use std::process;
use structopt::StructOpt;

mod errors {
    error_chain! {
        links {
            CargoEditLib(::cargo_edit::Error, ::cargo_edit::ErrorKind);
        }
    }
}
use crate::errors::*;

#[derive(Debug, StructOpt)]
#[structopt(bin_name = "cargo")]
enum Command {
    /// Find out why specified dependency required.
    #[structopt(name = "why")]
    Why(Args),
}

#[derive(Debug, StructOpt)]
struct Args {
    /// Crates to be resolved.
    #[structopt(name = "crates", required = true)]
    crates: Vec<String>,

    /// Path to the manifest where crate should be resolved.
    #[structopt(long = "manifest-path", value_name = "path")]
    manifest_path: Option<PathBuf>,
}

fn handle_why(args: &Args) -> Result<()> {
    let config = cargo::util::config::Config::default().unwrap();
    let path = cargo_edit::find(&args.manifest_path)?;
    let ws = cargo::core::Workspace::new(path.as_path(), &config).unwrap();
    let lock = cargo::ops::load_pkg_lockfile(&ws).unwrap().unwrap();

    for crate_name in args.crates.iter() {
        for pkgid in lock.iter().filter(|p| p.name() == crate_name.as_str()) {
            println!("{:?}", pkgid);
            let path = lock.path_to_top(&pkgid);
            for pkgid in path.iter() {
                println!("  {:?}", pkgid);
            }
        }
    }

    Ok(())
}

fn main() {
    let args: Command = Command::from_args();
    let Command::Why(args) = args;

    if let Err(err) = handle_why(&args) {
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
