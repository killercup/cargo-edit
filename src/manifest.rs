use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::{env, str};
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use toml;

/// A Crate Dependency
pub type Dependency = (String, toml::Value);

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
    }
}

enum CargoFile {
    Config,
    Lock,
}

/// A Cargo Manifest
#[derive(Debug, PartialEq)]
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
                             &(ref name, ref data): &Dependency)
                             -> Result<(), ManifestError> {
        let ref mut manifest = self.data;
        let entry = manifest.entry(String::from(table))
                            .or_insert(toml::Value::Table(BTreeMap::new()));
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
    ///     use cargo_edit::Manifest;
    ///     use toml;
    ///
    ///     let mut manifest = Manifest { data: toml::Table::new() };
    ///     let dep = ("cargo-edit".to_owned(), toml::Value::String("0.1.0".to_owned()));
    ///     let _ = manifest.insert_into_table("dependencies", &dep);
    ///     assert!(manifest.remove_from_table("dependencies", &dep.0).is_ok());
    ///     assert!(manifest.remove_from_table("dependencies", &dep.0).is_err());
    /// # }
    /// ```
    #[cfg_attr(feature = "dev", allow(toplevel_ref_arg))]
    pub fn remove_from_table(&mut self, table: &str, name: &str) -> Result<(), ManifestError> {
        let ref mut manifest = self.data;
        let entry = manifest.entry(String::from(table));

        match entry {
            Entry::Vacant(_) => Err(ManifestError::NonExistentTable(table.into())),
            Entry::Occupied(entry) => {
                match *entry.into_mut() {
                    toml::Value::Table(ref mut deps) => {
                        deps.remove(name)
                            .map(|_| ())
                            .ok_or(ManifestError::NonExistentDependency(name.into(), table.into()))
                    }
                    _ => Err(ManifestError::NonExistentTable(table.into())),
                }
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
              .ok_or(parser.errors.pop())
              .map_err(Option::unwrap)
              .map_err(From::from)
              .map(|data| Manifest { data: data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use toml;

    #[test]
    fn add_remove_dependency() {
        let mut manifest = Manifest { data: toml::Table::new() };
        // Create a copy containing empty "dependencies" table because removing
        //   the last entry in a table does not remove the section.
        let mut copy = Manifest { data: toml::Table::new() };
        copy.data.insert("dependencies".to_owned(),
                         toml::Value::Table(BTreeMap::new()));
        let dep = ("cargo-edit".to_owned(),
                   toml::Value::String("0.1.0".to_owned()));
        let _ = manifest.insert_into_table("dependencies", &dep);
        assert!(manifest.remove_from_table("dependencies", &dep.0).is_ok());
        assert_eq!(manifest, copy);
    }

    #[test]
    fn remove_dependency_no_section() {
        let mut manifest = Manifest { data: toml::Table::new() };
        let dep = ("cargo-edit".to_owned(),
                   toml::Value::String("0.1.0".to_owned()));
        assert!(manifest.remove_from_table("dependencies", &dep.0).is_err());
    }

    #[test]
    fn remove_dependency_non_existent() {
        let mut manifest = Manifest { data: toml::Table::new() };
        let dep = ("cargo-edit".to_owned(),
                   toml::Value::String("0.1.0".to_owned()));
        let other_dep = ("other-dep".to_owned(),
                         toml::Value::String("0.1.0".to_owned()));
        let _ = manifest.insert_into_table("dependencies", &other_dep);
        assert!(manifest.remove_from_table("dependencies", &dep.0).is_err());

    }
}
