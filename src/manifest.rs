use std::fs::{self};
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::{env, str};

use semver::{Version, VersionReq};
use termcolor::{BufferWriter, Color, ColorSpec, WriteColor};

use crate::dependency::Dependency;
use crate::errors::*;

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
    pub fn package_name(&self) -> Result<&str> {
        self.data
            .as_table()
            .get("package")
            .and_then(|m| m["name"].as_str())
            .ok_or_else(|| ErrorKind::ParseCargoToml.into())
    }

    /// Get the specified table from the manifest.
    pub fn get_table<'a>(&'a self, table_path: &[String]) -> Result<&'a toml_edit::Item> {
        /// Descend into a manifest until the required table is found.
        fn descend<'a>(input: &'a toml_edit::Item, path: &[String]) -> Result<&'a toml_edit::Item> {
            if let Some(segment) = path.get(0) {
                let value = input
                    .get(&segment)
                    .ok_or_else(|| ErrorKind::NonExistentTable(segment.clone()))?;

                if value.is_table_like() {
                    descend(value, &path[1..])
                } else {
                    Err(ErrorKind::NonExistentTable(segment.clone()).into())
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
    ) -> Result<&'a mut toml_edit::Item> {
        /// Descend into a manifest until the required table is found.
        fn descend<'a>(
            input: &'a mut toml_edit::Item,
            path: &[String],
        ) -> Result<&'a mut toml_edit::Item> {
            if let Some(segment) = path.get(0) {
                let value = input[&segment].or_insert(toml_edit::table());

                if value.is_table_like() {
                    descend(value, &path[1..])
                } else {
                    Err(ErrorKind::NonExistentTable(segment.clone()).into())
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
    pub fn features(&self) -> Result<Vec<String>> {
        let mut features: Vec<String> = match self.data.as_table().get("features") {
            None => vec![],
            Some(item) => match item {
                toml_edit::Item::None => vec![],
                toml_edit::Item::Table(t) => t
                    .get_values()
                    .iter()
                    .map(|(keys, _val)| keys.iter().map(|&k| k.get().trim().to_owned()))
                    .flatten()
                    .collect(),
                _ => return Err(ErrorKind::InvalidCargoConfig.into()),
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
                            .then(|| key.to_owned())
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
        let d: toml_edit::Document = input.parse().chain_err(|| "Manifest not valid TOML")?;

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
    pub fn find(path: &Option<PathBuf>) -> Result<Self> {
        let path = dunce::canonicalize(find(path)?)?;
        Self::try_new(&path)
    }

    /// Construct the `LocalManifest` corresponding to the `Path` provided.
    pub fn try_new(path: &Path) -> Result<Self> {
        let path = path.to_path_buf();
        let data =
            std::fs::read_to_string(&path).chain_err(|| "Failed to read manifest contents")?;
        let manifest = data.parse().chain_err(|| "Unable to parse Cargo.toml")?;
        Ok(LocalManifest { manifest, path })
    }

    /// Write changes back to the file
    pub fn write(&self) -> Result<()> {
        if !self.manifest.data.contains_key("package")
            && !self.manifest.data.contains_key("project")
        {
            if self.manifest.data.contains_key("workspace") {
                return Err(ErrorKind::UnexpectedRootManifest.into());
            } else {
                return Err(ErrorKind::InvalidManifest.into());
            }
        }

        let s = self.manifest.data.to_string();
        let new_contents_bytes = s.as_bytes();

        std::fs::write(&self.path, new_contents_bytes)
            .chain_err(|| "Failed to write updated Cargo.toml")
    }

    /// Instruct this manifest to upgrade a single dependency. If this manifest does not have that
    /// dependency, it does nothing.
    pub fn upgrade(
        &mut self,
        dependency: &Dependency,
        dry_run: bool,
        skip_compatible: bool,
    ) -> Result<()> {
        for (table_path, table) in self.get_sections() {
            let table_like = table.as_table_like().expect("Unexpected non-table");
            for (name, toml_item) in table_like.iter() {
                let dep_name = toml_item
                    .as_table_like()
                    .and_then(|t| t.get("package").and_then(|p| p.as_str()))
                    .unwrap_or(name);
                if dep_name == dependency.name {
                    if skip_compatible {
                        if let Some(old_version) = get_version(toml_item)?.as_str() {
                            if old_version_compatible(dependency, old_version)? {
                                continue;
                            }
                        }
                    }
                    self.update_table_named_entry(&table_path, name, dependency, dry_run)?;
                }
            }
        }

        self.write()
    }

    /// Lookup a depednency
    pub fn get_dependency(&self, table_path: &[String], dep_key: &str) -> Result<Dependency> {
        let crate_root = self.path.parent().expect("manifest path is absolute");
        let table = self.get_table(table_path)?;
        let table = table
            .as_table_like()
            .ok_or_else(|| ErrorKind::NonExistentTable(table_path.join(".")))?;
        let dep_item = table.get(dep_key).ok_or_else(|| {
            ErrorKind::NonExistentDependency(dep_key.into(), table_path.join("."))
        })?;
        Dependency::from_toml(crate_root, dep_key, dep_item).ok_or_else(|| {
            format!("Invalid dependency {}.{}", table_path.join("."), dep_key).into()
        })
    }

    /// Add entry to a Cargo.toml.
    pub fn insert_into_table(&mut self, table_path: &[String], dep: &Dependency) -> Result<()> {
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

    /// Update an entry in Cargo.toml.
    pub fn update_table_entry(
        &mut self,
        table_path: &[String],
        dep: &Dependency,
        dry_run: bool,
    ) -> Result<()> {
        self.update_table_named_entry(table_path, dep.toml_key(), dep, dry_run)
    }

    /// Update an entry with a specified name in Cargo.toml.
    pub fn update_table_named_entry(
        &mut self,
        table_path: &[String],
        dep_key: &str,
        dep: &Dependency,
        dry_run: bool,
    ) -> Result<()> {
        let crate_root = self
            .path
            .parent()
            .expect("manifest path is absolute")
            .to_owned();
        let new_dependency = dep.to_toml(self.path.parent().expect("manifest path is absolute"));

        let table = self.get_table_mut(table_path)?;

        // If (and only if) there is an old entry, merge the new one in.
        if table.as_table_like().unwrap().contains_key(dep_key) {
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
    ///   assert!(manifest.remove_from_table("dependencies", &dep.name).is_ok());
    ///   assert!(manifest.remove_from_table("dependencies", &dep.name).is_err());
    ///   assert!(!manifest.data.contains_key("dependencies"));
    /// ```
    pub fn remove_from_table(&mut self, table: &str, name: &str) -> Result<()> {
        let parent_table = self
            .data
            .get_mut(table)
            .filter(|t| t.is_table_like())
            .ok_or_else(|| ErrorKind::NonExistentTable(table.into()))?;

        {
            let dep = parent_table
                .get_mut(name)
                .filter(|t| !t.is_none())
                .ok_or_else(|| ErrorKind::NonExistentDependency(name.into(), table.into()))?;
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
    pub fn add_deps(&mut self, table: &[String], deps: &[Dependency]) -> Result<()> {
        deps.iter()
            .map(|dep| self.insert_into_table(table, dep))
            .collect::<Result<Vec<_>>>()?;

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
pub fn find(specified: &Option<PathBuf>) -> Result<PathBuf> {
    match *specified {
        Some(ref path)
            if fs::metadata(&path)
                .chain_err(|| "Failed to get cargo file metadata")?
                .is_file() =>
        {
            Ok(path.to_owned())
        }
        Some(ref path) => search(path),
        None => search(&env::current_dir().chain_err(|| "Failed to get current directory")?),
    }
}

/// Search for Cargo.toml in this directory and recursively up the tree until one is found.
fn search(dir: &Path) -> Result<PathBuf> {
    let manifest = dir.join(MANIFEST_FILENAME);

    if fs::metadata(&manifest).is_ok() {
        Ok(manifest)
    } else {
        dir.parent()
            .ok_or_else(|| ErrorKind::MissingManifest.into())
            .and_then(search)
    }
}

fn get_version(old_dep: &toml_edit::Item) -> Result<toml_edit::Item> {
    if str_or_1_len_table(old_dep) {
        Ok(old_dep.clone())
    } else if old_dep.is_table_like() {
        let version = old_dep.get("version").ok_or("Missing version field")?;
        Ok(version.clone())
    } else {
        unreachable!("Invalid old dependency type")
    }
}

fn old_version_compatible(dependency: &Dependency, old_version: &str) -> Result<bool> {
    let old_version = VersionReq::parse(old_version).chain_err(|| {
        ErrorKind::ParseVersion(dependency.name.to_string(), old_version.to_string())
    })?;

    let current_version = match dependency.version() {
        Some(current_version) => current_version,
        None => return Ok(false),
    };

    let current_version = Version::parse(current_version).chain_err(|| {
        ErrorKind::ParseVersion(dependency.name.to_string(), current_version.into())
    })?;

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
) -> Result<()> {
    let old_version = get_version(old_dep)?;
    let new_version = get_version(new_dep)?;

    if let (Some(old_version), Some(new_version)) = (old_version.as_str(), new_version.as_str()) {
        if old_version == new_version {
            return Ok(());
        }
        let colorchoice = crate::colorize_stderr();
        let bufwtr = BufferWriter::stderr(colorchoice);
        let mut buffer = bufwtr.buffer();
        buffer
            .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
            .chain_err(|| "Failed to set output colour")?;
        write!(&mut buffer, "    Upgrading ").chain_err(|| "Failed to write upgrade message")?;
        buffer
            .set_color(&ColorSpec::new())
            .chain_err(|| "Failed to clear output colour")?;
        writeln!(
            &mut buffer,
            "{} v{} -> v{}",
            crate_name, old_version, new_version,
        )
        .chain_err(|| "Failed to write upgrade versions")?;
        bufwtr
            .print(&buffer)
            .chain_err(|| "Failed to print upgrade message")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency::Dependency;

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
            .remove_from_table("dependencies", &dep.name)
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
            .remove_from_table("dependencies", &dep.name)
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
            .remove_from_table("dependencies", &dep.name)
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
    fn old_version_is_compatible() -> Result<()> {
        let with_version = Dependency::new("foo").set_version("2.3.4");
        assert!(!old_version_compatible(&with_version, "1")?);
        assert!(old_version_compatible(&with_version, "2")?);
        assert!(!old_version_compatible(&with_version, "3")?);
        Ok(())
    }

    #[test]
    fn old_incompatible_with_missing_new_version() -> Result<()> {
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
