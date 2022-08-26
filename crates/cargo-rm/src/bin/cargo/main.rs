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

mod cli;
mod commands;

use std::process;

use clap::Parser;

fn main() {
    let args = cli::Cli::parse();

    if let Err(err) = args.exec() {
        eprintln!("Error: {:?}", err);

        process::exit(1);
    }
}
