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

mod cli;
mod commands;

use std::process;

fn main() {
    let mut config = cargo::Config::default().unwrap();
    if let Err(err) = cli::main(&mut config) {
        eprintln!("Error: {:?}", err);

        process::exit(1);
    }
}
