use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::{env, str};
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use toml;

use dependency::Dependency;

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
            display("{}:{}{} {}",
                loline + 1, locol + 1,
                if loline != hiline || locol != hicol {
                    format!("-{}:{}", hiline + 1,
                            hicol + 1)
                } else {
                    "".to_string()
                },
                error)
        }
    }
}

enum CargoFile {
    Config,
    Lock,
}

/// A Cargo Manifest
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    /// Manifest contents as TOML data
    pub data: toml::Table,
}

/// If a manifest is specified, return that one, otherise perform a manifest search starting from
/// the current directory.
/// If a manifest is specified, return that one. If a path is specified, perform a manifest search
/// starting from there. If nothing is specified, start searching from the current directory
/// (`cwd`).
fn find(specified: &Option<&str>, file: CargoFile) -> Result<PathBuf, Box<Error>> {
    let file_path = specified.map(PathBuf::from);

    if let Some(path) = file_path {
        if try!(fs::metadata(&path)).is_file() {
            Ok(path)
        } else {
            search(&path, file).map_err(From::from)
        }
    } else {
        env::current_dir()
            .map_err(From::from)
            .and_then(|ref dir| search(dir, file).map_err(From::from))
    }
}

/// Search for Cargo.toml in this directory and recursively up the tree until one is found.
fn search(dir: &Path, file: CargoFile) -> Result<PathBuf, ManifestError> {
    let manifest = match file {
        CargoFile::Config => dir.join("Cargo.toml"),
        CargoFile::Lock => dir.join("Cargo.lock"),
    };

    fs::metadata(&manifest)
        .map(|_| manifest)
        .or(dir.parent().ok_or(ManifestError::MissingManifest).and_then(|dir| search(dir, file)))
}

impl Manifest {
    /// Look for a `Cargo.toml` file
    ///
    /// Starts at the given path an goes into its parent directories until the manifest file is
    /// found. If no path is given, the process's working directory is used as a starting point.
    pub fn find_file(path: &Option<&str>) -> Result<File, Box<Error>> {
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
    pub fn find_lock_file(path: &Option<&str>) -> Result<File, Box<Error>> {
        find(path, CargoFile::Lock).and_then(|path| {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .map_err(From::from)
        })
    }

    /// Open the `Cargo.toml` for a path (or the process' `cwd`)
    pub fn open(path: &Option<&str>) -> Result<Manifest, Box<Error>> {
        let mut file = try!(Manifest::find_file(path));
        let mut data = String::new();
        try!(file.read_to_string(&mut data));

        data.parse()
    }

    /// Open the `Cargo.lock` for a path (or the process' `cwd`)
    pub fn open_lock_file(path: &Option<&str>) -> Result<Manifest, Box<Error>> {
        let mut file = try!(Manifest::find_lock_file(path));
        let mut data = String::new();
        try!(file.read_to_string(&mut data));

        data.parse()
    }

    /// Overwrite a file with TOML data.
    pub fn write_to_file(&self, file: &mut File) -> Result<(), Box<Error>> {
        let mut toml = self.data.clone();

        let (proj_header, proj_data) = try!(toml.remove("package")
                                                .map(|data| ("package", data))
                                                .or_else(|| {
                                                    toml.remove("project")
                                                        .map(|data| ("project", data))
                                                })
                                                .ok_or(ManifestError::MissingManifest));

        let new_contents = format!("[{}]\n{}{}",
                                   proj_header,
                                   proj_data,
                                   toml::Value::Table(toml));
        let new_contents_bytes = new_contents.as_bytes();

        // We need to truncate the file, otherwise the new contents
        // will be mixed up with the old ones.
        try!(file.set_len(new_contents_bytes.len() as u64));
        file.write_all(new_contents_bytes).map_err(From::from)
    }

    /// Add entry to a Cargo.toml.
    #[cfg_attr(feature = "dev", allow(toplevel_ref_arg))]
    pub fn insert_into_table(&mut self,
                             table: &str,
                             dep: &Dependency)
                             -> Result<(), ManifestError> {
        let (ref name, ref data) = dep.to_toml();
        let ref mut manifest = self.data;
        let entry = manifest.entry(String::from(table))
                            .or_insert_with(|| toml::Value::Table(BTreeMap::new()));
        match *entry {
            toml::Value::Table(ref mut deps) => {
                deps.insert(name.clone(), data.clone());
                Ok(())
            }
            _ => Err(ManifestError::NonExistentTable(table.into())),
        }
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
    ///     let mut manifest = Manifest { data: toml::Table::new() };
    ///     let dep = Dependency::new("cargo-edit").set_version("0.1.0");
    ///     let _ = manifest.insert_into_table("dependencies", &dep);
    ///     assert!(manifest.remove_from_table("dependencies", &dep.name).is_ok());
    ///     assert!(manifest.remove_from_table("dependencies", &dep.name).is_err());
    ///     assert!(manifest.data.is_empty());
    /// # }
    /// ```
    #[cfg_attr(feature = "dev", allow(toplevel_ref_arg))]
    pub fn remove_from_table(&mut self, table: &str, name: &str) -> Result<(), ManifestError> {
        let ref mut manifest = self.data;
        let entry = manifest.entry(String::from(table));

        match entry {
            Entry::Vacant(_) => Err(ManifestError::NonExistentTable(table.into())),
            Entry::Occupied(mut section) => {
                let result = match *section.get_mut() {
                    toml::Value::Table(ref mut deps) => {
                        deps.remove(name)
                            .map(|_| ())
                            .ok_or_else(|| ManifestError::NonExistentDependency(name.into(), table.into()))
                    }
                    _ => Err(ManifestError::NonExistentTable(table.into())),
                };
                if let Some(empty) = section.get().as_table().and_then(|x| Some(x.is_empty())) {
                    if empty {
                        section.remove();
                    }
                }
                result
            }
        }
    }

    /// Add multiple dependencies to manifest
    pub fn add_deps(&mut self, table: &str, deps: &[Dependency]) -> Result<(), Box<Error>> {
        deps.iter()
            .map(|dep| self.insert_into_table(table, &dep))
            .collect::<Result<Vec<_>, _>>()
            .map_err(From::from)
            .map(|_| ())
    }
}

impl str::FromStr for Manifest {
    type Err = Box<Error>;

    /// Read manifest data from string
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut parser = toml::Parser::new(&input);

        parser.parse()
              .ok_or(format_parse_error(parser))
              .map_err(Option::unwrap)
              .map_err(From::from)
              .map(|data| Manifest { data: data })
    }
}

fn format_parse_error(mut parser: toml::Parser) -> Option<ManifestError> {
    match parser.errors.pop() {
        Some(error) => {
            let (loline, locol) = parser.to_linecol(error.lo);
            let (hiline, hicol) = parser.to_linecol(error.hi);
            Some(ManifestError::ParseError(error.desc, loline, locol, hiline, hicol))       
        },
        None => None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dependency::Dependency;
    use toml;

    #[test]
    fn add_remove_dependency() {
        let mut manifest = Manifest { data: toml::Table::new() };
        let clone = manifest.clone();
        let dep = Dependency::new("cargo-edit").set_version("0.1.0");
        let _ = manifest.insert_into_table("dependencies", &dep);
        assert!(manifest.remove_from_table("dependencies", &dep.name).is_ok());
        assert_eq!(manifest, clone);
    }

    #[test]
    fn remove_dependency_no_section() {
        let mut manifest = Manifest { data: toml::Table::new() };
        let dep = Dependency::new("cargo-edit").set_version("0.1.0");
        assert!(manifest.remove_from_table("dependencies", &dep.name).is_err());
    }

    #[test]
    fn remove_dependency_non_existent() {
        let mut manifest = Manifest { data: toml::Table::new() };
        let dep = Dependency::new("cargo-edit").set_version("0.1.0");
        let other_dep = Dependency::new("other-dep").set_version("0.1.0");
        let _ = manifest.insert_into_table("dependencies", &other_dep);
        assert!(manifest.remove_from_table("dependencies", &dep.name).is_err());
    }
}
