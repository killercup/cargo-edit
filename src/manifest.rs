use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use toml;

pub type Dependency = (String, toml::Value);
pub type TomlMap = BTreeMap<String, toml::Value>;

#[derive(Debug)]
/// Catch-all error for misconfigured crates.
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

#[derive(Debug, PartialEq)]
pub struct Manifest {
    data: TomlMap
}

impl Manifest {
    pub fn from_str(input: &str) -> Result<Manifest, Box<Error>> {
        let mut parser = toml::Parser::new(&input);

        parser.parse()
        .ok_or(parser.errors.pop())
        .map_err(Option::unwrap).map_err(From::from)
        .map(|data| {
            Manifest { data: data }
        })
    }

    pub fn find_file(path: &Option<&String>) -> Result<File, Box<Error>> {
        /// If a manifest is specified, return that one, otherise perform a manifest search starting from
        /// the current directory.
        fn find(specified: &Option<&String>) -> Result<PathBuf, Box<Error>> {
            specified.map(PathBuf::from).ok_or(())
            .or_else(|_| env::current_dir().map_err(From::from)
                         .and_then(|ref dir| search(dir).map_err(From::from)))
        }

        /// Search for Cargo.toml in this directory and recursively up the tree until one is found.
        #[allow(unconditional_recursion)] //Incorrect lint; recursion is conditional.
        fn search(dir: &Path) -> Result<PathBuf, ManifestError> {
            let manifest = dir.join("Cargo.toml");
            fs::metadata(&manifest).map(|_| manifest)
            .or(dir.parent().ok_or(ManifestError).and_then(search))
        }

        find(path)
        .and_then(|path| {
            OpenOptions::new()
            .read(true).write(true).open(path)
            .map_err(From::from)
        })
    }

    pub fn open(path: &Option<&String>) -> Result<Manifest, Box<Error>> {
        let mut file = try!(Manifest::find_file(path));
        let mut data = String::new();
        try!(file.read_to_string(&mut data));

        Manifest::from_str(&data)
    }

    /// Overwrite a file with TOML data.
    pub fn write_to_file<T: Seek + Write>(&self, file: &mut T)
            -> Result<(), Box<Error>> {
        try!(file.seek(SeekFrom::Start(0)));
        let mut toml = self.data.clone();

        let (proj_header, proj_data) =
            try!(toml.remove("package").map(|data| ("package", data))
                 .or_else(|| toml.remove("project").map(|data| ("project", data)))
                 .ok_or(ManifestError));
        write!(file, "[{}]\n{}{}", proj_header, proj_data,
               toml::Value::Table(toml)).map_err(From::from)
    }

    /// Add entry to a Cargo.toml.
    fn insert_into_table(&mut self, table: &str, &(ref name, ref data): &Dependency)
            -> Result<(), ManifestError> {
        let ref mut manifest = self.data;
        let entry = manifest.entry(String::from(table))
            .or_insert(toml::Value::Table(BTreeMap::new()));
        match entry {
            &mut toml::Value::Table(ref mut deps) => {
                deps.insert(name.clone(), data.clone());
                Ok(())
            }
            _ => Err(ManifestError)
        }
    }

    /// Add entry to manifest
    pub fn add_deps(&mut self, table: &str, deps: &[Dependency])
            -> Result<(), Box<Error>> {
        deps.iter()
        .map(|dep| self.insert_into_table(table, &dep))
        .collect::<Result<Vec<_>, _>>()
        .map_err(From::from)
        .map(|_| ())
    }
}

#[cfg(test)]
mod test {
    use args::Args;
    use super::Manifest;

    static default_cargo_toml: &'static str = r#"[package]
authors = ["Some Guy"]
name = "lorem-ipsum"
version = "0.1.0"

[dependencies]
foo-bar = "0.1""#;

    #[test]
    fn add_dependency() {
        let opts = Args {
            arg_section: String::from("dependencies"),
            arg_dep: vec![String::from("lorem-ipsum")],
            ..Default::default()
        };

        let mut manifile = Manifest::from_str(default_cargo_toml).unwrap();

        manifile.add_deps(
            &opts.get_section(),
            &opts.get_dependencies()
        );

        println!("{:#?}", manifile);

        let lorem = manifile.data.get(&opts.get_section()).expect("no section")
            .lookup("lorem-ipsum").expect("no lorem")
            .as_str().expect("not a str");

        assert_eq!(
            lorem,
            "*"
        );
    }
}
