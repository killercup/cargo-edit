//! Core of cargo-add command

mod crate_spec;
mod dependency;
mod manifest;

use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::path::Path;

use anyhow::Context as _;
use cargo::core::Registry;
use cargo::CargoResult;
use cargo::Config;
use indexmap::IndexSet;
use toml_edit::Item as TomlItem;

use crate_spec::CrateSpec;
use dependency::Dependency;
use dependency::GitSource;
use dependency::PathSource;
use dependency::RegistrySource;
use dependency::Source;
use manifest::LocalManifest;

/// Information on what dependencies should be added
#[derive(Clone, Debug)]
pub struct AddOptions<'a> {
    /// Configuration information for cargo operations
    pub config: &'a Config,
    /// Package to add dependencies to
    pub spec: &'a cargo::core::Package,
    /// Dependencies to add or modify
    pub dependencies: Vec<DepOp>,
    /// Which dependency section to add these to
    pub section: DepTable<'a>,
    /// Act as if dependencies will be added
    pub dry_run: bool,
}

/// Add dependencies to a manifest
pub fn add(workspace: &cargo::core::Workspace, options: &AddOptions<'_>) -> CargoResult<()> {
    let dep_table = options
        .section
        .to_table()
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();

    let manifest_path = options.spec.manifest_path().to_path_buf();
    let mut manifest = LocalManifest::try_new(&manifest_path)?;

    let mut registry = cargo::core::registry::PackageRegistry::new(options.config)?;

    let deps = {
        let _lock = options.config.acquire_package_cache_lock()?;
        registry.lock_patches();
        options
            .dependencies
            .iter()
            .map(|raw| {
                resolve_dependency(
                    &manifest,
                    raw,
                    workspace,
                    options.section,
                    options.config,
                    &mut registry,
                )
            })
            .collect::<CargoResult<Vec<_>>>()?
    };

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
                options
                    .config
                    .shell()
                    .warn(format!("unrecognized features: {unknown_features:?}"))?;
            };
        }

        print_msg(&mut options.config.shell(), &dep, &dep_table)?;
        if let Some(Source::Path(src)) = dep.source() {
            if src.path == manifest.path.parent().unwrap_or_else(|| Path::new("")) {
                anyhow::bail!(
                    "cannot add `{}` as a dependency to itself",
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
        options.config.shell().warn("aborting add due to dry run")?;
    } else {
        manifest.write()?;
    }

    Ok(())
}

/// Dependency entry operation
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepOp {
    /// Describes the crate
    pub crate_spec: String,
    /// Dependency key, overriding the package name in crate_spec
    pub rename: Option<String>,

    /// Feature flags to activate
    pub features: Option<IndexSet<String>>,
    /// Whether the default feature should be activated
    pub default_features: Option<bool>,

    /// Whether dependency is optional
    pub optional: Option<bool>,

    /// Registry for looking up dependency version
    pub registry: Option<String>,

    /// Git repo for dependency
    pub git: Option<String>,
    /// Specify an alternative git branch
    pub branch: Option<String>,
    /// Specify a specific git rev
    pub rev: Option<String>,
    /// Specify a specific git tag
    pub tag: Option<String>,
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
    arg: &DepOp,
    ws: &cargo::core::Workspace,
    section: DepTable<'_>,
    config: &Config,
    registry: &mut cargo::core::registry::PackageRegistry,
) -> CargoResult<Dependency> {
    let crate_spec = CrateSpec::resolve(&arg.crate_spec)?;

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

    if let Some(url) = &arg.git {
        match &crate_spec {
            CrateSpec::Path(path) => {
                anyhow::bail!(
                    "cannot specify a git URL (`{url}`) with a path (`{}`).",
                    path.display()
                )
            }
            CrateSpec::PkgId {
                name: _,
                version_req: Some(v),
            } => {
                // crate specifier includes a version (e.g. `docopt@0.8`)
                anyhow::bail!("cannot specify a git URL (`{url}`) with a version (`{v}`).",)
            }
            CrateSpec::PkgId {
                name: _,
                version_req: None,
            } => {
                assert!(arg.registry.is_none());
                let mut src = GitSource::new(url);
                if let Some(branch) = &arg.branch {
                    src = src.set_branch(branch);
                }
                if let Some(tag) = &arg.tag {
                    src = src.set_tag(tag);
                }
                if let Some(rev) = &arg.rev {
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
                let v = format!("{op}{version}", version = package.version());
                src = src.set_version(v);
            }
            dependency = dependency.set_source(src);
        } else {
            let latest = get_latest_dependency(&dependency, false, config, registry)?;

            if dependency.name != latest.name {
                config.shell().warn(format!(
                    "translating `{}` to `{}`",
                    dependency.name, latest.name,
                ))?;
                dependency.name = latest.name; // Normalize the name
            }
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

    dependency = populate_available_features(dependency, config, registry)?;

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

fn get_latest_dependency(
    dependency: &Dependency,
    _flag_allow_prerelease: bool,
    config: &Config,
    registry: &mut cargo::core::registry::PackageRegistry,
) -> CargoResult<Dependency> {
    let query = dependency.query(config)?;
    let possibilities = loop {
        match registry.query_vec(&query, true) {
            std::task::Poll::Ready(res) => {
                break res?;
            }
            std::task::Poll::Pending => registry.block_until_ready()?,
        }
    };
    let latest = possibilities
        .iter()
        .max_by_key(|s| {
            // Fallback to a pre-release if no official release is available by sorting them as
            // less.
            let stable = s.version().pre.is_empty();
            (stable, s.version())
        })
        .ok_or_else(|| {
            anyhow::format_err!("the crate `{dependency}` could not be found in registry index.")
        })?;
    let mut dep = Dependency::from(latest);
    if let Some(reg_name) = dependency.registry.as_deref() {
        dep = dep.set_registry(reg_name);
    }
    Ok(dep)
}

fn populate_dependency(mut dependency: Dependency, arg: &DepOp) -> Dependency {
    if let Some(registry) = &arg.registry {
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
    if let Some(value) = arg.features.as_ref() {
        dependency = dependency.extend_features(value.iter().cloned());
    }

    if let Some(rename) = &arg.rename {
        dependency = dependency.set_rename(rename);
    }

    dependency
}

/// Lookup available features
fn populate_available_features(
    mut dependency: Dependency,
    config: &Config,
    registry: &mut cargo::core::registry::PackageRegistry,
) -> CargoResult<Dependency> {
    if !dependency.available_features.is_empty() {
        return Ok(dependency);
    }

    let query = dependency.query(config)?;
    let possibilities = loop {
        match registry.query_vec(&query, true) {
            std::task::Poll::Ready(res) => {
                break res?;
            }
            std::task::Poll::Pending => registry.block_until_ready()?,
        }
    };
    // Ensure widest feature flag compatibility by picking the earliest version that could show up
    // in the lock file for a given version requirement.
    let lowest_common_denominator = possibilities
        .iter()
        .min_by_key(|s| {
            // Fallback to a pre-release if no official release is available by sorting them as
            // more.
            let is_pre = !s.version().pre.is_empty();
            (is_pre, s.version())
        })
        .ok_or_else(|| {
            anyhow::format_err!("the crate `{dependency}` could not be found in registry index.")
        })?;
    dependency = dependency.set_available_features_from_cargo(lowest_common_denominator.features());

    Ok(dependency)
}

fn print_msg(
    shell: &mut cargo::core::Shell,
    dep: &Dependency,
    section: &[String],
) -> CargoResult<()> {
    use std::fmt::Write;

    let mut message = String::new();
    write!(message, "{}", dep.name)?;
    match dep.source() {
        Some(Source::Registry(src)) => {
            if src.version.chars().next().unwrap_or('0').is_ascii_digit() {
                write!(message, " v{}", src.version)?;
            } else {
                write!(message, " {}", src.version)?;
            }
        }
        Some(Source::Path(_)) => {
            write!(message, " (local)")?;
        }
        Some(Source::Git(_)) => {
            write!(message, " (git)")?;
        }
        None => {}
    }
    write!(message, " to")?;
    if dep.optional().unwrap_or(false) {
        write!(message, " optional")?;
    }
    let section = if section.len() == 1 {
        section[0].clone()
    } else {
        format!("{} for target `{}`", &section[2], &section[1])
    };
    write!(message, " {section}")?;
    write!(message, ".")?;

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
        writeln!(message)?;
        write!(message, "{:>13}Features:", " ")?;
        for feat in activated {
            writeln!(message)?;
            write!(message, "{:>13}+ {}", " ", feat)?;
        }
        for feat in deactivated {
            writeln!(message)?;
            write!(message, "{:>13}- {}", " ", feat)?;
        }
    }

    shell.status("Adding", message)?;

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

fn get_manifest_from_path(path: &Path) -> CargoResult<LocalManifest> {
    let cargo_file = path.join("Cargo.toml");
    LocalManifest::try_new(&cargo_file).with_context(|| "Unable to open local Cargo.toml")
}
