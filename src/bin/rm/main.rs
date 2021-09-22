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

use cargo_edit::{manifest_from_pkgid, LocalManifest};
use std::borrow::Cow;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use structopt::{clap::AppSettings, StructOpt};
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
enum Command {
    /// Remove a dependency from a Cargo.toml manifest file.
    #[structopt(name = "rm")]
    Rm(Args),
}

#[derive(Debug, StructOpt)]
#[structopt(setting = AppSettings::ColoredHelp)]
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

    /// Package id of the crate to remove this dependency from.
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
            if !dep_used(&manifest, dep) {
                if let Ok(toml_edit::Item::Table(feature_table)) =
                    manifest.get_table(&["features".to_string()])
                {
                    for (_feature, mut activated_crates) in feature_table.iter_mut() {
                        if let toml_edit::Item::Value(toml_edit::Value::Array(
                            feature_activations,
                        )) = &mut activated_crates
                        {
                            remove_feature_activation(feature_activations, dep);
                        }
                    }
                }
            }

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

/// Is there a crate with this name as any kind of dependency?
/// (maybe optional, maybe target specific).
fn dep_used(manifest: &LocalManifest, dep: &str) -> bool {
    for (_, tbl) in manifest.get_sections() {
        if let toml_edit::Item::Table(tbl) = tbl {
            if tbl.contains_key(dep) {
                return true;
            }
        }
    }
    false
}

fn remove_feature_activation(feature_activations: &mut toml_edit::Array, dep: &str) {
    let dep_feature: &str = &format!("{}/", dep);

    let remove_list: Vec<usize> = feature_activations
        .iter()
        .enumerate()
        .filter_map(|(idx, feature_activation)| {
            if let toml_edit::Value::String(feature_activation) = feature_activation {
                let activation = feature_activation.value();
                (activation == dep || activation.starts_with(dep_feature)).then(|| idx)
            } else {
                None
            }
        })
        .collect();

    // Remove found idx in revers order so we don't invalidate the idx.
    for idx in remove_list.iter().rev() {
        feature_activations.remove(*idx);
    }
}

fn main() {
    let args: Command = Command::from_args();
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
