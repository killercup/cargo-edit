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

use crate::args::{Args, Command};
use cargo_edit::{
    find, manifest_from_pkgid, registry_url, update_registry_index, Dependency, Manifest,
};
use std::borrow::Cow;
use std::io::Write;
use std::process;
use structopt::StructOpt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use toml_edit::Item as TomlItem;

mod args;

mod errors {
    use thiserror::Error as ThisError;

    #[derive(Debug, ThisError)]
    pub enum Error {
        /// Specified a dependency with both a git URL and a version.
        #[error("Cannot specify a git URL (`{git}`) with a version (`{version}`).")]
        GitUrlWithVersion {
            /// git URL
            git: String,
            /// Version
            version: String,
        },
        /// Specified multiple crates with path or git or vers
        #[error("Cannot specify multiple crates with path or git or vers")]
        MultipleCratesWithGitOrPathOrVers,
        /// Specified multiple crates with renaming.
        #[error("Cannot specify multiple crates with rename")]
        MultipleCratesWithRename,

        /// An error originating from the cargo-edit library
        #[error(transparent)]
        CargoEditLib(#[from] cargo_edit::Error),

        /// An IO error
        #[error(transparent)]
        Io(#[from] std::io::Error),

        /// An erorr from the semver crate
        #[error(transparent)]
        SemVerParse(#[from] semver::ReqParseError),

        /// An ad-hoc error
        #[error("{0}")]
        Custom(String),

        /// An error with it's source
        #[error("{error}")]
        Wrapped {
            error: Box<Error>,
            source: Box<Error>,
        },
    }

    impl Error {
        pub fn wrap<T, U>(error: T, source: U) -> Error
        where
            T: Into<Error>,
            U: Into<Error>,
        {
            Error::Wrapped {
                error: Box::new(error.into()),
                source: Box::new(source.into()),
            }
            /// Specified multiple crates with features.
            MultipleCratesWithFeatures {
                description("Specified multiple crates with features")
                display("Cannot specify multiple crates with features")
            }
        }
    }

    impl<'a> From<&'a str> for Error {
        fn from(s: &'a str) -> Error {
            Error::Custom(s.into())
        }
    }

    impl From<String> for Error {
        fn from(s: String) -> Error {
            Error::Custom(s)
        }
    }

    pub type Result<T> = std::result::Result<T, Error>;
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
    write!(output, " {}", section)?;
    if let Some(f) = &dep.features {
        writeln!(output, " with features: {:?}", f)?
    } else {
        writeln!(output)?
    }
    Ok(())
}

fn handle_add(args: &Args) -> Result<()> {
    let manifest_path = if let Some(ref pkgid) = args.pkgid {
        let pkg = manifest_from_pkgid(pkgid)?;
        Cow::Owned(Some(pkg.manifest_path))
    } else {
        Cow::Borrowed(&args.manifest_path)
    };
    let mut manifest = Manifest::open(&manifest_path)?;
    let deps = &args.parse_dependencies()?;

    if !args.offline && std::env::var("CARGO_IS_TEST").is_err() {
        let url = registry_url(
            &find(&manifest_path)?,
            args.registry.as_ref().map(String::as_ref),
        )?;
        update_registry_index(&url)?;
    }

    deps.iter()
        .map(|dep| {
            if !args.quiet {
                print_msg(dep, &args.get_section(), args.optional)?;
            }
            manifest
                .insert_into_table(&args.get_section(), dep)
                .map(|_| {
                    manifest
                        .get_table(&args.get_section())
                        .map(TomlItem::as_table_mut)
                        .map(|table_option| {
                            table_option.map(|table| {
                                if args.sort {
                                    table.sort_values();
                                }
                            })
                        })
                })
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
    use failure::Fail;
    use std::error::Error;

    let args: Command = Command::from_args();
    let Command::Add(args) = args;

    if let Err(err) = handle_add(&args) {
        eprintln!("Command failed due to unhandled error: {}\n", err);

        let mut sources: &dyn Error = &err;
        while let Some(source) = sources.source() {
            eprintln!("Caused by: {}", source);
            sources = source;
        }

        // this should change to use std::backtrace::Backtrace when that is stabilized
        if let Some(backtrace) = Fail::backtrace(&err) {
            eprintln!("Backtrace: {:?}", backtrace);
        }

        process::exit(1);
    }
}
