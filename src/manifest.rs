use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::{env, str};

use semver::Version;

use super::errors::*;
use super::metadata::find_manifest_path;

#[derive(PartialEq, Eq, Hash, Ord, PartialOrd, Clone, Debug, Copy)]
pub enum DepKind {
    Normal,
    Development,
    Build,
}

/// Dependency table to add dep to
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepTable {
    kind: DepKind,
    target: Option<String>,
}

impl DepTable {
    const KINDS: &'static [Self] = &[
        Self::new().set_kind(DepKind::Normal),
        Self::new().set_kind(DepKind::Development),
        Self::new().set_kind(DepKind::Build),
    ];

    /// Reference to a Dependency Table
    pub(crate) const fn new() -> Self {
        Self {
            kind: DepKind::Normal,
            target: None,
        }
    }

    /// Choose the type of dependency
    pub(crate) const fn set_kind(mut self, kind: DepKind) -> Self {
        self.kind = kind;
        self
    }

    /// Choose the platform for the dependency
    pub(crate) fn set_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    fn kind_table(&self) -> &str {
        match self.kind {
            DepKind::Normal => "dependencies",
            DepKind::Development => "dev-dependencies",
            DepKind::Build => "build-dependencies",
        }
    }
}

impl Default for DepTable {
    fn default() -> Self {
        Self::new()
    }
}

impl From<DepKind> for DepTable {
    fn from(other: DepKind) -> Self {
        Self::new().set_kind(other)
    }
}

/// A Cargo manifest
#[derive(Debug, Clone)]
pub struct Manifest {
    /// Manifest contents as TOML data
    pub data: toml_edit::Document,
}

impl Manifest {
    /// Get the specified table from the manifest.
    ///
    /// If there is no table at the specified path, then a non-existent table
    /// error will be returned.
    pub(crate) fn get_table_mut<'a>(
        &'a mut self,
        table_path: &[String],
    ) -> CargoResult<&'a mut toml_edit::Item> {
        self.get_table_mut_internal(table_path, false)
    }

    /// Get all sections in the manifest that exist and might contain dependencies.
    /// The returned items are always `Table` or `InlineTable`.
    pub(crate) fn get_sections(&self) -> Vec<(DepTable, toml_edit::Item)> {
        let mut sections = Vec::new();

        for table in DepTable::KINDS {
            let dependency_type = table.kind_table();
            // Dependencies can be in the three standard sections...
            if self
                .data
                .get(dependency_type)
                .map(|t| t.is_table_like())
                .unwrap_or(false)
            {
                sections.push((table.clone(), self.data[dependency_type].clone()))
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
                            table.clone().set_target(target_name),
                            dependency_table.clone(),
                        )
                    })
                });

            sections.extend(target_sections);
        }

        sections
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
    type Err = anyhow::Error;

    /// Read manifest data from string
    fn from_str(input: &str) -> ::std::result::Result<Self, Self::Err> {
        let d: toml_edit::Document = input.parse().context("Manifest not valid TOML")?;

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
        if !path.is_absolute() {
            anyhow::bail!("can only edit absolute paths, got {}", path.display());
        }
        let data =
            std::fs::read_to_string(&path).with_context(|| "Failed to read manifest contents")?;
        let manifest = data.parse().context("Unable to parse Cargo.toml")?;
        Ok(LocalManifest {
            manifest,
            path: path.to_owned(),
        })
    }

    /// Write changes back to the file
    pub fn write(&self) -> CargoResult<()> {
        let s = self.manifest.data.to_string();
        let new_contents_bytes = s.as_bytes();

        std::fs::write(&self.path, new_contents_bytes).context("Failed to write updated Cargo.toml")
    }

    /// Remove entry from a Cargo.toml.
    ///
    /// # Examples
    ///
    /// ```
    ///   use cargo_edit::{Dependency, LocalManifest, Manifest, RegistrySource};
    ///   use toml_edit;
    ///
    ///   let root = std::path::PathBuf::from("/").canonicalize().unwrap();
    ///   let path = root.join("Cargo.toml");
    ///   let manifest: toml_edit::Document = "
    ///   [dependencies]
    ///   cargo-edit = '0.1.0'
    ///   ".parse().unwrap();
    ///   let mut manifest = LocalManifest { path, manifest: Manifest { data: manifest } };
    ///   assert!(manifest.remove_from_table(&["dependencies".to_owned()], "cargo-edit").is_ok());
    ///   assert!(manifest.remove_from_table(&["dependencies".to_owned()], "cargo-edit").is_err());
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

    /// Allow mutating depedencies, wherever they live
    pub fn get_dependency_tables_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut dyn toml_edit::TableLike> + '_ {
        let root = self.data.as_table_mut();
        root.iter_mut().flat_map(|(k, v)| {
            if DepTable::KINDS
                .iter()
                .any(|kind| kind.kind_table() == k.get())
            {
                v.as_table_like_mut().into_iter().collect::<Vec<_>>()
            } else if k == "workspace" {
                v.as_table_like_mut()
                    .unwrap()
                    .iter_mut()
                    .filter_map(|(k, v)| {
                        if k.get() == "dependencies" {
                            v.as_table_like_mut()
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            } else if k == "target" {
                v.as_table_like_mut()
                    .unwrap()
                    .iter_mut()
                    .flat_map(|(_, v)| {
                        v.as_table_like_mut().into_iter().flat_map(|v| {
                            v.iter_mut().filter_map(|(k, v)| {
                                if DepTable::KINDS
                                    .iter()
                                    .any(|kind| kind.kind_table() == k.get())
                                {
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

    /// Iterates mutably over the `[workspace.dependencies]`.
    pub fn get_workspace_dependency_table_mut(&mut self) -> Option<&mut dyn toml_edit::TableLike> {
        self.data
            .get_mut("workspace")?
            .get_mut("dependencies")?
            .as_table_like_mut()
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
                #[allow(clippy::unnecessary_lazy_evaluations)] // requires 1.62
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
        Some(path) => find_manifest_path(path),
        None => find_manifest_path(
            &env::current_dir().with_context(|| "Failed to get current directory")?,
        ),
    }
}

/// Get a dependency's version from its entry in the dependency table
pub fn get_dep_version(dep_item: &toml_edit::Item) -> CargoResult<&str> {
    if let Some(req) = dep_item.as_str() {
        Ok(req)
    } else if dep_item.is_table_like() {
        let version = dep_item
            .get("version")
            .ok_or_else(|| anyhow::format_err!("Missing version field"))?;
        version
            .as_str()
            .ok_or_else(|| anyhow::format_err!("Expect version to be a string"))
    } else {
        anyhow::bail!("Invalid dependency type");
    }
}

/// Set a dependency's version in its entry in the dependency table
pub fn set_dep_version(dep_item: &mut toml_edit::Item, new_version: &str) -> CargoResult<()> {
    if dep_item.is_str() {
        overwrite_value(dep_item, new_version);
    } else if let Some(table) = dep_item.as_table_like_mut() {
        let version = table
            .get_mut("version")
            .ok_or_else(|| anyhow::format_err!("Missing version field"))?;
        overwrite_value(version, new_version);
    } else {
        anyhow::bail!("Invalid dependency type");
    }
    Ok(())
}

/// Overwrite a value while preserving the original formatting
fn overwrite_value(item: &mut toml_edit::Item, value: impl Into<toml_edit::Value>) {
    let mut value = value.into();

    let existing_decor = item
        .as_value()
        .map(|v| v.decor().clone())
        .unwrap_or_default();

    *value.decor_mut() = existing_decor;

    *item = toml_edit::Item::Value(value);
}

pub fn str_or_1_len_table(item: &toml_edit::Item) -> bool {
    item.is_str() || item.as_table_like().map(|t| t.len() == 1).unwrap_or(false)
}
