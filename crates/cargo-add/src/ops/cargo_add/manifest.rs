use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::str;

use super::dependency::Dependency;
use super::errors::*;

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
    pub fn get_table_mut<'a>(
        &'a mut self,
        table_path: &[String],
    ) -> CargoResult<&'a mut toml_edit::Item> {
        /// Descend into a manifest until the required table is found.
        fn descend<'a>(
            input: &'a mut toml_edit::Item,
            path: &[String],
        ) -> CargoResult<&'a mut toml_edit::Item> {
            if let Some(segment) = path.get(0) {
                let value = input[&segment].or_insert(toml_edit::table());

                if value.is_table_like() {
                    descend(value, &path[1..])
                } else {
                    Err(non_existent_table_err(segment))
                }
            } else {
                Ok(input)
            }
        }

        descend(self.data.as_item_mut(), table_path)
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
    /// Construct the `LocalManifest` corresponding to the `Path` provided.
    pub fn try_new(path: &Path) -> CargoResult<Self> {
        let path = dunce::canonicalize(path).with_context(|| "Failed to read manifest contents")?;
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

        let table = self.get_table_mut(table_path)?;
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

pub fn str_or_1_len_table(item: &toml_edit::Item) -> bool {
    item.is_str() || item.as_table_like().map(|t| t.len() == 1).unwrap_or(false)
}
