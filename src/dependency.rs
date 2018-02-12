use toml_edit;
use std::iter::FromIterator;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
enum DependencySource {
    Version(String),
    Git(String),
    Path(String),
}

/// A dependency handled by Cargo
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Dependency {
    /// The name of the dependency (as it is set in its `Cargo.toml` and known to crates.io)
    pub name: String,
    optional: bool,
    source: DependencySource,
    features: Vec<String>,
}

impl Default for Dependency {
    fn default() -> Dependency {
        Dependency {
            name: "".into(),
            optional: false,
            source: DependencySource::Version("0.1.0".into()),
            features: vec![],
        }
    }
}

impl Dependency {
    /// Create a new dependency with a name
    pub fn new(name: &str) -> Dependency {
        Dependency {
            name: name.into(),
            ..Dependency::default()
        }
    }

    /// Set dependency to a given version
    pub fn set_version(mut self, version: &str) -> Dependency {
        self.source = DependencySource::Version(version.into());
        self
    }

    /// Set dependency to a given repository
    pub fn set_git(mut self, repo: &str) -> Dependency {
        self.source = DependencySource::Git(repo.into());
        self
    }

    /// Set dependency to a given path
    pub fn set_path(mut self, path: &str) -> Dependency {
        self.source = DependencySource::Path(path.into());
        self
    }

    /// Set whether the dependency is optional
    pub fn set_optional(mut self, opt: bool) -> Dependency {
        self.optional = opt;
        self
    }
    /// Set whether the features is array of string
    pub fn set_features(mut self, features: Option<String>) -> Dependency {
        if let Some(f) = features {
            self.features = f.split(' ').map(String::from).collect::<Vec<String>>();
        }
        self
    }

    /// Get version of dependency
    pub fn version(&self) -> Option<&str> {
        if let DependencySource::Version(ref version) = self.source {
            Some(version)
        } else {
            None
        }
    }

    /// Convert dependency to TOML
    ///
    /// Returns a tuple with the dependency's name and either the version as a `String`
    /// or the path/git repository as an `InlineTable`.
    /// (If the dependency is set as `optional`, an `InlineTable` is returned in any case.)
    pub fn to_toml(&self) -> (String, toml_edit::Item) {
        let data: toml_edit::Item = match (self.optional, self.features.len(), self.source.clone())
        {
            // Extra short when version flag only
            (false, 0, DependencySource::Version(v)) => toml_edit::value(v),
            // Other cases are represented as an inline table
            (optional, _len, source) => {
                let mut data = toml_edit::InlineTable::default();

                match source {
                    DependencySource::Version(v) => {
                        data.get_or_insert("version", v);
                    }
                    DependencySource::Git(v) => {
                        data.get_or_insert("git", v);
                    }
                    DependencySource::Path(v) => {
                        data.get_or_insert("path", v);
                    }
                }
                if self.optional {
                    data.get_or_insert("optional", optional);
                }
                if !self.features.is_empty() {
                    let features = toml_edit::Value::from_iter(self.features.iter().cloned());
                    data.get_or_insert("features", features);
                }
                data.fmt();
                toml_edit::value(toml_edit::Value::InlineTable(data))
            }
        };

        (self.name.clone(), data)
    }
}
