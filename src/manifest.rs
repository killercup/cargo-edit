use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use toml;

pub type Dependency = (String, toml::Value);
pub type TomlMap = BTreeMap<String, toml::Value>;

#[derive(Debug)]
// Catch-all error for misconfigured crates.
pub struct ManifestError;

impl Error for ManifestError {
    fn description(&self) -> &str {
        "Your Cargo.toml is either missing or incorrectly structured."
    }
}

impl fmt::Display for ManifestError {
    fn fmt(&self, format: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        format.write_str(self.description())
    }
}

// If a manifest is specified, return that one, otherise perform a manifest search starting from
// the current directory.
pub fn find_manifest(specified: Option<&String>) -> Result<PathBuf, Box<Error>> {
    specified.map(PathBuf::from).ok_or(())
    .or_else(|_| env::current_dir().map_err(From::from)
                 .and_then(|ref dir| manifest_search(dir).map_err(From::from)))
}

// Search for Cargo.toml in this directory and recursively up the tree until one is found.
#[allow(unconditional_recursion)] //Incorrect lint; recursion is conditional.
fn manifest_search(dir: &Path) -> Result<PathBuf, ManifestError> {
    let manifest = dir.join("Cargo.toml");
    fs::metadata(&manifest).map(|_| manifest)
    .or(dir.parent().ok_or(ManifestError).and_then(manifest_search))
}

// Read all the contents of a file & parse as a TOML table.
pub fn read_as_toml(file: &mut File) -> Result<TomlMap, Box<Error>> {
    let mut data = String::new();
    try!(file.read_to_string(&mut data));
    let mut parser = toml::Parser::new(&data);
    parser.parse().ok_or(parser.errors.pop()).map_err(Option::unwrap).map_err(From::from)
}

// Overwrite a file with TOML data.
pub fn write_from_toml(file: &mut File, mut toml: TomlMap)
        -> Result<(), Box<Error>> {
    try!(file.seek(SeekFrom::Start(0)));
    let (proj_header, proj_data) =
        try!(toml.remove("package").map(|data| ("package", data))
             .or_else(|| toml.remove("project").map(|data| ("project", data)))
             .ok_or(ManifestError));
    write!(file, "[{}]\n{}{}", proj_header, proj_data,
           toml::Value::Table(toml)).map_err(From::from)
}

// Add entry to a Cargo.toml.
pub fn insert_into_table(manifest: &mut TomlMap, table: String, (name, data): Dependency)
        -> Result<(), ManifestError> {
    match manifest.entry(table)
    .or_insert(toml::Value::Table(BTreeMap::new())) {
        &mut toml::Value::Table(ref mut deps) => {
            deps.insert(name, data);
            Ok(())
        }
        _ => Err(ManifestError)
    }
}
