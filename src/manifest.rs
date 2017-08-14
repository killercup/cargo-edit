use dependency::Dependency;
use std::{env, str};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use toml;
use serde::Serialize;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

/// Enumeration of errors which can occur when working with a rust manifest.
quick_error! {
    #[derive(Debug)]
    pub enum ManifestError {
        /// Cargo.toml could not be found.
        MissingManifest {
            description("missing manifest")
            display("Your Cargo.toml is missing.")
        }
        /// The TOML table could not be found.
        NonExistentTable(table: String) {
            description("non existent table")
            display("The table `{}` could not be found.", table)
        }
        /// The dependency could not be found.
        NonExistentDependency(name: String, table: String) {
            description("non existent dependency")
            display("The dependency `{}` could not be found in `{}`.", name, table)
        }
        ParseError(error: String, loline: usize, locol: usize, hiline: usize, hicol: usize) {
            description("parse error")
            display("{line}:{col}{upto} {error_msg}",
                line = loline + 1,
                col = locol + 1,
                upto = if loline != hiline || locol != hicol {
                    format!("-{}:{}", hiline + 1, hicol + 1)
                } else {
                    "".to_string()
                },
                error_msg = error)
        }
    }
}

enum CargoFile {
    Config,
    Lock,
}

impl CargoFile {
    fn name(&self) -> &str {
        match *self {
            CargoFile::Config => "Cargo.toml",
            CargoFile::Lock => "Cargo.lock",
        }
    }
}

/// A Cargo Manifest
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    /// Manifest contents as TOML data
    pub data: toml::value::Table,
}

fn toml_pretty(value: &toml::Value) -> Result<String, Box<Error>> {
    let mut out = String::new();
    {
        let mut ser = toml::Serializer::pretty(&mut out);
        ser.pretty_string_literal(false);
        value.serialize(&mut ser)?;
    }
    Ok(out)
}

/// If a manifest is specified, return that one, otherise perform a manifest search starting from
/// the current directory.
/// If a manifest is specified, return that one. If a path is specified, perform a manifest search
/// starting from there. If nothing is specified, start searching from the current directory
/// (`cwd`).
fn find(specified: &Option<PathBuf>, file: CargoFile) -> Result<PathBuf, Box<Error>> {
    match *specified {
        Some(ref path) if fs::metadata(&path)?.is_file() => Ok(path.to_owned()),
        Some(ref path) => search(path, file),
        None => search(&env::current_dir()?, file),
    }.map_err(From::from)
}

/// Search for Cargo.toml in this directory and recursively up the tree until one is found.
fn search(dir: &Path, file: CargoFile) -> Result<PathBuf, ManifestError> {
    let manifest = dir.join(file.name());

    if fs::metadata(&manifest).is_ok() {
        Ok(manifest)
    } else {
        dir.parent()
            .ok_or(ManifestError::MissingManifest)
            .and_then(|dir| search(dir, file))
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
) -> Result<(), Box<Error>> {
    let old_version =
        if old_dep.is_str() || old_dep.as_table().map(|o| o.len() == 1).unwrap_or(false) {
            old_dep.clone()
        } else if let Some(old) = old_dep.as_table() {
            if let Some(old_dep) = old.clone().remove("version") {
                old_dep
            } else {
                return Err(From::from("Missing version field"));
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
            .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
        write!(&mut buffer, "Upgrading ")?;
        buffer.set_color(&ColorSpec::new())?;
        write!(
            &mut buffer,
            "{} v{} -> v{}\n",
            crate_name,
            old_version,
            new_version,
        )?;
        bufwtr.print(&buffer)?;
    }
    Ok(())
}

impl Manifest {
    /// Look for a `Cargo.toml` file
    ///
    /// Starts at the given path an goes into its parent directories until the manifest file is
    /// found. If no path is given, the process's working directory is used as a starting point.
    pub fn find_file(path: &Option<PathBuf>) -> Result<File, Box<Error>> {
        find(path, CargoFile::Config).and_then(|path| {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .map_err(From::from)
        })
    }

    /// Look for a `Cargo.lock` file
    ///
    /// Starts at the given path an goes into its parent directories until the manifest file is
    /// found. If no path is given, the process' working directory is used as a starting point.
    pub fn find_lock_file(path: &Option<PathBuf>) -> Result<File, Box<Error>> {
        find(path, CargoFile::Lock).and_then(|path| {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .map_err(From::from)
        })
    }

    /// Open the `Cargo.toml` for a path (or the process' `cwd`)
    pub fn open(path: &Option<PathBuf>) -> Result<Manifest, Box<Error>> {
        let mut file = Manifest::find_file(path)?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;

        data.parse()
    }

    /// Open the `Cargo.lock` for a path (or the process' `cwd`)
    pub fn open_lock_file(path: &Option<PathBuf>) -> Result<Manifest, Box<Error>> {
        let mut file = Manifest::find_lock_file(path)?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;

        data.parse()
    }

    /// Get the specified table from the manifest.
    pub fn get_table<'a>(
        &'a mut self,
        table_path: &[String],
    ) -> Result<&'a mut BTreeMap<String, toml::Value>, ManifestError> {

        /// Descend into a manifest until the required table is found.
        fn descend<'a>(
            input: &'a mut BTreeMap<String, toml::Value>,
            path: &[String],
        ) -> Result<&'a mut BTreeMap<String, toml::Value>, ManifestError> {
            if let Some(segment) = path.get(0) {
                let value = input
                    .entry(segment.to_owned())
                    .or_insert_with(|| toml::Value::Table(BTreeMap::new()));

                match *value {
                    toml::Value::Table(ref mut table) => descend(table, &path[1..]),
                    _ => Err(ManifestError::NonExistentTable(segment.clone())),
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
                .map(|table| {
                    sections.push((vec![dependency_type.to_string()], table.clone()))
                });

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
    pub fn write_to_file(&self, file: &mut File) -> Result<(), Box<Error>> {
        let mut toml = self.data.clone();

        let (proj_header, proj_data) = toml.remove("package")
            .map(|data| ("package", data))
            .or_else(|| toml.remove("project").map(|data| ("project", data)))
            .ok_or(ManifestError::MissingManifest)?;

        let new_contents = format!(
            "[{}]\n{}\n{}",
            proj_header,
            toml_pretty(&proj_data)?,
            toml_pretty(&toml::Value::Table(toml))?,
        );
        let new_contents_bytes = new_contents.as_bytes();

        // We need to truncate the file, otherwise the new contents
        // will be mixed up with the old ones.
        file.set_len(new_contents_bytes.len() as u64)?;
        file.write_all(new_contents_bytes).map_err(From::from)
    }

    /// Add entry to a Cargo.toml.
    pub fn insert_into_table(
        &mut self,
        table_path: &[String],
        dep: &Dependency,
    ) -> Result<(), ManifestError> {
        let mut table = self.get_table(table_path)?;

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
    pub fn update_table_entry(
        &mut self,
        table_path: &[String],
        dep: &Dependency,
    ) -> Result<(), ManifestError> {
        let mut table = self.get_table(table_path)?;
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
    pub fn remove_from_table(&mut self, table: &str, name: &str) -> Result<(), ManifestError> {
        let manifest = &mut self.data;
        let entry = manifest.entry(String::from(table));

        match entry {
            Entry::Vacant(_) => Err(ManifestError::NonExistentTable(table.into())),
            Entry::Occupied(mut section) => {
                let result = match *section.get_mut() {
                    toml::Value::Table(ref mut deps) => {
                        deps.remove(name).map(|_| ()).ok_or_else(|| {
                            ManifestError::NonExistentDependency(name.into(), table.into())
                        })
                    }
                    _ => Err(ManifestError::NonExistentTable(table.into())),
                };
                if section.get().as_table().map(|x| x.is_empty()) == Some(true) {
                    section.remove();
                }
                result
            }
        }
    }

    /// Add multiple dependencies to manifest
    pub fn add_deps(&mut self, table: &[String], deps: &[Dependency]) -> Result<(), Box<Error>> {
        deps.iter()
            .map(|dep| self.insert_into_table(table, dep))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }
}

impl str::FromStr for Manifest {
    type Err = Box<Error>;

    /// Read manifest data from string
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let d: toml::value::Value = input.parse()?;
        let e = d.as_table()
            .ok_or_else(|| ManifestError::NonExistentTable(String::from("Main")))?;

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
