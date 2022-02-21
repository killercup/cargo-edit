#![allow(clippy::bool_assert_comparison)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use cargo::util::command_prelude::*;
use cargo_add::ops::cargo_add::CargoResult;
use cargo_add::ops::cargo_add::Context;
use cargo_add::ops::cargo_add::Dependency;
use cargo_add::ops::cargo_add::{
    colorize_stderr, registry_url, update_registry_index, LocalManifest,
};
use cargo_add::ops::cargo_add::{
    get_features_from_registry, get_manifest_from_path, get_manifest_from_url,
};
use cargo_add::ops::cargo_add::{get_latest_dependency, CrateSpec};
use indexmap::IndexSet;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use toml_edit::Item as TomlItem;

pub fn cli() -> clap::Command<'static> {
    clap::Command::new("add")
        .setting(clap::AppSettings::DeriveDisplayOrder)
        .about("Add dependencies to a Cargo.toml manifest file")
        .override_usage(
            "\
    cargo add [OPTIONS] <DEP>[@<VERSION>] [+<FEATURE>,...] ...
    cargo add [OPTIONS] <DEP_PATH> [+<FEATURE>,...] ...",
        )
        .after_help(
            "\
EXAMPLES:
  $ cargo add regex --build
  $ cargo add trycmd --dev
  $ cargo add ./crate/parser/
  $ cargo add serde +derive serde_json
",
        )
        .args([
            clap::Arg::new("crates")
                .takes_value(true)
                .value_name("DEP_ID")
                .multiple_occurrences(true)
                .required(true)
                .help("Reference to a package to add as a dependency")
                .long_help(
                "Reference to a package to add as a dependency

You can reference a packages by:
- `<name>`, like `cargo add serde` (latest version will be used)
- `<name>@<version-req>`, like `cargo add serde@1` or `cargo add serde@=1.0.38`
- `<path>`, like `cargo add ./crates/parser/`

Additionally, you can specify features for a dependency by following it with a `+<FEATURE>`.",
            ),
            clap::Arg::new("no-default-features")
                .long("no-default-features")
                .help("Disable the default features")
                .long_help(None),
            clap::Arg::new("default-features")
                .long("default-features")
                .help("Re-enable the default features")
                .long_help(None)
                .overrides_with("no-default-features"),
            clap::Arg::new("features")
                .short('F')
                .long("features")
                .takes_value(true)
                .value_name("FEATURES")
                .multiple_occurrences(true)
                .help("Space-separated list of features to add")
                .long_help("Space-separated list of features to add

Alternatively, you can specify features for a dependency by following it with a `+<FEATURE>`."),
            clap::Arg::new("optional")
                .long("optional")
                .help("Mark the dependency as optional")
                .long_help("Mark the dependency as optional

The package name will be exposed as feature of your crate.")
                .conflicts_with("dev"),
            clap::Arg::new("no-optional")
                .long("no-optional")
                .help("Mark the dependency as required")
                .long_help("Mark the dependency as required

The package will be removed from your features.")
                .conflicts_with("dev")
                .overrides_with("optional"),
            clap::Arg::new("rename")
                .short('r')
                .long("rename")
                .takes_value(true)
                .value_name("NAME")
                .help("Rename the dependency")
                .long_help("Rename the dependency

Example uses:
- Depending on multiple versions of a crate
- Depend on crates with the same name from different registries"),
            clap::Arg::new("registry")
                .long("registry")
                .takes_value(true)
                .value_name("NAME")
                .help("Package registry for this dependency")
                .long_help(None)
                .conflicts_with("git"),
        ])
        .arg_manifest_path()
        .args([
            clap::Arg::new("package")
                .short('p')
                .long("package")
                .takes_value(true)
                .value_name("SPEC")
                .help("Package to modify")
                .long_help(None),
            clap::Arg::new("offline")
                .long("offline")
                .help("Run without accessing the network")
                .long_help(None),
        ])
        .arg_quiet()
        .next_help_heading("SECTION")
        .args([
            clap::Arg::new("dev")
                .short('D')
                .long("dev")
                .help("Add as development dependency")
                .long_help("Add as development dependency

Dev-dependencies are not used when compiling a package for building, but are used for compiling tests, examples, and benchmarks.

These dependencies are not propagated to other packages which depend on this package.")
                .group("section"),
            clap::Arg::new("build")
                .short('B')
                .long("build")
                .help("Add as build dependency")
                .long_help("Add as build dependency

Build-dependencies are the only dependencies available for use by build scripts (`build.rs` files).")
                .group("section"),
            clap::Arg::new("target")
                .long("target")
                .takes_value(true)
                .value_name("TARGET")
                .forbid_empty_values(true)
                .help("Add as dependency to the given target platform")
                .long_help(None)
                .group("section"),
        ])
        .next_help_heading("UNSTABLE")
        .args([
            clap::Arg::new("unstable-features")
                .short('Z')
                .value_name("FLAG")
                .global(true)
                .takes_value(true)
                .multiple_occurrences(true)
                .possible_values(UnstableOptions::possible_values())
                .help("Unstable (nightly-only) flags")
                .long_help(None),
            clap::Arg::new("git")
                .long("git")
                .takes_value(true)
                .value_name("URI")
                .help("Git repository location")
                .long_help("Git repository location

Without any other information, cargo will use latest commit on the main branch."),
            clap::Arg::new("branch")
                .long("branch")
                .takes_value(true)
                .value_name("BRANCH")
                .help("Git branch to download the crate from")
                .long_help(None)
                .requires("git")
                .group("git-ref"),
            clap::Arg::new("tag")
                .long("tag")
                .takes_value(true)
                .value_name("TAG")
                .help("Git tag to download the crate from")
                .long_help(None)
                .requires("git")
                .group("git-ref"),
            clap::Arg::new("rev")
                .long("rev")
                .takes_value(true)
                .value_name("REV")
                .help("Git reference to download the crate from")
                .long_help("Git reference to download the crate from

This is the catch all, handling hashes to named references in remote repositories.")
                .requires("git")
                .group("git-ref"),
        ])
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ArgEnum)]
pub enum UnstableOptions {
    Git,
    InlineAdd,
}

impl UnstableOptions {
    /// Report all `possible_values`
    pub fn possible_values() -> impl Iterator<Item = clap::PossibleValue<'static>> {
        use clap::ArgEnum;
        Self::value_variants()
            .iter()
            .filter_map(ArgEnum::to_possible_value)
    }
}

impl std::fmt::Display for UnstableOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use clap::ArgEnum;
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl std::str::FromStr for UnstableOptions {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use clap::ArgEnum;
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
    }
}

pub fn exec(config: &Config, args: &ArgMatches) -> CargoResult<()> {
    let unstable_features: Vec<UnstableOptions> =
        args.values_of_t("unstable-features").unwrap_or_default();
    let quiet = args.is_present("quiet");
    let section = parse_section(args);
    let dep_table = section
        .to_table()
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();

    let ws = args.workspace(config)?;
    let packages = args.packages_from_flags()?;
    let packages = packages.get_packages(&ws)?;
    let package = match packages.len() {
        0 => anyhow::bail!("No packages selected.  Please specify one with `-p <PKGID>`"),
        1 => packages[0],
        len => anyhow::bail!(
            "{} packages selected.  Please specify one with `-p <PKGID>`",
            len
        ),
    };
    let manifest_path = package.manifest_path();
    let manifest_path = manifest_path.to_path_buf();
    let mut manifest = LocalManifest::try_new(&manifest_path)?;

    let raw_deps = parse_dependencies(args, &unstable_features)?;

    let registry = args.registry(config)?;
    if !args.is_present("offline") && std::env::var("CARGO_IS_TEST").is_err() {
        let url = registry_url(&manifest_path, registry.as_deref())?;
        update_registry_index(&url, quiet)?;
    }

    let deps = raw_deps
        .iter()
        .map(|raw| resolve_dependency(&manifest, raw, &ws))
        .collect::<CargoResult<Vec<_>>>()?;

    let was_sorted = manifest
        .get_table(&dep_table)
        .map(TomlItem::as_table)
        .map_or(true, |table_option| {
            table_option.map_or(true, |table| is_sorted(table.iter().map(|(name, _)| name)))
        });
    for dep in deps {
        if let Some(req_feats) = dep.features.as_deref() {
            let req_feats: BTreeSet<_> = req_feats.iter().map(|s| s.as_str()).collect();

            let available_features = dep
                .available_features
                .keys()
                .map(|s| s.as_ref())
                .collect::<BTreeSet<&str>>();

            let mut unknown_features: Vec<&&str> =
                req_feats.difference(&available_features).collect();
            unknown_features.sort();

            if !unknown_features.is_empty() {
                unrecognized_features_message(&format!(
                    "Unrecognized features: {:?}",
                    unknown_features
                ))?;
            };
        }

        if !quiet {
            print_msg(&dep, &dep_table)?;
        }
        if let Some(path) = dep.path() {
            if path == manifest.path.parent().unwrap_or_else(|| Path::new("")) {
                anyhow::bail!(
                    "Cannot add `{}` as a dependency to itself",
                    manifest.package_name()?
                )
            }
        }
        manifest.insert_into_table(&dep_table, &dep)?;
        manifest.gc_dep(dep.toml_key());
    }

    if was_sorted {
        if let Some(table) = manifest
            .get_table_mut(&dep_table)
            .ok()
            .and_then(TomlItem::as_table_like_mut)
        {
            table.sort_values();
        }
    }

    manifest.write()?;

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RawDependency<'m> {
    crate_spec: &'m str,
    rename: Option<&'m str>,

    features: Option<IndexSet<&'m str>>,
    default_features: Option<bool>,

    optional: Option<bool>,

    registry: Option<&'m str>,

    section: Section<'m>,

    git: Option<&'m str>,
    branch: Option<&'m str>,
    rev: Option<&'m str>,
    tag: Option<&'m str>,
}

fn parse_dependencies<'m>(
    matches: &'m ArgMatches,
    unstable_features: &[UnstableOptions],
) -> CargoResult<Vec<RawDependency<'m>>> {
    let crates = matches
        .values_of("crates")
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let git = matches.value_of("git");
    let branch = matches.value_of("branch");
    let rev = matches.value_of("rev");
    let tag = matches.value_of("tag");
    let rename = matches.value_of("rename");
    let registry = matches.value_of("registry");
    let default_features = default_features(matches);
    let features = matches
        .values_of("features")
        .map(|f| f.flat_map(parse_feature).collect::<IndexSet<_>>());
    let optional = optional(matches);
    let section = parse_section(matches);

    if crates.len() > 1 && git.is_some() {
        anyhow::bail!("Cannot specify multiple crates with path or git or vers");
    }
    if git.is_some() && !unstable_features.contains(&UnstableOptions::Git) {
        anyhow::bail!("`--git` is unstable and requires `-Z git`");
    }

    if crates.len() > 1 && rename.is_some() {
        anyhow::bail!("Cannot specify multiple crates with rename");
    }

    if crates.len() > 1 && features.is_some() {
        anyhow::bail!("Cannot specify multiple crates with features");
    }

    let mut deps: Vec<RawDependency> = Vec::new();
    for crate_spec in crates {
        if let Some(features) = crate_spec.strip_prefix('+') {
            if !unstable_features.contains(&UnstableOptions::InlineAdd) {
                anyhow::bail!("`+<feature>` is unstable and requires `-Z inline-add`");
            }

            if let Some(prior) = deps.last_mut() {
                let features = parse_feature(features);
                prior
                    .features
                    .get_or_insert_with(Default::default)
                    .extend(features);
            } else {
                anyhow::bail!("`+<feature>` must be preceded by a pkgid");
            }
        } else {
            let dep = RawDependency {
                crate_spec,
                rename,
                features: features.clone(),
                default_features,
                optional,
                registry,
                section: section.clone(),
                git,
                branch,
                rev,
                tag,
            };
            deps.push(dep);
        }
    }
    Ok(deps)
}

fn parse_feature(feature: &str) -> impl Iterator<Item = &str> {
    feature.split([' ', ',']).filter(|s| !s.is_empty())
}

fn default_features(matches: &ArgMatches) -> Option<bool> {
    resolve_bool_arg(
        matches.is_present("default-features"),
        matches.is_present("no-default-features"),
    )
}

fn optional(matches: &ArgMatches) -> Option<bool> {
    resolve_bool_arg(
        matches.is_present("optional"),
        matches.is_present("no-optional"),
    )
}

fn resolve_bool_arg(yes: bool, no: bool) -> Option<bool> {
    match (yes, no) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
        (_, _) => unreachable!("clap should make this impossible"),
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Section<'m> {
    Dep,
    DevDep,
    BuildDep,
    TargetDep(&'m str),
}

impl<'m> Section<'m> {
    fn to_table(&self) -> Vec<&str> {
        match self {
            Self::Dep => vec!["dependencies"],
            Self::DevDep => vec!["dev-dependencies"],
            Self::BuildDep => vec!["build-dependencies"],
            Self::TargetDep(target) => vec!["target", target, "dependencies"],
        }
    }
}

fn parse_section(matches: &ArgMatches) -> Section<'_> {
    if matches.is_present("dev") {
        Section::DevDep
    } else if matches.is_present("build") {
        Section::BuildDep
    } else if let Some(target) = matches.value_of("target") {
        assert!(!target.is_empty(), "Target specification may not be empty");
        Section::TargetDep(target)
    } else {
        Section::Dep
    }
}

fn resolve_dependency(
    manifest: &LocalManifest,
    arg: &RawDependency<'_>,
    ws: &cargo::core::Workspace,
) -> CargoResult<Dependency> {
    let crate_spec = CrateSpec::resolve(arg.crate_spec)?;
    let manifest_path = manifest.path.as_path();

    let mut dependency = match &crate_spec {
        CrateSpec::PkgId {
            name: _,
            version_req: Some(_),
        } => {
            let mut dependency = crate_spec.to_dependency()?;
            dependency = populate_dependency(dependency, arg);
            // crate specifier includes a version (e.g. `docopt@0.8`)
            if let Some(url) = arg.git {
                let url = url.clone();
                let version = dependency.version().unwrap().to_string();
                anyhow::bail!(
                    "Cannot specify a git URL (`{}`) with a version (`{}`).",
                    url,
                    version
                )
            }

            dependency
        }
        CrateSpec::PkgId {
            name,
            version_req: None,
        } => {
            let mut dependency = crate_spec.to_dependency()?;
            dependency = populate_dependency(dependency, arg);

            if let Some(repo) = arg.git {
                assert!(arg.registry.is_none());
                dependency = dependency.set_git(
                    repo,
                    arg.branch.map(String::from),
                    arg.tag.map(String::from),
                    arg.rev.map(String::from),
                );
            } else if let Some(old) = get_existing_dependency(arg, manifest, dependency.toml_key())
            {
                dependency = populate_dependency(old, arg);
            } else if let Some(package) = ws.members().find(|p| p.name().as_str() == *name) {
                // Only special-case workspaces when the user doesn't provide any extra
                // information, otherwise, trust the user.
                dependency = dependency.set_path(package.root().to_owned());
                // dev-dependencies do not need the version populated
                if arg.section != Section::DevDep {
                    let op = "";
                    let v = format!("{op}{version}", op = op, version = package.version());
                    dependency = dependency.set_version(&v);
                }
            } else {
                let registry_url = registry_url(manifest_path, arg.registry)?;
                let latest =
                    get_latest_dependency(name, false, manifest_path, Some(&registry_url))?;

                let op = "";
                let v = format!(
                    "{op}{version}",
                    op = op,
                    // If version is unavailable `get_latest_dependency` must have
                    // returned `Err(FetchVersionError::GetVersion)`
                    version = latest.version().unwrap_or_else(|| unreachable!())
                );

                dependency.name = latest.name; // Normalize the name
                dependency = dependency
                    .set_version(&v)
                    .set_available_features(latest.available_features);
            }

            dependency
        }
        CrateSpec::Path(_) => {
            let mut dependency = crate_spec.to_dependency()?;
            dependency = populate_dependency(dependency, arg);

            if let Some(old) = get_existing_dependency(arg, manifest, dependency.toml_key()) {
                if old.path() == dependency.path() {
                    if let Some(version) = old.version() {
                        dependency = dependency.set_version(version);
                    }
                }
            } else if arg.section != Section::DevDep {
                // dev-dependencies do not need the version populated
                let dep_path = dependency.path().map(ToOwned::to_owned);
                if let Some(dep_path) = dep_path {
                    if let Some(package) = ws.members().find(|p| p.root() == dep_path.as_path()) {
                        let op = "";
                        let v = format!("{op}{version}", op = op, version = package.version());

                        dependency = dependency.set_version(&v);
                    }
                }
            }

            dependency
        }
    };

    if let Some(registry) = arg.registry {
        dependency = dependency.set_registry(registry);
    }
    dependency = populate_available_features(dependency, manifest_path, arg)?;

    Ok(dependency)
}

/// Provide the existing dependency for the target table
///
/// If it doesn't exist but exists in another table, let's use that as most likely users
/// want to use the same version across all tables unless they are renaming.
fn get_existing_dependency(
    arg: &RawDependency<'_>,
    manifest: &LocalManifest,
    dep_key: &str,
) -> Option<Dependency> {
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
    enum Key {
        Dev,
        Build,
        Target,
        Runtime,
        Existing,
    }

    let target_section = arg.section.to_table();
    let mut possible: Vec<_> = manifest
        .get_dependency_versions(dep_key)
        .filter_map(|(path, dep)| dep.ok().map(|dep| (path, dep)))
        .map(|(path, dep)| {
            let key = if path == target_section {
                Key::Existing
            } else {
                match path[0].as_str() {
                    "dependencies" => Key::Runtime,
                    "target" => Key::Target,
                    "build-dependencies" => Key::Build,
                    "dev-dependencies" => Key::Dev,
                    other => unreachable!("Unknown dependency section: {}", other),
                }
            };
            (key, dep)
        })
        .collect();
    if possible.is_empty() {
        return None;
    }

    possible.sort_by_key(|(key, _)| *key);
    let (key, mut dep) = possible.pop().expect("checked for empty earlier");
    // dev-dependencies do not need the version populated when path is set though we
    // should preserve it if the user chose to populate it.
    if dep.path().is_some() && arg.section == Section::DevDep && key != Key::Existing {
        dep = dep.clear_version();
    }
    Some(dep)
}

fn populate_dependency(mut dependency: Dependency, arg: &RawDependency<'_>) -> Dependency {
    let requested_features: Option<Vec<_>> = arg.features.as_ref().map(|v| {
        v.iter()
            .flat_map(|s| parse_feature(s))
            .map(|f| f.to_owned())
            .collect()
    });

    dependency = dependency
        .set_optional(arg.optional)
        .set_default_features(arg.default_features)
        .set_features(requested_features);

    if let Some(ref rename) = arg.rename {
        dependency = dependency.set_rename(rename);
    }

    dependency
}

/// Lookup available features
fn populate_available_features(
    dependency: Dependency,
    manifest_path: &Path,
    arg: &RawDependency<'_>,
) -> CargoResult<Dependency> {
    if !dependency.available_features.is_empty() {
        return Ok(dependency);
    }

    let available_features = if let Some(path) = dependency.path() {
        let manifest = get_manifest_from_path(path)?;
        manifest.features()?
    } else if let Some(repo) = dependency.git() {
        get_manifest_from_url(repo)?
            .map(|m| m.features())
            .transpose()?
            .unwrap_or_default()
    } else if let Some(version) = dependency.version() {
        let registry_url = registry_url(manifest_path, arg.registry.as_deref())?;
        get_features_from_registry(&dependency.name, version, &registry_url)?
    } else {
        BTreeMap::new()
    };

    let dependency = dependency.set_available_features(available_features);
    Ok(dependency)
}

fn print_msg(dep: &Dependency, section: &[String]) -> CargoResult<()> {
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
    if dep.optional().unwrap_or(false) {
        write!(output, " optional")?;
    }
    let section = if section.len() == 1 {
        section[0].clone()
    } else {
        format!("{} for target `{}`", &section[2], &section[1])
    };
    write!(output, " {}", section)?;
    writeln!(output, ".")?;

    let mut activated = dep.features.clone().unwrap_or_default();
    if dep.default_features().unwrap_or(true) {
        activated.extend(
            dep.available_features
                .get("default")
                .into_iter()
                .flat_map(|v| v.clone()),
        );
    }
    activated.sort();
    let mut deactivated;
    if dep.available_features.is_empty() {
        deactivated = vec![];
    } else {
        deactivated = dep
            .available_features
            .keys()
            .filter(|f| !activated.contains(f) && *f != "default")
            .collect::<Vec<_>>();
    }
    deactivated.sort();
    if !activated.is_empty() || !deactivated.is_empty() {
        writeln!(output, "{:>13}Features:", " ")?;
        for feat in activated {
            output.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
            write!(output, "{:>13}+ ", " ")?;
            output.reset()?;
            writeln!(output, "{}", feat)?;
        }
        for feat in deactivated {
            output.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
            write!(output, "{:>13}- ", " ")?;
            output.reset()?;
            writeln!(output, "{}", feat)?;
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

fn unrecognized_features_message(message: &str) -> CargoResult<()> {
    let colorchoice = colorize_stderr();
    let mut output = StandardStream::stderr(colorchoice);
    output.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
    write!(output, "{:>12}", "Warning:")?;
    output.reset()?;
    writeln!(output, " {}", message)
        .with_context(|| "Failed to write unrecognized features message")?;
    Ok(())
}