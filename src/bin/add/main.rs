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

#[macro_use]
extern crate error_chain;

use crate::args::{Args, Command, UnstableOptions};
use cargo_edit::{
    colorize_stderr, find, manifest_from_pkgid, registry_url, update_registry_index, Dependency,
    LocalManifest,
};
use clap::Parser;
use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;
use std::process;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use toml_edit::Item as TomlItem;

mod args;

mod errors {
    error_chain! {
        errors {
            /// Specified a dependency with both a git URL and a version.
            GitUrlWithVersion(git: String, version: String) {
                description("Specified git URL with version")
                display("Cannot specify a git URL (`{}`) with a version (`{}`).", git, version)
            }
            /// Specified multiple crates with path or git or vers
            MultipleCratesWithGitOrPathOrVers {
                description("Specified multiple crates with path or git or vers")
                display("Cannot specify multiple crates with path or git or vers")
            }
            /// Specified multiple crates with renaming.
            MultipleCratesWithRename {
                description("Specified multiple crates with rename")
                display("Cannot specify multiple crates with rename")
            }
            /// Specified multiple crates with features.
            MultipleCratesWithFeatures {
                description("Specified multiple crates with features")
                display("Cannot specify multiple crates with features")
            }
            AddingSelf(crate_: String) {
                description("Adding crate to itself")
                display("Cannot add `{}` as a dependency to itself", crate_)
            }
        }
        links {
            CargoEditLib(::cargo_edit::Error, ::cargo_edit::ErrorKind);
        }
        foreign_links {
            CargoMetadata(::cargo_metadata::Error)#[doc = "An error from the cargo_metadata crate"];
            Io(::std::io::Error);
        }
    }
}

use crate::errors::*;

fn print_msg(dep: &Dependency, section: &[String], optional: bool) -> Result<()> {
    let colorchoice = colorize_stderr();
    let mut output = StandardStream::stderr(colorchoice);
    output.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    write!(output, "{:>12}", "Adding")?;
    output.reset()?;
    write!(output, " {}", dep.name)?;
    if dep.path().is_some() {
        write!(output, " (local)")?;
    } else if let Some(version) = dep.version() {
        if version.chars().next().unwrap_or('0').is_ascii_digit() {
            write!(output, " v{}", version)?;
        } else {
            write!(output, " {}", version)?;
        }
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
        if !f.is_empty() {
            write!(output, " with features: {}", f.join(", "))?;
        }
    }
    writeln!(output, ".")?;

    if !&dep.available_features.is_empty() {
        writeln!(output, "{:>13}Available features:", " ")?;
        for feat in dep.available_features.iter() {
            writeln!(output, "{:>13}- {}", " ", feat)?;
        }
    }
    Ok(())
}

// Based on Iterator::is_sorted from nightly std; remove in favor of that when stabilized.
fn is_sorted(mut it: impl Iterator<Item = impl PartialOrd>) -> bool {
    let mut last = match it.next() {
        Some(e) => e,
        None => return true,
    };

    for curr in it {
        if curr < last {
            return false;
        }
        last = curr;
    }

    true
}

fn unrecognized_features_message(message: &str) -> Result<()> {
    let colorchoice = colorize_stderr();
    let mut output = StandardStream::stderr(colorchoice);
    output.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
    write!(output, "{:>12}", "Warning:")?;
    output.reset()?;
    writeln!(output, " {}", message)
        .chain_err(|| "Failed to write unrecognized features message")?;
    Ok(())
}

fn handle_add(mut args: Args) -> Result<()> {
    if args.git.is_some() && !args.unstable_features.contains(&UnstableOptions::Git) {
        return Err("`--git` is unstable and requires `-Z git`".into());
    }

    if let Some(ref pkgid) = args.pkgid {
        let pkg = manifest_from_pkgid(args.manifest_path.as_deref(), pkgid)?;
        args.manifest_path = Some(pkg.manifest_path.into_std_path_buf());
    }
    let mut manifest = LocalManifest::find(&args.manifest_path)?;

    if !args.offline && std::env::var("CARGO_IS_TEST").is_err() {
        let url = registry_url(
            &find(&args.manifest_path)?,
            args.registry.as_ref().map(String::as_ref),
        )?;
        update_registry_index(&url, args.quiet)?;
    }
    let requested_features: Option<BTreeSet<&str>> = args.features.as_ref().map(|v| {
        v.iter()
            .flat_map(|s| s.split(' '))
            .flat_map(|s| s.split(','))
            .filter(|s| !s.is_empty())
            .collect()
    });

    let deps = &args.parse_dependencies(
        requested_features
            .as_ref()
            .map(|s| s.iter().map(|s| s.to_string()).collect()),
    )?;

    if let Some(req_feats) = requested_features {
        assert!(deps.len() == 1);
        let available_features = deps[0]
            .available_features
            .iter()
            .map(|s| s.as_ref())
            .collect::<BTreeSet<&str>>();

        let mut unknown_features: Vec<&&str> = req_feats.difference(&available_features).collect();
        unknown_features.sort();

        if !unknown_features.is_empty() {
            unrecognized_features_message(&format!(
                "Unrecognized features: {:?}",
                unknown_features
            ))?;
        };
    };

    let was_sorted = manifest
        .get_table(&args.get_section())
        .map(TomlItem::as_table)
        .map_or(true, |table_option| {
            table_option.map_or(true, |table| is_sorted(table.iter().map(|(name, _)| name)))
        });
    deps.iter()
        .map(|dep| {
            if !args.quiet {
                print_msg(dep, &args.get_section(), args.optional)?;
            }
            if let Some(path) = dep.path() {
                if path == manifest.path.parent().unwrap_or_else(|| Path::new("")) {
                    return Err(ErrorKind::AddingSelf(manifest.package_name()?.to_owned()).into());
                }
            }
            manifest.insert_into_table(&args.get_section(), dep)?;
            manifest.gc_dep(dep.toml_key());
            Ok(())
        })
        .collect::<Result<Vec<_>>>()
        .map_err(|err| {
            eprintln!("Could not edit `Cargo.toml`.\n\nERROR: {}", err);
            err
        })?;

    if was_sorted {
        if let Some(table) = manifest
            .get_table_mut(&args.get_section())
            .ok()
            .and_then(TomlItem::as_table_like_mut)
        {
            table.sort_values();
        }
    }

    manifest.write()?;

    Ok(())
}

fn main() {
    let args: Command = Command::parse();
    let Command::Add(args) = args;

    if let Err(err) = handle_add(args) {
        eprintln!("Command failed due to unhandled error: {}", err);

        let mut gap = false;
        for e in err.iter().skip(1) {
            if !gap {
                eprintln!();
                gap = true;
            }
            eprintln!("Caused by: {}", e);
        }

        if let Some(backtrace) = err.backtrace() {
            eprintln!();
            eprintln!("Backtrace: {:?}", backtrace);
        }

        process::exit(1);
    }
}
