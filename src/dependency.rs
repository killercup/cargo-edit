use std::path::{Path, PathBuf};

use crate::manifest::str_or_1_len_table;

/// A dependency handled by Cargo
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Dependency {
    /// The name of the dependency (as it is set in its `Cargo.toml` and known to crates.io)
    pub name: String,
    optional: bool,
    /// List of features to add (or None to keep features unchanged).
    pub features: Option<Vec<String>>,
    default_features: bool,
    source: DependencySource,
    /// If the dependency is renamed, this is the new name for the dependency
    /// as a string.  None if it is not renamed.
    rename: Option<String>,

    /// Features that are exposed by the dependency
    pub available_features: Vec<String>,
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

    /// Set the available features of the dependency to a given vec
    pub fn set_available_features(mut self, available_features: Vec<String>) -> Dependency {
        self.available_features = available_features;
        self
    }

    /// Set dependency to a given repository
    pub fn set_git(
        mut self,
        repo: &str,
        branch: Option<String>,
        tag: Option<String>,
        rev: Option<String>,
    ) -> Dependency {
        self.source = DependencySource::Git {
            repo: repo.into(),
            branch,
            tag,
            rev,
        };
        self
    }

    /// Set dependency to a given path
    ///
    /// # Panic
    ///
    /// Panics if the path is relative
    pub fn set_path(mut self, path: PathBuf) -> Dependency {
        assert!(
            path.is_absolute(),
            "Absolute path needed, got: {}",
            path.display()
        );
        let old_version = match self.source {
            DependencySource::Version { version, .. } => version,
            _ => None,
        };
        self.source = DependencySource::Version {
            version: old_version,
            path: Some(path),
            registry: None,
        };
        self
    }

    /// Set whether the dependency is optional
    pub fn set_optional(mut self, opt: bool) -> Dependency {
        self.optional = opt;
        self
    }
    /// Set features as an array of string (does some basic parsing)
    pub fn set_features(mut self, features: Option<Vec<String>>) -> Dependency {
        self.features = features;
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

    /// Get the path of the dependency
    pub fn path(&self) -> Option<&Path> {
        if let DependencySource::Version {
            path: Some(ref path),
            ..
        } = self.source
        {
            Some(path.as_path())
        } else {
            None
        }
    }

    /// Get the alias for the dependency (if any)
    pub fn rename(&self) -> Option<&str> {
        self.rename.as_deref()
    }

    /// Get the dependency name as defined in the manifest,
    /// that is, either the alias (rename field if Some),
    /// or the official package name (name field).
    pub fn toml_key(&self) -> &str {
        self.rename().unwrap_or(&self.name)
    }

    /// Convert dependency to TOML
    ///
    /// Returns a tuple with the dependency's name and either the version as a `String`
    /// or the path/git repository as an `InlineTable`.
    /// (If the dependency is set as `optional` or `default-features` is set to `false`,
    /// an `InlineTable` is returned in any case.)
    ///
    /// # Panic
    ///
    /// Panics if the path is relative
    pub fn to_toml(&self, crate_root: &Path) -> toml_edit::Item {
        assert!(
            crate_root.is_absolute(),
            "Absolute path needed, got: {}",
            crate_root.display()
        );
        let data: toml_edit::Item = match (
            self.optional,
            self.features.as_ref(),
            self.default_features,
            self.source.clone(),
            self.rename.as_ref(),
        ) {
            // Extra short when version flag only
            (
                false,
                None,
                true,
                DependencySource::Version {
                    version: Some(v),
                    path: None,
                    registry: None,
                },
                None,
            ) => toml_edit::value(v),
            // Other cases are represented as an inline table
            (optional, features, default_features, source, rename) => {
                let mut data = toml_edit::InlineTable::default();

                match source {
                    DependencySource::Version {
                        version,
                        path,
                        registry,
                    } => {
                        if let Some(v) = version {
                            data.insert("version", v.into());
                        }
                        if let Some(p) = path {
                            let relpath = pathdiff::diff_paths(p, crate_root)
                                .expect("both paths are absolute");
                            let relpath = relpath.to_str().unwrap().replace('\\', "/");
                            data.insert("path", relpath.into());
                        }
                        if let Some(r) = registry {
                            data.insert("registry", r.into());
                        }
                    }
                    DependencySource::Git {
                        repo,
                        branch,
                        tag,
                        rev,
                    } => {
                        data.insert("git", repo.into());
                        if let Some(branch) = branch {
                            data.insert("branch", branch.into());
                        }
                        if let Some(tag) = tag {
                            data.insert("tag", tag.into());
                        }
                        if let Some(rev) = rev {
                            data.insert("rev", rev.into());
                        }
                    }
                }
                if rename.is_some() {
                    data.insert("package", self.name.as_str().into());
                }
                if !self.default_features {
                    data.insert("default-features", default_features.into());
                }
                if let Some(features) = features {
                    let features: toml_edit::Value = features.iter().cloned().collect();
                    data.insert("features", features);
                }
                if self.optional {
                    data.insert("optional", optional.into());
                }

                toml_edit::value(toml_edit::Value::InlineTable(data))
            }
        };

        data
    }

    /// Modify existing entry to match this dependency
    pub fn update_toml(&self, crate_root: &Path, item: &mut toml_edit::Item) {
        if str_or_1_len_table(item) {
            // Nothing to preserve
            *item = self.to_toml(crate_root);
        } else if !is_package_eq(item, &self.name, self.rename.as_deref()) {
            // No existing keys are relevant when the package changes
            *item = self.to_toml(crate_root);
        } else if let Some(table) = item.as_table_like_mut() {
            let new_toml = self.to_toml(crate_root);
            if let Some(name) = new_toml.as_str() {
                table.insert("version", toml_edit::value(name));
                for key in &["path", "git", "branch", "tag", "rev"] {
                    table.remove(key);
                }
            } else {
                let new_toml = new_toml
                    .as_inline_table()
                    .expect("If a dep isn't a str, then its a table");
                merge_table(table, new_toml);
                // These are not relevant when overwriting.  Doing this after to preserve order for
                // existing fields
                for key in &["version", "path", "git", "branch", "tag", "rev"] {
                    if !new_toml.contains_key(key) {
                        table.remove(key);
                    }
                }
            }
            table.fmt();
        } else {
            unreachable!("Invalid dependency type: {}", item.type_name());
        }
    }
}

fn is_package_eq(item: &mut toml_edit::Item, name: &str, rename: Option<&str>) -> bool {
    if let Some(table) = item.as_table_like_mut() {
        let existing_package = table.get("package").and_then(|i| i.as_str());
        let new_package = rename.map(|_| name);
        existing_package == new_package
    } else {
        false
    }
}

fn merge_table(old_dep: &mut dyn toml_edit::TableLike, new: &toml_edit::InlineTable) {
    for (k, v) in new.iter() {
        old_dep.insert(k, toml_edit::value(v.clone()));
    }
}

impl Default for Dependency {
    fn default() -> Dependency {
        Dependency {
            name: "".into(),
            rename: None,
            optional: false,
            features: None,
            default_features: true,
            source: DependencySource::Version {
                version: None,
                path: None,
                registry: None,
            },
            available_features: vec![],
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum DependencySource {
    Version {
        version: Option<String>,
        path: Option<PathBuf>,
        registry: Option<String>,
    },
    Git {
        repo: String,
        branch: Option<String>,
        tag: Option<String>,
        rev: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use crate::dependency::Dependency;
    use std::path::Path;

    #[test]
    fn to_toml_simple_dep() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let dep = Dependency::new("dep");
        let key = dep.toml_key();
        let _item = dep.to_toml(&crate_root);

        assert_eq!(key, "dep".to_owned());
    }

    #[test]
    fn to_toml_simple_dep_with_version() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let dep = Dependency::new("dep").set_version("1.0");
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        assert_eq!(key, "dep".to_owned());
        assert_eq!(item.as_str(), Some("1.0"));
    }

    #[test]
    fn to_toml_optional_dep() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let dep = Dependency::new("dep").set_optional(true);
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        assert_eq!(key, "dep".to_owned());
        assert!(item.is_inline_table());

        let dep = item.as_inline_table().unwrap();
        assert_eq!(dep.get("optional").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn to_toml_dep_without_default_features() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let dep = Dependency::new("dep").set_default_features(false);
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        assert_eq!(key, "dep".to_owned());
        assert!(item.is_inline_table());

        let dep = item.as_inline_table().unwrap();
        assert_eq!(dep.get("default-features").unwrap().as_bool(), Some(false));
    }

    #[test]
    fn to_toml_dep_with_path_source() {
        let root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let crate_root = root.join("foo");
        let dep = Dependency::new("dep").set_path(root.join("bar"));
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        assert_eq!(key, "dep".to_owned());
        assert!(item.is_inline_table());

        let dep = item.as_inline_table().unwrap();
        assert_eq!(dep.get("path").unwrap().as_str(), Some("../bar"));
    }

    #[test]
    fn to_toml_dep_with_git_source() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let dep = Dependency::new("dep").set_git("https://foor/bar.git", None, None, None);
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        assert_eq!(key, "dep".to_owned());
        assert!(item.is_inline_table());

        let dep = item.as_inline_table().unwrap();
        assert_eq!(
            dep.get("git").unwrap().as_str(),
            Some("https://foor/bar.git")
        );
    }

    #[test]
    fn to_toml_renamed_dep() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let dep = Dependency::new("dep").set_rename("d");
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        assert_eq!(key, "d".to_owned());
        assert!(item.is_inline_table());

        let dep = item.as_inline_table().unwrap();
        assert_eq!(dep.get("package").unwrap().as_str(), Some("dep"));
    }

    #[test]
    fn to_toml_dep_from_alt_registry() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let dep = Dependency::new("dep").set_registry("alternative");
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        assert_eq!(key, "dep".to_owned());
        assert!(item.is_inline_table());

        let dep = item.as_inline_table().unwrap();
        assert_eq!(dep.get("registry").unwrap().as_str(), Some("alternative"));
    }

    #[test]
    fn to_toml_complex_dep() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let dep = Dependency::new("dep")
            .set_version("1.0")
            .set_default_features(false)
            .set_rename("d");
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        assert_eq!(key, "d".to_owned());
        assert!(item.is_inline_table());

        let dep = item.as_inline_table().unwrap();
        assert_eq!(dep.get("package").unwrap().as_str(), Some("dep"));
        assert_eq!(dep.get("version").unwrap().as_str(), Some("1.0"));
        assert_eq!(dep.get("default-features").unwrap().as_bool(), Some(false));
    }

    #[test]
    fn paths_with_forward_slashes_are_left_as_is() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let path = crate_root.join("sibling/crate");
        let relpath = "sibling/crate";
        let dep = Dependency::new("dep").set_path(path);
        let item = dep.to_toml(&crate_root);

        let table = item.as_inline_table().unwrap();
        let got = table.get("path").unwrap().as_str().unwrap();
        assert_eq!(got, relpath);
    }

    #[test]
    #[cfg(windows)]
    fn normalise_windows_style_paths() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let original = crate_root.join(r"sibling\crate");
        let should_be = "sibling/crate";
        let dep = Dependency::new("dep").set_path(original);
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        let table = item.as_inline_table().unwrap();
        let got = table.get("path").unwrap().as_str().unwrap();
        assert_eq!(got, should_be);
    }
}
