use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::{env, str};

use serde::Serialize;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};
use toml;

use errors::*;
use dependency::Dependency;

const MANIFEST_FILENAME: &str = "Cargo.toml";

/// A Cargo Manifest
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    /// Manifest contents as TOML data
    pub data: toml::value::Table,
}

fn toml_pretty(value: &toml::Value) -> Result<String> {
    let mut out = String::new();
    {
        let mut ser = toml::Serializer::pretty(&mut out);
        ser.pretty_string_literal(false);
        value
            .serialize(&mut ser)
            .chain_err(|| "Failed to serialize new Cargo.toml contents")?;
    }
    Ok(out)
}

/// If a manifest is specified, return that one, otherise perform a manifest search starting from
/// the current directory.
/// If a manifest is specified, return that one. If a path is specified, perform a manifest search
/// starting from there. If nothing is specified, start searching from the current directory
/// (`cwd`).
fn find(specified: &Option<PathBuf>) -> Result<PathBuf> {
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
            .and_then(|dir| search(dir))
    }
}

/// Merge a new dependency into an old entry. See `Dependency::to_toml` for what the format of the
/// new dependency will be.
fn merge_dependencies(old_dep: &mut toml::value::Value, new: &Dependency) {
    let mut new_toml = new.to_toml().1;

    if old_dep.is_str() || old_dep.as_table().map(|o| o.len() == 1).unwrap_or(false) {
        // The old dependency is just a version/git/path. We are safe to overwrite.
        ::std::mem::replace(old_dep, new_toml);
    } else if let Some(old) = old_dep.as_table_mut() {
        // Get rid of the old version field, whatever form that takes.
        old.remove("version");
        old.remove("path");
        old.remove("git");

        // Overwrite update the old dependency with the relevant fields from the new one.
        match new_toml {
            toml::Value::Table(ref mut n) => old.append(n),
            v @ toml::Value::String(_) => {
                // The new dependency is only a string if it is a plain version.
                old.insert("version".to_string(), v.to_owned());
            }
            n => unreachable!("Invalid new dependency type: {:?}", n),
        }
    } else {
        unreachable!("Invalid old dependency type");
    }
}

/// Print a message if the new dependency version is different from the old one.
fn print_upgrade_if_necessary(
    crate_name: &str,
    old_dep: &toml::Value,
    new_version: &toml::Value,
) -> Result<()> {
    let old_version =
        if old_dep.is_str() || old_dep.as_table().map(|o| o.len() == 1).unwrap_or(false) {
            old_dep.clone()
        } else if let Some(old) = old_dep.as_table() {
            if let Some(old_dep) = old.clone().remove("version") {
                old_dep
            } else {
                return Err("Missing version field".into());
            }
        } else {
            unreachable!("Invalid old dependency type");
        };

    if let (Some(old_version), Some(new_version)) = (old_version.as_str(), new_version.as_str()) {
        if old_version == new_version {
            return Ok(());
        }
        let bufwtr = BufferWriter::stdout(ColorChoice::Always);
        let mut buffer = bufwtr.buffer();
        buffer
            .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))
            .chain_err(|| "Failed to set output colour")?;
        write!(&mut buffer, "Upgrading ").chain_err(|| "Failed to write upgrade message")?;
        buffer
            .set_color(&ColorSpec::new())
            .chain_err(|| "Failed to clear output colour")?;
        write!(
            &mut buffer,
            "{} v{} -> v{}\n",
            crate_name,
            old_version,
            new_version,
        ).chain_err(|| "Failed to write upgrade versions")?;
        bufwtr
            .print(&buffer)
            .chain_err(|| "Failed to print upgrade message")?;
    }
    Ok(())
}

impl Manifest {
    /// Look for a `Cargo.toml` file
    ///
    /// Starts at the given path an goes into its parent directories until the manifest file is
    /// found. If no path is given, the process's working directory is used as a starting point.
    pub fn find_file(path: &Option<PathBuf>) -> Result<File> {
        find(path).and_then(|path| {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .chain_err(|| "Failed to find Cargo.toml")
        })
    }

    /// Open the `Cargo.toml` for a path (or the process' `cwd`)
    pub fn open(path: &Option<PathBuf>) -> Result<Manifest> {
        let mut file = Manifest::find_file(path)?;
        let mut data = String::new();
        file.read_to_string(&mut data)
            .chain_err(|| "Failed to read manifest contents")?;

        data.parse().chain_err(|| "Unable to parse Cargo.toml")
    }

    /// Get the specified table from the manifest.
    pub fn get_table<'a>(
        &'a mut self,
        table_path: &[String],
    ) -> Result<&'a mut BTreeMap<String, toml::Value>> {
        /// Descend into a manifest until the required table is found.
        fn descend<'a>(
            input: &'a mut BTreeMap<String, toml::Value>,
            path: &[String],
        ) -> Result<&'a mut BTreeMap<String, toml::Value>> {
            if let Some(segment) = path.get(0) {
                let value = input
                    .entry(segment.to_owned())
                    .or_insert_with(|| toml::Value::Table(BTreeMap::new()));

                match *value {
                    toml::Value::Table(ref mut table) => descend(table, &path[1..]),
                    _ => Err(ErrorKind::NonExistentTable(segment.clone()).into()),
                }
            } else {
                Ok(input)
            }
        }

        descend(&mut self.data, table_path)
    }

    /// Get all sections in the manifest that exist and might contain dependencies.
    pub fn get_sections(&self) -> Vec<(Vec<String>, BTreeMap<String, toml::Value>)> {
        let mut sections = Vec::new();

        for dependency_type in &["dev-dependencies", "build-dependencies", "dependencies"] {
            // Dependencies can be in the three standard sections...
            self.data
                .get(&dependency_type.to_string())
                .and_then(toml::Value::as_table)
                .map(|table| sections.push((vec![dependency_type.to_string()], table.clone())));

            // ... and in `target.<target>.(build-/dev-)dependencies`.
            let target_sections = self.data
                .get("target")
                .and_then(toml::Value::as_table)
                .into_iter()
                .flat_map(|target_tables| target_tables.into_iter())
                .filter_map(|(target_name, target_table)| {
                    target_table
                        .get(dependency_type)
                        .and_then(toml::Value::as_table)
                        .map(|dependency_table| {
                            (
                                vec![
                                    "target".to_string(),
                                    target_name.to_string(),
                                    dependency_type.to_string(),
                                ],
                                dependency_table.to_owned(),
                            )
                        })
                });

            sections.extend(target_sections);
        }

        sections
    }

    /// Overwrite a file with TOML data.
    pub fn write_to_file(&self, file: &mut File) -> Result<()> {
        let mut toml = self.data.clone();

        let (proj_header, proj_data) = toml.remove("package")
            .map(|data| ("package", data))
            .or_else(|| toml.remove("project").map(|data| ("project", data)))
            .ok_or_else(|| {
                if toml.contains_key("workspace") {
                    ErrorKind::UnexpectedRootManifest
                } else {
                    ErrorKind::InvalidManifest
                }
            })?;

        let new_contents = format!(
            "[{}]\n{}\n{}",
            proj_header,
            toml_pretty(&proj_data)?,
            toml_pretty(&toml::Value::Table(toml))?,
        );
        let new_contents_bytes = new_contents.as_bytes();

        // We need to truncate the file, otherwise the new contents
        // will be mixed up with the old ones.
        file.set_len(new_contents_bytes.len() as u64)
            .chain_err(|| "Failed to truncate Cargo.toml")?;
        file.write_all(new_contents_bytes)
            .chain_err(|| "Failed to write updated Cargo.toml")
    }

    /// Add entry to a Cargo.toml.
    pub fn insert_into_table(&mut self, table_path: &[String], dep: &Dependency) -> Result<()> {
        let table = self.get_table(table_path)?;

        table
            .get_mut(&dep.name)
            // If there exists an old entry, update it.
            .map(|old_dependency| merge_dependencies(old_dependency, dep))
            // Otherwise insert.
            .unwrap_or_else(|| {
                let (ref name, ref mut new_dependency) = dep.to_toml();

                table.insert(name.clone(), new_dependency.clone());
            });

        Ok(())
    }

    /// Update an entry in Cargo.toml.
    pub fn update_table_entry(&mut self, table_path: &[String], dep: &Dependency) -> Result<()> {
        let table = self.get_table(table_path)?;
        let new_dep = dep.to_toml().1;

        // If (and only if) there is an old entry, merge the new one in.
        let old_dependency = table.get_mut(&dep.name);
        if let Some(old_dependency) = old_dependency {
            if let Err(e) = print_upgrade_if_necessary(&dep.name, old_dependency, &new_dep) {
                eprintln!("Error while displaying upgrade message, {}", e);
            }
            merge_dependencies(old_dependency, dep);
        }

        Ok(())
    }

    /// Remove entry from a Cargo.toml.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate cargo_edit;
    /// # extern crate toml;
    /// # fn main() {
    ///     use cargo_edit::{Dependency, Manifest};
    ///     use toml;
    ///
    ///     let mut manifest = Manifest { data: toml::value::Table::new() };
    ///     let dep = Dependency::new("cargo-edit").set_version("0.1.0");
    ///     let _ = manifest.insert_into_table(&vec!["dependencies".to_owned()], &dep);
    ///     assert!(manifest.remove_from_table("dependencies", &dep.name).is_ok());
    ///     assert!(manifest.remove_from_table("dependencies", &dep.name).is_err());
    ///     assert!(manifest.data.is_empty());
    /// # }
    /// ```
    pub fn remove_from_table(&mut self, table: &str, name: &str) -> Result<()> {
        let manifest = &mut self.data;
        let entry = manifest.entry(String::from(table));

        match entry {
            Entry::Vacant(_) => Err(ErrorKind::NonExistentTable(table.into())),
            Entry::Occupied(mut section) => {
                let result = match *section.get_mut() {
                    toml::Value::Table(ref mut deps) => deps.remove(name)
                        .map(|_| ())
                        .ok_or_else(|| ErrorKind::NonExistentDependency(name.into(), table.into())),
                    _ => Err(ErrorKind::NonExistentTable(table.into())),
                };
                if section.get().as_table().map(|x| x.is_empty()) == Some(true) {
                    section.remove();
                }
                result
            }
        }?;

        Ok(())
    }

    /// Add multiple dependencies to manifest
    pub fn add_deps(&mut self, table: &[String], deps: &[Dependency]) -> Result<()> {
        deps.iter()
            .map(|dep| self.insert_into_table(table, dep))
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }
}

impl str::FromStr for Manifest {
    type Err = Error;

    /// Read manifest data from string
    fn from_str(input: &str) -> ::std::result::Result<Self, Self::Err> {
        let d: toml::value::Value = input.parse().chain_err(|| "Manifest not valid TOML")?;
        let e = d.as_table()
            .ok_or_else(|| ErrorKind::NonExistentTable(String::from("Main")))?;

        Ok(Manifest { data: e.to_owned() })
    }
}

#[cfg(test)]
mod tests {
    use dependency::Dependency;
    use super::*;
    use toml;

    #[test]
    fn add_remove_dependency() {
        let mut manifest = Manifest {
            data: toml::value::Table::new(),
        };
        let clone = manifest.clone();
        let dep = Dependency::new("cargo-edit").set_version("0.1.0");
        let _ = manifest.insert_into_table(&["dependencies".to_owned()], &dep);
        assert!(
            manifest
                .remove_from_table("dependencies", &dep.name)
                .is_ok()
        );
        assert_eq!(manifest, clone);
    }

    #[test]
    fn update_dependency() {
        let mut manifest = Manifest {
            data: toml::value::Table::new(),
        };
        let dep = Dependency::new("cargo-edit").set_version("0.1.0");
        manifest
            .insert_into_table(&["dependencies".to_owned()], &dep)
            .unwrap();

        let new_dep = Dependency::new("cargo-edit").set_version("0.2.0");
        manifest
            .update_table_entry(&["dependencies".to_owned()], &new_dep)
            .unwrap();
    }

    #[test]
    fn update_wrong_dependency() {
        let mut manifest = Manifest {
            data: toml::value::Table::new(),
        };
        let dep = Dependency::new("cargo-edit").set_version("0.1.0");
        manifest
            .insert_into_table(&["dependencies".to_owned()], &dep)
            .unwrap();
        let original = manifest.clone();

        let new_dep = Dependency::new("wrong-dep").set_version("0.2.0");
        manifest
            .update_table_entry(&["dependencies".to_owned()], &new_dep)
            .unwrap();

        assert_eq!(manifest, original);
    }

    #[test]
    fn remove_dependency_no_section() {
        let mut manifest = Manifest {
            data: toml::value::Table::new(),
        };
        let dep = Dependency::new("cargo-edit").set_version("0.1.0");
        assert!(
            manifest
                .remove_from_table("dependencies", &dep.name)
                .is_err()
        );
    }

    #[test]
    fn remove_dependency_non_existent() {
        let mut manifest = Manifest {
            data: toml::value::Table::new(),
        };
        let dep = Dependency::new("cargo-edit").set_version("0.1.0");
        let other_dep = Dependency::new("other-dep").set_version("0.1.0");
        let _ = manifest.insert_into_table(&["dependencies".to_owned()], &other_dep);
        assert!(
            manifest
                .remove_from_table("dependencies", &dep.name)
                .is_err()
        );
    }
}
