use std::collections::BTreeMap;
use toml;

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
}

impl Default for Dependency {
    fn default() -> Dependency {
        Dependency {
            name: "".into(),
            optional: false,
            source: DependencySource::Version("0.1.0".into()),
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
    /// Returns a tuple with the dependency's name and either the version as a String or the
    /// the path/git repository as a table. (If the dependency is set as `optional`, a tables is
    /// returned in any case.)
    pub fn to_toml(&self) -> (String, toml::Value) {
        let data: toml::Value = match (self.optional, self.source.clone()) {
            // Extra short when version flag only
            (false, DependencySource::Version(v)) => toml::Value::String(v),
            // Other cases are represented as tables
            (optional, source) => {
                let mut data = BTreeMap::new();

                match source {
                    DependencySource::Version(v) => {
                        data.insert("version".into(), toml::Value::String(v));
                    }
                    DependencySource::Git(v) => {
                        data.insert("git".into(), toml::Value::String(v));
                    }
                    DependencySource::Path(v) => {
                        data.insert("path".into(), toml::Value::String(v));
                    }
                }
                data.insert("optional".into(), toml::Value::Boolean(optional));

                toml::Value::Table(data)
            }
        };

        (self.name.clone(), data)
    }
}
