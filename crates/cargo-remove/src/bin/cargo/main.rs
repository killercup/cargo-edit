//! `cargo remove`
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

fn main() {
    let mut config = cargo::Config::default().unwrap();
    if let Err(err) = cli::main(&mut config) {
        cargo::exit_with_error(err, &mut config.shell());
    }
}
