use toml_edit;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
enum DependencySource {
    Version {
        version: Option<String>,
        path: Option<String>,
        registry: Option<String>,
    },
    Git {
        repo: String,
        branch: Option<String>,
    },
}

/// A dependency handled by Cargo
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Dependency {
    /// The name of the dependency (as it is set in its `Cargo.toml` and known to crates.io)
    pub name: String,
    optional: bool,
    default_features: bool,
    source: DependencySource,
    /// If the dependency is renamed, this is the new name for the dependency
    /// as a string.  None if it is not renamed.
    rename: Option<String>,
}

impl Default for Dependency {
    fn default() -> Dependency {
        Dependency {
            name: "".into(),
            rename: None,
            optional: false,
            default_features: true,
            source: DependencySource::Version {
                version: None,
                path: None,
                registry: None,
            },
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
        // versions might have semver metadata appended which we do not want to
        // store in the cargo toml files.  This would cause a warning upon compilation
        // ("version requirement […] includes semver metadata which will be ignored")
        let version = version.split('+').next().unwrap();
        let (old_path, old_registry) = match self.source {
            DependencySource::Version { path, registry, .. } => (path, registry),
            _ => (None, None),
        };
        self.source = DependencySource::Version {
            version: Some(version.into()),
            path: old_path,
            registry: old_registry,
        };
        self
    }

    /// Set dependency to a given repository
    pub fn set_git(mut self, repo: &str, branch: Option<String>) -> Dependency {
        self.source = DependencySource::Git {
            repo: repo.into(),
            branch,
        };
        self
    }

    /// Set dependency to a given path
    pub fn set_path(mut self, path: &str) -> Dependency {
        let old_version = match self.source {
            DependencySource::Version { version, .. } => version,
            _ => None,
        };
        self.source = DependencySource::Version {
            version: old_version,
            path: Some(path.into()),
            registry: None,
        };
        self
    }

    /// Set whether the dependency is optional
    pub fn set_optional(mut self, opt: bool) -> Dependency {
        self.optional = opt;
        self
    }

    /// Set the value of default-features for the dependency
    pub fn set_default_features(mut self, default_features: bool) -> Dependency {
        self.default_features = default_features;
        self
    }

    /// Set the alias for the dependency
    pub fn set_rename(mut self, rename: &str) -> Dependency {
        self.rename = Some(rename.into());
        self
    }

    /// Get the dependency name as defined in the manifest,
    /// that is, either the alias (rename field if Some),
    /// or the official package name (name field).
    pub fn name_in_manifest(&self) -> &str {
        &self.rename().unwrap_or(&self.name)
    }

    /// Set the value of registry for the dependency
    pub fn set_registry(mut self, registry: &str) -> Dependency {
        let old_version = match self.source {
            DependencySource::Version { version, .. } => version,
            _ => None,
        };
        self.source = DependencySource::Version {
            version: old_version,
            path: None,
            registry: Some(registry.into()),
        };
        self
    }

    /// Get version of dependency
    pub fn version(&self) -> Option<&str> {
        if let DependencySource::Version {
            version: Some(ref version),
            ..
        } = self.source
        {
            Some(version)
        } else {
            None
        }
    }

    /// Get the alias for the dependency (if any)
    pub fn rename(&self) -> Option<&str> {
        match &self.rename {
            Some(rename) => Some(&rename),
            None => None,
        }
    }

    /// Convert dependency to TOML
    ///
    /// Returns a tuple with the dependency's name and either the version as a `String`
    /// or the path/git repository as an `InlineTable`.
    /// (If the dependency is set as `optional` or `default-features` is set to `false`,
    /// an `InlineTable` is returned in any case.)
    pub fn to_toml(&self) -> (String, toml_edit::Item) {
        let data: toml_edit::Item = match (
            self.optional,
            self.default_features,
            self.source.clone(),
            self.rename.as_ref(),
        ) {
            // Extra short when version flag only
            (
                false,
                true,
                DependencySource::Version {
                    version: Some(v),
                    path: None,
                    registry: None,
                },
                None,
            ) => toml_edit::value(v),
            // Other cases are represented as an inline table
            (optional, default_features, source, rename) => {
                let mut data = toml_edit::InlineTable::default();

                match source {
                    DependencySource::Version {
                        version,
                        path,
                        registry,
                    } => {
                        if let Some(v) = version {
                            data.get_or_insert("version", v);
                        }
                        if let Some(p) = path {
                            data.get_or_insert("path", p);
                        }
                        if let Some(r) = registry {
                            data.get_or_insert("registry", r);
                        }
                    }
                    DependencySource::Git { repo, branch } => {
                        data.get_or_insert("git", repo);
                        branch.map(|branch| data.get_or_insert("branch", branch));
                    }
                }
                if self.optional {
                    data.get_or_insert("optional", optional);
                }
                if !self.default_features {
                    data.get_or_insert("default-features", default_features);
                }
                if rename.is_some() {
                    data.get_or_insert("package", self.name.clone());
                }

                data.fmt();
                toml_edit::value(toml_edit::Value::InlineTable(data))
            }
        };

        (self.name_in_manifest().to_string(), data)
    }
}

#[cfg(test)]
mod tests {
    use crate::dependency::Dependency;

    #[test]
    fn to_toml_simple_dep() {
        let toml = Dependency::new("dep").to_toml();

        assert_eq!(toml.0, "dep".to_owned());
    }

    #[test]
    fn to_toml_simple_dep_with_version() {
        let toml = Dependency::new("dep").set_version("1.0").to_toml();

        assert_eq!(toml.0, "dep".to_owned());
        assert_eq!(toml.1.as_str(), Some("1.0"));
    }

    #[test]
    fn to_toml_optional_dep() {
        let toml = Dependency::new("dep").set_optional(true).to_toml();

        assert_eq!(toml.0, "dep".to_owned());
        assert!(toml.1.is_inline_table());

        let dep = toml.1.as_inline_table().unwrap();
        assert_eq!(dep.get("optional").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn to_toml_dep_without_default_features() {
        let toml = Dependency::new("dep").set_default_features(false).to_toml();

        assert_eq!(toml.0, "dep".to_owned());
        assert!(toml.1.is_inline_table());

        let dep = toml.1.as_inline_table().unwrap();
        assert_eq!(dep.get("default-features").unwrap().as_bool(), Some(false));
    }

    #[test]
    fn to_toml_dep_with_path_source() {
        let toml = Dependency::new("dep").set_path("~/foo/bar").to_toml();

        assert_eq!(toml.0, "dep".to_owned());
        assert!(toml.1.is_inline_table());

        let dep = toml.1.as_inline_table().unwrap();
        assert_eq!(dep.get("path").unwrap().as_str(), Some("~/foo/bar"));
    }

    #[test]
    fn to_toml_dep_with_git_source() {
        let toml = Dependency::new("dep")
            .set_git("https://foor/bar.git", None)
            .to_toml();

        assert_eq!(toml.0, "dep".to_owned());
        assert!(toml.1.is_inline_table());

        let dep = toml.1.as_inline_table().unwrap();
        assert_eq!(
            dep.get("git").unwrap().as_str(),
            Some("https://foor/bar.git")
        );
    }

    #[test]
    fn to_toml_renamed_dep() {
        let toml = Dependency::new("dep").set_rename("d").to_toml();

        assert_eq!(toml.0, "d".to_owned());
        assert!(toml.1.is_inline_table());

        let dep = toml.1.as_inline_table().unwrap();
        assert_eq!(dep.get("package").unwrap().as_str(), Some("dep"));
    }

    #[test]
    fn to_toml_dep_from_alt_registry() {
        let toml = Dependency::new("dep").set_registry("alternative").to_toml();

        assert_eq!(toml.0, "dep".to_owned());
        assert!(toml.1.is_inline_table());

        let dep = toml.1.as_inline_table().unwrap();
        assert_eq!(dep.get("registry").unwrap().as_str(), Some("alternative"));
    }

    #[test]
    fn to_toml_complex_dep() {
        let toml = Dependency::new("dep")
            .set_version("1.0")
            .set_default_features(false)
            .set_rename("d")
            .to_toml();

        assert_eq!(toml.0, "d".to_owned());
        assert!(toml.1.is_inline_table());

        let dep = toml.1.as_inline_table().unwrap();
        assert_eq!(dep.get("package").unwrap().as_str(), Some("dep"));
        assert_eq!(dep.get("version").unwrap().as_str(), Some("1.0"));
        assert_eq!(dep.get("default-features").unwrap().as_bool(), Some(false));
    }
}
