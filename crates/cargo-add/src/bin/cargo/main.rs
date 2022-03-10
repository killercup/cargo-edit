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

fn main() {
    let mut config = cargo::Config::default().unwrap();
    let result = cli::main(&mut config);
    match result {
        Err(e) => cargo::exit_with_error(e, &mut *config.shell()),
        Ok(()) => {}
    }
}
