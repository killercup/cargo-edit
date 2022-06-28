use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::{env, str};

use semver::{Version, VersionReq};
use termcolor::{BufferWriter, Color, ColorSpec, WriteColor};

use super::dependency::Dependency;
use super::errors::*;

const MANIFEST_FILENAME: &str = "Cargo.toml";
const DEP_TABLES: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];

/// A Cargo manifest
#[derive(Debug, Clone)]
pub struct Manifest {
    /// Manifest contents as TOML data
    pub data: toml_edit::Document,
}

impl Manifest {
    /// Get the manifest's package name
    pub fn package_name(&self) -> CargoResult<&str> {
        self.data
            .as_table()
            .get("package")
            .and_then(|m| m["name"].as_str())
            .ok_or_else(parse_manifest_err)
    }

    /// Get the specified table from the manifest.
    pub fn get_table<'a>(&'a self, table_path: &[String]) -> CargoResult<&'a toml_edit::Item> {
        /// Descend into a manifest until the required table is found.
        fn descend<'a>(
            input: &'a toml_edit::Item,
            path: &[String],
        ) -> CargoResult<&'a toml_edit::Item> {
            if let Some(segment) = path.get(0) {
                let value = input
                    .get(&segment)
                    .ok_or_else(|| non_existent_table_err(segment))?;

                if value.is_table_like() {
                    descend(value, &path[1..])
                } else {
                    Err(non_existent_table_err(segment))
                }
            } else {
                Ok(input)
            }
        }

        descend(self.data.as_item(), table_path)
    }

    /// Get the specified table from the manifest.
    ///
    /// If there is no table at the specified path, then a non-existent table
    /// error will be returned.
    pub fn get_table_mut<'a>(
        &'a mut self,
        table_path: &[String],
    ) -> CargoResult<&'a mut toml_edit::Item> {
        self.get_table_mut_internal(table_path, false)
    }

    /// Get the specified table from the manifest, inserting it if it does not
    /// exist.
    pub fn get_or_insert_table_mut<'a>(
        &'a mut self,
        table_path: &[String],
    ) -> CargoResult<&'a mut toml_edit::Item> {
        self.get_table_mut_internal(table_path, true)
    }

    /// Get all sections in the manifest that exist and might contain dependencies.
    /// The returned items are always `Table` or `InlineTable`.
    pub fn get_sections(&self) -> Vec<(Vec<String>, toml_edit::Item)> {
        let mut sections = Vec::new();

        for dependency_type in DEP_TABLES {
            // Dependencies can be in the three standard sections...
            if self
                .data
                .get(dependency_type)
                .map(|t| t.is_table_like())
                .unwrap_or(false)
            {
                sections.push((
                    vec![String::from(*dependency_type)],
                    self.data[dependency_type].clone(),
                ))
            }

            // ... and in `target.<target>.(build-/dev-)dependencies`.
            let target_sections = self
                .data
                .as_table()
                .get("target")
                .and_then(toml_edit::Item::as_table_like)
                .into_iter()
                .flat_map(toml_edit::TableLike::iter)
                .filter_map(|(target_name, target_table)| {
                    let dependency_table = target_table.get(dependency_type)?;
                    dependency_table.as_table_like().map(|_| {
                        (
                            vec![
                                "target".to_string(),
                                target_name.to_string(),
                                String::from(*dependency_type),
                            ],
                            dependency_table.clone(),
                        )
                    })
                });

            sections.extend(target_sections);
        }

        sections
    }

    /// returns features exposed by this manifest
    pub fn features(&self) -> CargoResult<BTreeMap<String, Vec<String>>> {
        let mut features: BTreeMap<String, Vec<String>> = match self.data.as_table().get("features")
        {
            None => BTreeMap::default(),
            Some(item) => match item {
                toml_edit::Item::None => BTreeMap::default(),
                toml_edit::Item::Table(t) => t
                    .iter()
                    .map(|(k, v)| {
                        let k = k.to_owned();
                        let v = v
                            .as_array()
                            .cloned()
                            .unwrap_or_default()
                            .iter()
                            .map(|v| v.as_str().map(|s| s.to_owned()))
                            .collect::<Option<Vec<_>>>();
                        v.map(|v| (k, v))
                    })
                    .collect::<Option<_>>()
                    .ok_or_else(invalid_cargo_config)?,
                _ => return Err(invalid_cargo_config()),
            },
        };

        let sections = self.get_sections();
        for (_, deps) in sections {
            features.extend(
                deps.as_table_like()
                    .unwrap()
                    .iter()
                    .filter_map(|(key, dep_item)| {
                        let table = dep_item.as_table_like()?;
                        table
                            .get("optional")
                            .and_then(|o| o.as_value())
                            .and_then(|o| o.as_bool())
                            .unwrap_or(false)
                            .then(|| (key.to_owned(), vec![]))
                    }),
            );
        }

        Ok(features)
    }

    fn get_table_mut_internal<'a>(
        &'a mut self,
        table_path: &[String],
        insert_if_not_exists: bool,
    ) -> CargoResult<&'a mut toml_edit::Item> {
        /// Descend into a manifest until the required table is found.
        fn descend<'a>(
            input: &'a mut toml_edit::Item,
            path: &[String],
            insert_if_not_exists: bool,
        ) -> CargoResult<&'a mut toml_edit::Item> {
            if let Some(segment) = path.get(0) {
                let value = if insert_if_not_exists {
                    input[&segment].or_insert(toml_edit::table())
                } else {
                    input
                        .get_mut(&segment)
                        .ok_or_else(|| non_existent_table_err(segment))?
                };

                if value.is_table_like() {
                    descend(value, &path[1..], insert_if_not_exists)
                } else {
                    Err(non_existent_table_err(segment))
                }
            } else {
                Ok(input)
            }
        }

        descend(self.data.as_item_mut(), table_path, insert_if_not_exists)
    }
}

impl str::FromStr for Manifest {
    type Err = Error;

    /// Read manifest data from string
    fn from_str(input: &str) -> ::std::result::Result<Self, Self::Err> {
        let d: toml_edit::Document = input.parse().with_context(|| "Manifest not valid TOML")?;

        Ok(Manifest { data: d })
    }
}

impl std::fmt::Display for Manifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.data.to_string();
        s.fmt(f)
    }
}

/// A Cargo manifest that is available locally.
#[derive(Debug)]
pub struct LocalManifest {
    /// Path to the manifest
    pub path: PathBuf,
    /// Manifest contents
    pub manifest: Manifest,
}

impl Deref for LocalManifest {
    type Target = Manifest;

    fn deref(&self) -> &Manifest {
        &self.manifest
    }
}

impl DerefMut for LocalManifest {
    fn deref_mut(&mut self) -> &mut Manifest {
        &mut self.manifest
    }
}

impl LocalManifest {
    /// Construct a `LocalManifest`. If no path is provided, make an educated guess as to which one
    /// the user means.
    pub fn find(path: Option<&Path>) -> CargoResult<Self> {
        let path = dunce::canonicalize(find(path)?)?;
        Self::try_new(&path)
    }

    /// Construct the `LocalManifest` corresponding to the `Path` provided.
    pub fn try_new(path: &Path) -> CargoResult<Self> {
        let path = path.to_path_buf();
        let data =
            std::fs::read_to_string(&path).with_context(|| "Failed to read manifest contents")?;
        let manifest = data.parse().with_context(|| "Unable to parse Cargo.toml")?;
        Ok(LocalManifest { manifest, path })
    }

    /// Write changes back to the file
    pub fn write(&self) -> CargoResult<()> {
        if !self.manifest.data.contains_key("package")
            && !self.manifest.data.contains_key("project")
        {
            if self.manifest.data.contains_key("workspace") {
                anyhow::bail!(
                    "Found virtual manifest at {}, but this command requires running against an \
                         actual package in this workspace.",
                    self.path.display()
                );
            } else {
                anyhow::bail!(
                    "Missing expected `package` or `project` fields in {}",
                    self.path.display()
                );
            }
        }

        let s = self.manifest.data.to_string();
        let new_contents_bytes = s.as_bytes();

        std::fs::write(&self.path, new_contents_bytes)
            .with_context(|| "Failed to write updated Cargo.toml")
    }

    /// Instruct this manifest to upgrade a single dependency. If this manifest does not have that
    /// dependency, it does nothing.
    pub fn upgrade(
        &mut self,
        dependency: &Dependency,
        dry_run: bool,
        skip_compatible: bool,
    ) -> CargoResult<()> {
        for (table_path, table) in self.get_sections() {
            let table_like = table.as_table_like().expect("Unexpected non-table");
            for (name, toml_item) in table_like.iter() {
                let dep_name = toml_item
                    .as_table_like()
                    .and_then(|t| t.get("package").and_then(|p| p.as_str()))
                    .unwrap_or(name);
                if dep_name == dependency.name {
                    if skip_compatible {
                        let old_version = get_version(toml_item)?;
                        if old_version_compatible(dependency, old_version)? {
                            continue;
                        }
                    }
                    self.update_table_named_entry(&table_path, name, dependency, dry_run)?;
                }
            }
        }

        self.write()
    }

    /// Lookup a dependency
    pub fn get_dependency(&self, table_path: &[String], dep_key: &str) -> CargoResult<Dependency> {
        let crate_root = self.path.parent().expect("manifest path is absolute");
        let table = self.get_table(table_path)?;
        let table = table
            .as_table_like()
            .ok_or_else(|| non_existent_table_err(table_path.join(".")))?;
        let dep_item = table
            .get(dep_key)
            .ok_or_else(|| non_existent_dependency_err(dep_key, table_path.join(".")))?;
        Dependency::from_toml(crate_root, dep_key, dep_item).ok_or_else(|| {
            anyhow::format_err!("Invalid dependency {}.{}", table_path.join("."), dep_key)
        })
    }

    /// Returns all dependencies
    pub fn get_dependencies(
        &self,
    ) -> impl Iterator<Item = (Vec<String>, CargoResult<Dependency>)> + '_ {
        self.filter_dependencies(|_| true)
    }

    /// Lookup a dependency
    pub fn get_dependency_versions<'s>(
        &'s self,
        dep_key: &'s str,
    ) -> impl Iterator<Item = (Vec<String>, CargoResult<Dependency>)> + 's {
        self.filter_dependencies(move |key| key == dep_key)
    }

    fn filter_dependencies<'s, P>(
        &'s self,
        mut predicate: P,
    ) -> impl Iterator<Item = (Vec<String>, CargoResult<Dependency>)> + 's
    where
        P: FnMut(&str) -> bool + 's,
    {
        let crate_root = self.path.parent().expect("manifest path is absolute");
        self.get_sections()
            .into_iter()
            .filter_map(move |(table_path, table)| {
                let table = table.into_table().ok()?;
                Some(
                    table
                        .into_iter()
                        .filter_map(|(key, item)| {
                            if predicate(&key) {
                                Some((table_path.clone(), key, item))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .map(move |(table_path, dep_key, dep_item)| {
                let dep = Dependency::from_toml(crate_root, &dep_key, &dep_item);
                match dep {
                    Some(dep) => (table_path, Ok(dep)),
                    None => {
                        let message = anyhow::format_err!(
                            "Invalid dependency {}.{}",
                            table_path.join("."),
                            dep_key
                        );
                        (table_path, Err(message))
                    }
                }
            })
    }

    /// Add entry to a Cargo.toml.
    pub fn insert_into_table(
        &mut self,
        table_path: &[String],
        dep: &Dependency,
    ) -> CargoResult<()> {
        let crate_root = self
            .path
            .parent()
            .expect("manifest path is absolute")
            .to_owned();
        let dep_key = dep.toml_key();

        let table = self.get_or_insert_table_mut(table_path)?;
        if let Some(dep_item) = table.as_table_like_mut().unwrap().get_mut(dep_key) {
            dep.update_toml(&crate_root, dep_item);
        } else {
            let new_dependency = dep.to_toml(&crate_root);
            table[dep_key] = new_dependency;
        }
        if let Some(t) = table.as_inline_table_mut() {
            t.fmt()
        }

        Ok(())
    }

    /// Update an entry in Cargo.toml.
    pub fn update_table_entry(
        &mut self,
        table_path: &[String],
        dep: &Dependency,
        dry_run: bool,
    ) -> CargoResult<()> {
        self.update_table_named_entry(table_path, dep.toml_key(), dep, dry_run)
    }

    /// Update an entry with a specified name in Cargo.toml.
    pub fn update_table_named_entry(
        &mut self,
        table_path: &[String],
        dep_key: &str,
        dep: &Dependency,
        dry_run: bool,
    ) -> CargoResult<()> {
        let crate_root = self
            .path
            .parent()
            .expect("manifest path is absolute")
            .to_owned();
        let table = self.get_or_insert_table_mut(table_path)?;

        // If (and only if) there is an old entry, merge the new one in.
        if table.as_table_like().unwrap().contains_key(dep_key) {
            let new_dependency = dep.to_toml(&crate_root);

            if let Err(e) = print_upgrade_if_necessary(&dep.name, &table[dep_key], &new_dependency)
            {
                eprintln!("Error while displaying upgrade message, {}", e);
            }
            if !dry_run {
                dep.update_toml(&crate_root, &mut table[dep_key]);
                if let Some(t) = table.as_inline_table_mut() {
                    t.fmt()
                }
            }
        }

        Ok(())
    }

    /// Remove entry from a Cargo.toml.
    ///
    /// # Examples
    ///
    /// ```
    ///   use cargo_edit::{Dependency, LocalManifest, Manifest};
    ///   use toml_edit;
    ///
    ///   let root = std::path::PathBuf::from("/").canonicalize().unwrap();
    ///   let path = root.join("Cargo.toml");
    ///   let mut manifest = LocalManifest { path, manifest: Manifest { data: toml_edit::Document::new() } };
    ///   let dep = Dependency::new("cargo-edit").set_version("0.1.0");
    ///   let _ = manifest.insert_into_table(&vec!["dependencies".to_owned()], &dep);
    ///   assert!(manifest.remove_from_table(&["dependencies".to_owned()], &dep.name).is_ok());
    ///   assert!(manifest.remove_from_table(&["dependencies".to_owned()], &dep.name).is_err());
    ///   assert!(!manifest.data.contains_key("dependencies"));
    /// ```
    pub fn remove_from_table(&mut self, table_path: &[String], name: &str) -> CargoResult<()> {
        let parent_table = self.get_table_mut(table_path)?;

        {
            let dep = parent_table
                .get_mut(name)
                .filter(|t| !t.is_none())
                .ok_or_else(|| non_existent_dependency_err(name, table_path.join(".")))?;
            // remove the dependency
            *dep = toml_edit::Item::None;
        }

        // remove table if empty
        if parent_table.as_table_like().unwrap().is_empty() {
            *parent_table = toml_edit::Item::None;
        }

        Ok(())
    }

    /// Add multiple dependencies to manifest
    pub fn add_deps(&mut self, table: &[String], deps: &[Dependency]) -> CargoResult<()> {
        deps.iter()
            .map(|dep| self.insert_into_table(table, dep))
            .collect::<CargoResult<Vec<_>>>()?;

        Ok(())
    }

    /// Allow mutating depedencies, wherever they live
    pub fn get_dependency_tables_mut<'r>(
        &'r mut self,
    ) -> impl Iterator<Item = &mut dyn toml_edit::TableLike> + 'r {
        let root = self.data.as_table_mut();
        root.iter_mut().flat_map(|(k, v)| {
            if DEP_TABLES.contains(&k.get()) {
                v.as_table_like_mut().into_iter().collect::<Vec<_>>()
            } else if k == "target" {
                v.as_table_like_mut()
                    .unwrap()
                    .iter_mut()
                    .flat_map(|(_, v)| {
                        v.as_table_like_mut().into_iter().flat_map(|v| {
                            v.iter_mut().filter_map(|(k, v)| {
                                if DEP_TABLES.contains(&k.get()) {
                                    v.as_table_like_mut()
                                } else {
                                    None
                                }
                            })
                        })
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        })
    }

    /// Override the manifest's version
    pub fn set_package_version(&mut self, version: &Version) {
        self.data["package"]["version"] = toml_edit::value(version.to_string());
    }

    /// Remove references to `dep_key` if its no longer present
    pub fn gc_dep(&mut self, dep_key: &str) {
        let status = self.dep_feature(dep_key);
        if matches!(status, FeatureStatus::None | FeatureStatus::DepFeature) {
            if let toml_edit::Item::Table(feature_table) = &mut self.data.as_table_mut()["features"]
            {
                for (_feature, mut activated_crates) in feature_table.iter_mut() {
                    if let toml_edit::Item::Value(toml_edit::Value::Array(feature_activations)) =
                        &mut activated_crates
                    {
                        remove_feature_activation(feature_activations, dep_key, status);
                    }
                }
            }
        }
    }

    fn dep_feature(&self, dep_key: &str) -> FeatureStatus {
        let mut status = FeatureStatus::None;
        for (_, tbl) in self.get_sections() {
            if let toml_edit::Item::Table(tbl) = tbl {
                if let Some(dep_item) = tbl.get(dep_key) {
                    let optional = dep_item.get("optional");
                    let optional = optional.and_then(|i| i.as_value());
                    let optional = optional.and_then(|i| i.as_bool());
                    let optional = optional.unwrap_or(false);
                    if optional {
                        return FeatureStatus::Feature;
                    } else {
                        status = FeatureStatus::DepFeature;
                    }
                }
            }
        }
        status
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum FeatureStatus {
    None,
    DepFeature,
    Feature,
}

fn remove_feature_activation(
    feature_activations: &mut toml_edit::Array,
    dep: &str,
    status: FeatureStatus,
) {
    let dep_feature: &str = &format!("{}/", dep);

    let remove_list: Vec<usize> = feature_activations
        .iter()
        .enumerate()
        .filter_map(|(idx, feature_activation)| {
            if let toml_edit::Value::String(feature_activation) = feature_activation {
                let activation = feature_activation.value();
                match status {
                    FeatureStatus::None => activation == dep || activation.starts_with(dep_feature),
                    FeatureStatus::DepFeature => activation == dep,
                    FeatureStatus::Feature => false,
                }
                .then(|| idx)
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

/// If a manifest is specified, return that one, otherise perform a manifest search starting from
/// the current directory.
/// If a manifest is specified, return that one. If a path is specified, perform a manifest search
/// starting from there. If nothing is specified, start searching from the current directory
/// (`cwd`).
pub fn find(specified: Option<&Path>) -> CargoResult<PathBuf> {
    match specified {
        Some(path)
            if fs::metadata(&path)
                .with_context(|| "Failed to get cargo file metadata")?
                .is_file() =>
        {
            Ok(path.to_owned())
        }
        Some(path) => search(path),
        None => search(&env::current_dir().with_context(|| "Failed to get current directory")?),
    }
}

/// Search for Cargo.toml in this directory and recursively up the tree until one is found.
fn search(dir: &Path) -> CargoResult<PathBuf> {
    let mut current_dir = dir;

    loop {
        let manifest = current_dir.join(MANIFEST_FILENAME);
        if fs::metadata(&manifest).is_ok() {
            return Ok(manifest);
        }

        current_dir = match current_dir.parent() {
            Some(current_dir) => current_dir,
            None => {
                anyhow::bail!("Unable to find Cargo.toml for {}", dir.display());
            }
        };
    }
}

fn get_version(old_dep: &toml_edit::Item) -> CargoResult<&str> {
    if let Some(req) = old_dep.as_str() {
        Ok(req)
    } else if old_dep.is_table_like() {
        let version = old_dep
            .get("version")
            .ok_or_else(|| anyhow::format_err!("Missing version field"))?;
        version
            .as_str()
            .ok_or_else(|| anyhow::format_err!("Expect version to be a string"))
    } else {
        unreachable!("Invalid old dependency type")
    }
}

fn old_version_compatible(dependency: &Dependency, old_version: &str) -> CargoResult<bool> {
    let old_version = VersionReq::parse(old_version)
        .with_context(|| parse_version_err(&dependency.name, old_version))?;

    let current_version = match dependency.version() {
        Some(current_version) => current_version,
        None => return Ok(false),
    };

    let current_version = Version::parse(current_version)
        .with_context(|| parse_version_err(&dependency.name, current_version))?;

    Ok(old_version.matches(&current_version))
}

pub fn str_or_1_len_table(item: &toml_edit::Item) -> bool {
    item.is_str() || item.as_table_like().map(|t| t.len() == 1).unwrap_or(false)
}

/// Print a message if the new dependency version is different from the old one.
fn print_upgrade_if_necessary(
    crate_name: &str,
    old_dep: &toml_edit::Item,
    new_dep: &toml_edit::Item,
) -> CargoResult<()> {
    let old_version = get_version(old_dep)?;
    let new_version = get_version(new_dep)?;

    if old_version == new_version {
        return Ok(());
    }

    let colorchoice = super::colorize_stderr();
    let bufwtr = BufferWriter::stderr(colorchoice);
    let mut buffer = bufwtr.buffer();
    buffer
        .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
        .with_context(|| "Failed to set output colour")?;
    write!(&mut buffer, "   Upgrading ").with_context(|| "Failed to write upgrade message")?;
    buffer
        .set_color(&ColorSpec::new())
        .with_context(|| "Failed to clear output colour")?;
    writeln!(
        &mut buffer,
        "{} v{} -> v{}",
        crate_name, old_version, new_version,
    )
    .with_context(|| "Failed to write upgrade versions")?;
    bufwtr
        .print(&buffer)
        .with_context(|| "Failed to print upgrade message")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::dependency::Dependency;
    use super::*;

    #[test]
    fn add_remove_dependency() {
        let root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let mut manifest = LocalManifest {
            path: root.join("Cargo.toml"),
            manifest: Manifest {
                data: toml_edit::Document::new(),
            },
        };
        let clone = manifest.clone();
        let dep = Dependency::new("cargo-edit").set_version("0.1.0");
        let _ = manifest.insert_into_table(&["dependencies".to_owned()], &dep);
        assert!(manifest
            .remove_from_table(&["dependencies".to_owned()], &dep.name)
            .is_ok());
        assert_eq!(manifest.data.to_string(), clone.data.to_string());
    }

    #[test]
    fn update_dependency() {
        let root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let mut manifest = LocalManifest {
            path: root.join("Cargo.toml"),
            manifest: Manifest {
                data: toml_edit::Document::new(),
            },
        };
        let dep = Dependency::new("cargo-edit").set_version("0.1.0");
        manifest
            .insert_into_table(&["dependencies".to_owned()], &dep)
            .unwrap();

        let new_dep = Dependency::new("cargo-edit").set_version("0.2.0");
        manifest
            .update_table_entry(&["dependencies".to_owned()], &new_dep, false)
            .unwrap();
    }

    #[test]
    fn update_wrong_dependency() {
        let root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let mut manifest = LocalManifest {
            path: root.join("Cargo.toml"),
            manifest: Manifest {
                data: toml_edit::Document::new(),
            },
        };
        let dep = Dependency::new("cargo-edit").set_version("0.1.0");
        manifest
            .insert_into_table(&["dependencies".to_owned()], &dep)
            .unwrap();
        let original = manifest.clone();

        let new_dep = Dependency::new("wrong-dep").set_version("0.2.0");
        manifest
            .update_table_entry(&["dependencies".to_owned()], &new_dep, false)
            .unwrap();

        assert_eq!(manifest.data.to_string(), original.data.to_string());
    }

    #[test]
    fn remove_dependency_no_section() {
        let root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let mut manifest = LocalManifest {
            path: root.join("Cargo.toml"),
            manifest: Manifest {
                data: toml_edit::Document::new(),
            },
        };
        let dep = Dependency::new("cargo-edit").set_version("0.1.0");
        assert!(manifest
            .remove_from_table(&["dependencies".to_owned()], &dep.name)
            .is_err());
    }

    #[test]
    fn remove_dependency_non_existent() {
        let root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let mut manifest = LocalManifest {
            path: root.join("Cargo.toml"),
            manifest: Manifest {
                data: toml_edit::Document::new(),
            },
        };
        let dep = Dependency::new("cargo-edit").set_version("0.1.0");
        let other_dep = Dependency::new("other-dep").set_version("0.1.0");
        assert!(manifest
            .insert_into_table(&["dependencies".to_owned()], &other_dep)
            .is_ok());
        assert!(manifest
            .remove_from_table(&["dependencies".to_owned()], &dep.name)
            .is_err());
    }

    #[test]
    fn set_package_version_overrides() {
        let original = r#"
[package]
name = "simple"
version = "0.1.0"
edition = "2015"

[dependencies]
"#;
        let expected = r#"
[package]
name = "simple"
version = "2.0.0"
edition = "2015"

[dependencies]
"#;
        let root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let mut manifest = LocalManifest {
            path: root.join("Cargo.toml"),
            manifest: original.parse::<Manifest>().unwrap(),
        };
        manifest.set_package_version(&semver::Version::parse("2.0.0").unwrap());
        let actual = manifest.to_string();

        assert_eq!(expected, actual);
    }

    #[test]
    fn old_version_is_compatible() -> CargoResult<()> {
        let with_version = Dependency::new("foo").set_version("2.3.4");
        assert!(!old_version_compatible(&with_version, "1")?);
        assert!(old_version_compatible(&with_version, "2")?);
        assert!(!old_version_compatible(&with_version, "3")?);
        Ok(())
    }

    #[test]
    fn old_incompatible_with_missing_new_version() -> CargoResult<()> {
        let no_version = Dependency::new("foo");
        assert!(!old_version_compatible(&no_version, "1")?);
        assert!(!old_version_compatible(&no_version, "2")?);
        Ok(())
    }

    #[test]
    fn old_incompatible_with_invalid() {
        let bad_version = Dependency::new("foo").set_version("CAKE CAKE");
        let good_version = Dependency::new("foo").set_version("1.2.3");
        assert!(old_version_compatible(&bad_version, "1").is_err());
        assert!(old_version_compatible(&good_version, "CAKE CAKE").is_err());
    }
}
