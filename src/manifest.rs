use std::fs::{self};
use std::io::Write;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::{env, str};

use semver::{Version, VersionReq};
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

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
    pub fn get_table<'a>(&'a mut self, table_path: &[String]) -> Result<&'a mut toml_edit::Item> {
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

        descend(&mut self.data.root, table_path)
    }

    /// Get all sections in the manifest that exist and might contain dependencies.
    /// The returned items are always `Table` or `InlineTable`.
    pub fn get_sections(&self) -> Vec<(Vec<String>, toml_edit::Item)> {
        let mut sections = Vec::new();

        for dependency_type in DEP_TABLES {
            // Dependencies can be in the three standard sections...
            if self.data[dependency_type].is_table_like() {
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
                    let dependency_table = &target_table[dependency_type];
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
        if self.manifest.data["package"].is_none() && self.manifest.data["project"].is_none() {
            if !self.manifest.data["workspace"].is_none() {
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

    /// Add entry to a Cargo.toml.
    pub fn insert_into_table(&mut self, table_path: &[String], dep: &Dependency) -> Result<()> {
        let (dep_key, new_dependency) =
            dep.to_toml(self.path.parent().expect("manifest path is absolute"));

        let table = self.get_table(table_path)?;
        let existing_dep = Self::find_dep(table, &dep.name);
        if let Some((old_dep_key, dep_item)) = existing_dep {
            // update an existing entry

            // if the `dep` is renamed in the `add` command,
            // but was present before, then we need to remove
            // the old entry and insert a new one
            // as the key has changed, e.g. from
            // a = "0.1"
            // to
            // alias = { version = "0.2", package = "a" }
            if let Some(renamed) = dep.rename() {
                table[renamed] = dep_item.clone();
                table[&old_dep_key] = toml_edit::Item::None;
            } else if dep_key != old_dep_key {
                // if `dep` had been renamed in the manifest,
                // and is not rename in the `add` command,
                // we need to remove the old entry and insert a new one
                // e.g. from
                // alias = { version = "0.1", package = "a" }
                // to
                // a = "0.2"
                table[&old_dep_key] = toml_edit::Item::None;
                table[&dep_key] = new_dependency.clone();
            }
            merge_dependencies(&mut table[&dep_key], new_dependency);
            if let Some(t) = table.as_inline_table_mut() {
                t.fmt()
            }
        } else {
            // insert a new entry
            table[dep_key] = new_dependency;
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
        self.update_table_named_entry(table_path, dep.name_in_manifest(), dep, dry_run)
    }

    /// Update an entry with a specified name in Cargo.toml.
    pub fn update_table_named_entry(
        &mut self,
        table_path: &[String],
        dep_key: &str,
        dep: &Dependency,
        dry_run: bool,
    ) -> Result<()> {
        let (_dep_key, new_dependency) =
            dep.to_toml(self.path.parent().expect("manifest path is absolute"));

        let table = self.get_table(table_path)?;

        // If (and only if) there is an old entry, merge the new one in.
        if !table[dep_key].is_none() {
            if let Err(e) = print_upgrade_if_necessary(&dep.name, &table[dep_key], &new_dependency)
            {
                eprintln!("Error while displaying upgrade message, {}", e);
            }
            if !dry_run {
                merge_dependencies(&mut table[dep_key], new_dependency);
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
    ///   assert!(manifest.data["dependencies"].is_none());
    /// ```
    pub fn remove_from_table(&mut self, table: &str, name: &str) -> Result<()> {
        if !self.data[table].is_table_like() {
            return Err(ErrorKind::NonExistentTable(table.into()).into());
        } else {
            {
                let dep = &mut self.data[table][name];
                if dep.is_none() {
                    return Err(ErrorKind::NonExistentDependency(name.into(), table.into()).into());
                }
                // remove the dependency
                *dep = toml_edit::Item::None;
            }

            // remove table if empty
            if self.data[table].as_table_like().unwrap().is_empty() {
                self.data[table] = toml_edit::Item::None;
            }
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

    /// Find a dependency by name (matching on package name for renamed deps)
    pub fn find_dep<'a>(
        table: &'a mut toml_edit::Item,
        dep_name: &'a str,
    ) -> Option<(String, &'a toml_edit::Item)> {
        table
            .as_table_like()
            .unwrap()
            .iter()
            .find(|&item| match item {
                (name, _) if name == dep_name => true,
                (_alias, toml_edit::Item::Table(table_dep))
                    if table_dep.contains_key("package") =>
                {
                    table_dep.get("package").unwrap().as_str() == Some(dep_name)
                }
                (_alias, toml_edit::Item::Value(toml_edit::Value::InlineTable(inline_dep)))
                    if inline_dep.contains_key("package") =>
                {
                    inline_dep.get("package").unwrap().as_str() == Some(dep_name)
                }
                _ => false,
            })
            .map(|dep| (dep.0.into(), dep.1))
    }

    /// Allow mutating depedencies, wherever they live
    pub fn get_dependency_tables_mut<'r>(
        &'r mut self,
    ) -> impl Iterator<Item = &mut dyn toml_edit::TableLike> + 'r {
        let root = self.data.as_table_mut();
        root.iter_mut().flat_map(|(k, v)| {
            if DEP_TABLES.contains(&k) {
                v.as_table_like_mut().into_iter().collect::<Vec<_>>()
            } else if k == "target" {
                v.as_table_like_mut()
                    .unwrap()
                    .iter_mut()
                    .flat_map(|(_, v)| {
                        v.as_table_like_mut().into_iter().flat_map(|v| {
                            v.iter_mut().filter_map(|(k, v)| {
                                if DEP_TABLES.contains(&k) {
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

/// Merge a new dependency into an old entry. See `Dependency::to_toml` for what the format of the
/// new dependency will be.
fn merge_dependencies(old_dep: &mut toml_edit::Item, new_toml: toml_edit::Item) {
    assert!(!old_dep.is_none());

    if str_or_1_len_table(old_dep) {
        // The old dependency is just a version/git/path. We are safe to overwrite.
        *old_dep = new_toml;
    } else if old_dep.is_table_like() {
        for key in &["version", "path", "git"] {
            // remove this key/value pairs
            old_dep[key] = toml_edit::Item::None;
        }
        if let Some(name) = new_toml.as_str() {
            old_dep["version"] = toml_edit::value(name);
        } else {
            merge_inline_table(old_dep, &new_toml);
        }
    } else {
        unreachable!("Invalid old dependency type");
    }

    if let Some(t) = old_dep.as_inline_table_mut() {
        t.fmt()
    }
}

fn merge_inline_table(old_dep: &mut toml_edit::Item, new: &toml_edit::Item) {
    for (k, v) in new
        .as_inline_table()
        .expect("expected an inline table")
        .iter()
    {
        old_dep[k] = toml_edit::value(v.clone());
    }
}

fn get_version(old_dep: &toml_edit::Item) -> Result<toml_edit::Item> {
    if str_or_1_len_table(old_dep) {
        Ok(old_dep.clone())
    } else if old_dep.is_table_like() {
        let version = old_dep["version"].clone();
        if version.is_none() {
            Err("Missing version field".into())
        } else {
            Ok(version)
        }
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

fn str_or_1_len_table(item: &toml_edit::Item) -> bool {
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
        let bufwtr = BufferWriter::stderr(ColorChoice::Always);
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
        let _ = manifest.insert_into_table(&["dependencies".to_owned()], &other_dep);
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
