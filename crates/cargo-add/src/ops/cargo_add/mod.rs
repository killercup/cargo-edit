//! Core of cargo-add command

mod crate_spec;
mod dependency;
mod errors;
mod fetch;
mod manifest;
mod registry;
mod util;
mod version;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::io::Write;
use std::path::Path;

use cargo::Config;
use indexmap::IndexSet;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use toml_edit::Item as TomlItem;

use crate_spec::CrateSpec;
use dependency::Dependency;
use dependency::GitSource;
use dependency::PathSource;
use dependency::RegistrySource;
use dependency::Source;
use fetch::{
    get_features_from_registry, get_latest_dependency, get_manifest_from_path,
    get_manifest_from_url, update_registry_index,
};
use manifest::LocalManifest;
use manifest::Manifest;
use registry::registry_url;
use util::colorize_stderr;
use version::VersionExt;

pub use errors::*;

/// Information on what dependencies should be added
#[derive(Clone, Debug)]
pub struct AddOptions<'a> {
    /// Configuration information for cargo operations
    pub config: &'a Config,
    /// Package to add dependencies to
    pub spec: &'a cargo::core::Package,
    /// Dependencies to add or modify
    pub dependencies: Vec<DepOp<'a>>,
    /// Which dependency section to add these to
    pub section: DepTable<'a>,
    /// Act as if dependencies will be added
    pub dry_run: bool,

    /// TODO: Remove this
    pub quiet: bool,
    /// TODO: Remove this
    pub registry: Option<&'a str>,
}

/// Add dependencies to a manifest
pub fn add(workspace: &cargo::core::Workspace, options: &AddOptions<'_>) -> CargoResult<()> {
    let dep_table = options
        .section
        .to_table()
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();

    let manifest_path = options.spec.manifest_path();
    let manifest_path = manifest_path.to_path_buf();
    let work_dir = manifest_path.parent().expect("always a parent directory");
    let mut manifest = LocalManifest::try_new(&manifest_path)?;

    if !options.config.offline() && std::env::var("CARGO_IS_TEST").is_err() {
        let url = registry_url(work_dir, options.registry)?;
        update_registry_index(&url, options.quiet)?;
    }

    let deps = options
        .dependencies
        .iter()
        .map(|raw| resolve_dependency(&manifest, raw, workspace, options.section))
        .collect::<CargoResult<Vec<_>>>()?;

    let was_sorted = manifest
        .get_table(&dep_table)
        .map(TomlItem::as_table)
        .map_or(true, |table_option| {
            table_option.map_or(true, |table| is_sorted(table.iter().map(|(name, _)| name)))
        });
    for dep in deps {
        if let Some(req_feats) = dep.features.as_ref() {
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

        if !options.quiet {
            print_msg(&dep, &dep_table)?;
        }
        if let Some(Source::Path(src)) = dep.source() {
            if src.path == manifest.path.parent().unwrap_or_else(|| Path::new("")) {
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

    if options.dry_run {
        dry_run_message()?;
    } else {
        manifest.write()?;
    }

    Ok(())
}

/// Dependency entry operation
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepOp<'m> {
    /// Describes the crate
    pub crate_spec: &'m str,
    /// Dependency key, overriding the package name in crate_spec
    pub rename: Option<&'m str>,

    /// Feature flags to activate
    pub features: Option<IndexSet<&'m str>>,
    /// Whether the default feature should be activated
    pub default_features: Option<bool>,

    /// Whether dependency is optional
    pub optional: Option<bool>,

    /// Registry for looking up dependency version
    pub registry: Option<&'m str>,

    /// Git repo for dependency
    pub git: Option<&'m str>,
    /// Specify an alternative git branch
    pub branch: Option<&'m str>,
    /// Specify a specific git rev
    pub rev: Option<&'m str>,
    /// Specify a specific git tag
    pub tag: Option<&'m str>,
}

/// Dependency table to add dep to
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DepTable<'m> {
    /// Used for building final artifact
    Normal,
    /// Used for testing
    Development,
    /// Used for build.rs
    Build,
    /// Used for building final artifact only on specific target platforms
    Target(&'m str),
}

impl<'m> DepTable<'m> {
    fn to_table(self) -> Vec<&'m str> {
        match self {
            Self::Normal => vec!["dependencies"],
            Self::Development => vec!["dev-dependencies"],
            Self::Build => vec!["build-dependencies"],
            Self::Target(target) => vec!["target", target, "dependencies"],
        }
    }
}

fn resolve_dependency(
    manifest: &LocalManifest,
    arg: &DepOp<'_>,
    ws: &cargo::core::Workspace,
    section: DepTable<'_>,
) -> CargoResult<Dependency> {
    let crate_spec = CrateSpec::resolve(arg.crate_spec)?;
    let manifest_path = manifest.path.as_path();

    let mut spec_dep = crate_spec.to_dependency()?;
    spec_dep = populate_dependency(spec_dep, arg);

    let old_dep = get_existing_dependency(manifest, spec_dep.toml_key(), section);

    let mut dependency = if let Some(mut old_dep) = old_dep.clone() {
        if spec_dep.source().is_some() {
            // Overwrite with `crate_spec`
            old_dep.source = spec_dep.source;
        }
        old_dep = populate_dependency(old_dep, arg);
        old_dep
    } else {
        spec_dep
    };

    if let Some(url) = arg.git {
        match &crate_spec {
            CrateSpec::Path(path) => {
                anyhow::bail!(
                    "Cannot specify a git URL (`{}`) with a path (`{}`).",
                    url,
                    path.display()
                )
            }
            CrateSpec::PkgId {
                name: _,
                version_req: Some(v),
            } => {
                // crate specifier includes a version (e.g. `docopt@0.8`)
                anyhow::bail!(
                    "Cannot specify a git URL (`{}`) with a version (`{}`).",
                    url,
                    v
                )
            }
            CrateSpec::PkgId {
                name: _,
                version_req: None,
            } => {
                assert!(arg.registry.is_none());
                let mut src = GitSource::new(url);
                if let Some(branch) = arg.branch {
                    src = src.set_branch(branch);
                }
                if let Some(tag) = arg.tag {
                    src = src.set_tag(tag);
                }
                if let Some(rev) = arg.rev {
                    src = src.set_rev(rev);
                }
                dependency = dependency.set_source(src);
            }
        }
    }

    if dependency.source().is_none() {
        if let Some(package) = ws.members().find(|p| p.name().as_str() == dependency.name) {
            // Only special-case workspaces when the user doesn't provide any extra
            // information, otherwise, trust the user.
            let mut src = PathSource::new(package.root());
            // dev-dependencies do not need the version populated
            if section != DepTable::Development {
                let op = "";
                let v = format!("{op}{version}", op = op, version = package.version());
                src = src.set_version(v);
            }
            dependency = dependency.set_source(src);
        } else {
            let work_dir = manifest_path.parent().expect("always a parent directory");
            let latest = get_latest_dependency(
                dependency.name.as_str(),
                false,
                work_dir,
                dependency.registry(),
            )?;

            dependency.name = latest.name; // Normalize the name
            dependency = dependency
                .set_source(latest.source.expect("latest always has a source"))
                .set_available_features(latest.available_features);
        }
    }

    let version_required = dependency.source().and_then(|s| s.as_registry()).is_some();
    let version_optional_in_section = section == DepTable::Development;
    let preserve_existing_version = old_dep
        .as_ref()
        .map(|d| d.version().is_some())
        .unwrap_or(false);
    if !version_required && !preserve_existing_version && version_optional_in_section {
        // dev-dependencies do not need the version populated
        dependency = dependency.clear_version();
    }

    dependency = populate_available_features(dependency, manifest_path)?;

    Ok(dependency)
}

/// Provide the existing dependency for the target table
///
/// If it doesn't exist but exists in another table, let's use that as most likely users
/// want to use the same version across all tables unless they are renaming.
fn get_existing_dependency(
    manifest: &LocalManifest,
    dep_key: &str,
    section: DepTable<'_>,
) -> Option<Dependency> {
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
    enum Key {
        Dev,
        Build,
        Target,
        Runtime,
        Existing,
    }

    let target_section = section.to_table();
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
    possible.sort_by_key(|(key, _)| *key);
    let (key, mut dep) = possible.pop()?;

    if key != Key::Existing {
        // When the dep comes from a different section, we only care about the source and not any
        // of the other fields, like `features`
        let unrelated = dep;
        dep = Dependency::new(&unrelated.name);
        dep.source = unrelated.source.clone();
        dep.registry = unrelated.registry.clone();

        // dev-dependencies do not need the version populated when path is set though we
        // should preserve it if the user chose to populate it.
        let version_required = unrelated.source().and_then(|s| s.as_registry()).is_some();
        let version_optional_in_section = section == DepTable::Development;
        if !version_required && version_optional_in_section {
            dep = dep.clear_version();
        }
    }

    Some(dep)
}

fn populate_dependency(mut dependency: Dependency, arg: &DepOp<'_>) -> Dependency {
    let requested_features: Option<IndexSet<_>> = arg.features.as_ref().map(|v| {
        v.iter()
            .flat_map(|s| parse_feature(s))
            .map(|f| f.to_owned())
            .collect()
    });

    if let Some(registry) = arg.registry {
        if registry.is_empty() {
            dependency.registry = None;
        } else {
            dependency.registry = Some(registry.to_owned());
        }
    }
    if let Some(value) = arg.optional {
        if value {
            dependency.optional = Some(true);
        } else {
            dependency.optional = None;
        }
    }
    if let Some(value) = arg.default_features {
        if value {
            dependency.default_features = None;
        } else {
            dependency.default_features = Some(false);
        }
    }
    if let Some(value) = requested_features {
        dependency = dependency.extend_features(value);
    }

    if let Some(rename) = arg.rename {
        dependency = dependency.set_rename(rename);
    }

    dependency
}

/// Split feature flag list
pub fn parse_feature(feature: &str) -> impl Iterator<Item = &str> {
    feature.split([' ', ',']).filter(|s| !s.is_empty())
}

/// Lookup available features
fn populate_available_features(
    dependency: Dependency,
    manifest_path: &Path,
) -> CargoResult<Dependency> {
    if !dependency.available_features.is_empty() {
        return Ok(dependency);
    }

    let available_features = match dependency.source() {
        Some(Source::Registry(src)) => {
            let work_dir = manifest_path.parent().expect("always a parent directory");
            let registry_url = registry_url(work_dir, dependency.registry())?;
            get_features_from_registry(&dependency.name, &src.version, &registry_url)?
        }
        Some(Source::Path(src)) => {
            let manifest = get_manifest_from_path(&src.path)?;
            manifest.features()?
        }
        Some(Source::Git(git)) => get_manifest_from_url(&git.git)?
            .map(|m| m.features())
            .transpose()?
            .unwrap_or_default(),
        None => BTreeMap::new(),
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
    match dep.source() {
        Some(Source::Registry(src)) => {
            if src.version.chars().next().unwrap_or('0').is_ascii_digit() {
                write!(output, " v{}", src.version)?;
            } else {
                write!(output, " {}", src.version)?;
            }
        }
        Some(Source::Path(_)) => {
            write!(output, " (local)")?;
        }
        Some(Source::Git(_)) => {
            write!(output, " (git)")?;
        }
        None => {}
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

    let mut activated: IndexSet<_> = dep.features.iter().flatten().map(|s| s.as_str()).collect();
    if dep.default_features().unwrap_or(true) {
        activated.insert("default");
    }
    let mut walk: VecDeque<_> = activated.iter().cloned().collect();
    while let Some(next) = walk.pop_front() {
        walk.extend(
            dep.available_features
                .get(next)
                .into_iter()
                .flatten()
                .map(|s| s.as_str()),
        );
        activated.extend(
            dep.available_features
                .get(next)
                .into_iter()
                .flatten()
                .map(|s| s.as_str()),
        );
    }
    activated.remove("default");
    activated.sort();
    let mut deactivated = dep
        .available_features
        .keys()
        .filter(|f| !activated.contains(f.as_str()) && *f != "default")
        .collect::<Vec<_>>();
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

fn dry_run_message() -> CargoResult<()> {
    let colorchoice = colorize_stderr();
    let mut output = StandardStream::stderr(colorchoice);
    output.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
    write!(output, "{:>12}", "Warning:")?;
    output.reset()?;
    writeln!(output, " aborting add due to dry run")
        .with_context(|| "Failed to write unrecognized features message")?;
    Ok(())
}
