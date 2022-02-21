use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::manifest::str_or_1_len_table;

/// A dependency handled by Cargo
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Dependency {
    /// The name of the dependency (as it is set in its `Cargo.toml` and known to crates.io)
    pub name: String,
    optional: Option<bool>,
    /// List of features to add (or None to keep features unchanged).
    pub features: Option<Vec<String>>,
    default_features: Option<bool>,
    source: DependencySource,
    /// If the dependency is renamed, this is the new name for the dependency
    /// as a string.  None if it is not renamed.
    rename: Option<String>,

    /// Features that are exposed by the dependency
    pub available_features: BTreeMap<String, Vec<String>>,
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

    /// Remove the existing version requirement
    pub fn clear_version(mut self) -> Dependency {
        if let DependencySource::Version {
            version, registry, ..
        } = &mut self.source
        {
            *version = None;
            *registry = None;
        }
        self
    }

    /// Set the available features of the dependency to a given vec
    pub fn set_available_features(
        mut self,
        available_features: BTreeMap<String, Vec<String>>,
    ) -> Dependency {
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
        let (old_version, old_registry) = match self.source {
            DependencySource::Version {
                version, registry, ..
            } => (version, registry),
            _ => (None, None),
        };
        self.source = DependencySource::Version {
            version: old_version,
            path: Some(path),
            registry: old_registry,
        };
        self
    }

    /// Set whether the dependency is optional
    pub fn set_optional(mut self, opt: Option<bool>) -> Dependency {
        self.optional = opt;
        self
    }
    /// Set features as an array of string (does some basic parsing)
    pub fn set_features(mut self, features: Option<Vec<String>>) -> Dependency {
        self.features = features;
        self
    }

    /// Set the value of default-features for the dependency
    pub fn set_default_features(mut self, default_features: Option<bool>) -> Dependency {
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
        let (old_version, old_path) = match self.source {
            DependencySource::Version { version, path, .. } => (version, path),
            _ => (None, None),
        };
        self.source = DependencySource::Version {
            version: old_version,
            path: old_path,
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

    /// Get registry of the dependency
    pub fn registry(&self) -> Option<&str> {
        if let DependencySource::Version {
            registry: Some(ref registry),
            ..
        } = self.source
        {
            Some(registry)
        } else {
            None
        }
    }

    /// Get the git repo of the dependency
    pub fn git(&self) -> Option<&str> {
        if let DependencySource::Git { repo, .. } = &self.source {
            Some(repo.as_str())
        } else {
            None
        }
    }

    /// Get the alias for the dependency (if any)
    pub fn rename(&self) -> Option<&str> {
        self.rename.as_deref()
    }

    /// Whether default features are activated
    pub fn default_features(&self) -> Option<bool> {
        self.default_features
    }
}

impl Dependency {
    /// Create a dependency from a TOML table entry
    pub fn from_toml(crate_root: &Path, key: &str, item: &toml_edit::Item) -> Option<Self> {
        if let Some(version) = item.as_str() {
            let dep = Dependency::new(key).set_version(version);
            Some(dep)
        } else if let Some(table) = item.as_table_like() {
            let (name, rename) = if let Some(value) = table.get("package") {
                (value.as_str()?.to_owned(), Some(key.to_owned()))
            } else {
                (key.to_owned(), None)
            };

            let source = if let Some(repo) = table.get("git") {
                let repo = repo.as_str()?.to_owned();
                let branch = if let Some(value) = table.get("branch") {
                    Some(value.as_str()?.to_owned())
                } else {
                    None
                };
                let tag = if let Some(value) = table.get("tag") {
                    Some(value.as_str()?.to_owned())
                } else {
                    None
                };
                let rev = if let Some(value) = table.get("rev") {
                    Some(value.as_str()?.to_owned())
                } else {
                    None
                };
                DependencySource::Git {
                    repo,
                    branch,
                    tag,
                    rev,
                }
            } else {
                let version = if let Some(value) = table.get("version") {
                    Some(value.as_str()?.to_owned())
                } else {
                    None
                };
                let path = if let Some(value) = table.get("path") {
                    let path = value.as_str()?;
                    let path = crate_root.join(path);
                    Some(path)
                } else {
                    None
                };
                let registry = if let Some(value) = table.get("registry") {
                    Some(value.as_str()?.to_owned())
                } else {
                    None
                };
                DependencySource::Version {
                    version,
                    path,
                    registry,
                }
            };

            let default_features = if let Some(value) = table.get("default-features") {
                value.as_bool()?
            } else {
                true
            };
            let default_features = Some(default_features);

            let features = if let Some(value) = table.get("features") {
                Some(
                    value
                        .as_array()?
                        .iter()
                        .map(|v| v.as_str().map(|s| s.to_owned()))
                        .collect::<Option<Vec<String>>>()?,
                )
            } else {
                None
            };

            let available_features = BTreeMap::default();

            let optional = if let Some(value) = table.get("optional") {
                value.as_bool()?
            } else {
                false
            };
            let optional = Some(optional);

            let dep = Dependency {
                name,
                rename,
                source,
                default_features,
                features,
                available_features,
                optional,
            };
            Some(dep)
        } else {
            None
        }
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
            self.optional.unwrap_or(false),
            self.features.as_ref(),
            self.default_features.unwrap_or(true),
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
            (_, _, _, _, _) => {
                let mut data = toml_edit::InlineTable::default();

                match &self.source {
                    DependencySource::Version {
                        version,
                        path,
                        registry,
                    } => {
                        if let Some(v) = version {
                            data.insert("version", v.into());
                        }
                        if let Some(p) = path {
                            let relpath = path_field(crate_root, p);
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
                if self.rename.is_some() {
                    data.insert("package", self.name.as_str().into());
                }
                match self.default_features {
                    Some(true) | None => {}
                    Some(false) => {
                        data.insert("default-features", false.into());
                    }
                }
                if let Some(features) = self.features.as_deref() {
                    let features: toml_edit::Value = features.iter().cloned().collect();
                    data.insert("features", features);
                }
                match self.optional {
                    Some(false) | None => {}
                    Some(true) => {
                        data.insert("optional", true.into());
                    }
                }

                toml_edit::value(toml_edit::Value::InlineTable(data))
            }
        };

        data
    }

    /// Modify existing entry to match this dependency
    pub fn update_toml(&self, crate_root: &Path, item: &mut toml_edit::Item) {
        #[allow(clippy::if_same_then_else)]
        if str_or_1_len_table(item) {
            // Nothing to preserve
            *item = self.to_toml(crate_root);
        } else if !is_package_eq(item, &self.name, self.rename.as_deref()) {
            // No existing keys are relevant when the package changes
            *item = self.to_toml(crate_root);
        } else if let Some(table) = item.as_table_like_mut() {
            match &self.source {
                DependencySource::Version {
                    version,
                    path,
                    registry,
                } => {
                    if let Some(v) = version {
                        table.insert("version", toml_edit::value(v));
                    } else {
                        table.remove("version");
                    }
                    if let Some(p) = path {
                        let relpath = path_field(crate_root, p);
                        table.insert("path", toml_edit::value(relpath));
                    } else {
                        table.remove("path");
                    }
                    if let Some(r) = registry {
                        table.insert("registry", toml_edit::value(r));
                    }
                    for key in ["git", "branch", "tag", "rev"] {
                        table.remove(key);
                    }
                }
                DependencySource::Git {
                    repo,
                    branch,
                    tag,
                    rev,
                } => {
                    table.insert("git", toml_edit::value(repo));
                    if let Some(branch) = branch {
                        table.insert("branch", toml_edit::value(branch));
                    } else {
                        table.remove("branch");
                    }
                    if let Some(tag) = tag {
                        table.insert("tag", toml_edit::value(tag));
                    } else {
                        table.remove("tag");
                    }
                    if let Some(rev) = rev {
                        table.insert("rev", toml_edit::value(rev));
                    } else {
                        table.remove("rev");
                    }
                    for key in ["version", "path", "registry"] {
                        table.remove(key);
                    }
                }
            }
            if self.rename.is_some() {
                table.insert("package", toml_edit::value(self.name.as_str()));
            }
            match self.default_features {
                Some(true) => {
                    table.remove("default-features");
                }
                Some(false) => {
                    table.insert("default-features", toml_edit::value(false));
                }
                None => {}
            }
            if let Some(new_features) = self.features.as_deref() {
                let mut features = table
                    .get("features")
                    .and_then(|i| i.as_value())
                    .and_then(|v| v.as_array())
                    .and_then(|a| {
                        a.iter()
                            .map(|v| v.as_str())
                            .collect::<Option<indexmap::IndexSet<_>>>()
                    })
                    .unwrap_or_default();
                features.extend(new_features.iter().map(|s| s.as_str()));
                let features = toml_edit::value(features.into_iter().collect::<toml_edit::Value>());
                table.insert("features", features);
            }
            match self.optional {
                Some(true) => {
                    table.insert("optional", toml_edit::value(true));
                }
                Some(false) => {
                    table.remove("optional");
                }
                None => {}
            }

            table.fmt();
        } else {
            unreachable!("Invalid dependency type: {}", item.type_name());
        }
    }
}

fn path_field(crate_root: &Path, abs_path: &Path) -> String {
    let relpath = pathdiff::diff_paths(abs_path, crate_root).expect("both paths are absolute");
    let relpath = relpath.to_str().unwrap().replace('\\', "/");
    relpath
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

impl Default for Dependency {
    fn default() -> Dependency {
        Dependency {
            name: "".into(),
            rename: None,
            optional: None,
            features: None,
            default_features: None,
            source: DependencySource::Version {
                version: None,
                path: None,
                registry: None,
            },
            available_features: BTreeMap::default(),
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
    use super::super::dependency::Dependency;
    use std::path::Path;

    #[test]
    fn to_toml_simple_dep() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let dep = Dependency::new("dep");
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        assert_eq!(key, "dep".to_owned());

        verify_roundtrip(&crate_root, key, &item);
    }

    #[test]
    fn to_toml_simple_dep_with_version() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let dep = Dependency::new("dep").set_version("1.0");
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        assert_eq!(key, "dep".to_owned());
        assert_eq!(item.as_str(), Some("1.0"));

        verify_roundtrip(&crate_root, key, &item);
    }

    #[test]
    fn to_toml_optional_dep() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let dep = Dependency::new("dep").set_optional(Some(true));
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        assert_eq!(key, "dep".to_owned());
        assert!(item.is_inline_table());

        let dep = item.as_inline_table().unwrap();
        assert_eq!(dep.get("optional").unwrap().as_bool(), Some(true));

        verify_roundtrip(&crate_root, key, &item);
    }

    #[test]
    fn to_toml_dep_without_default_features() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let dep = Dependency::new("dep").set_default_features(Some(false));
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        assert_eq!(key, "dep".to_owned());
        assert!(item.is_inline_table());

        let dep = item.as_inline_table().unwrap();
        assert_eq!(dep.get("default-features").unwrap().as_bool(), Some(false));

        verify_roundtrip(&crate_root, key, &item);
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

        verify_roundtrip(&crate_root, key, &item);
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

        verify_roundtrip(&crate_root, key, &item);
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

        verify_roundtrip(&crate_root, key, &item);
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

        verify_roundtrip(&crate_root, key, &item);
    }

    #[test]
    fn to_toml_complex_dep() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let dep = Dependency::new("dep")
            .set_version("1.0")
            .set_default_features(Some(false))
            .set_rename("d");
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        assert_eq!(key, "d".to_owned());
        assert!(item.is_inline_table());

        let dep = item.as_inline_table().unwrap();
        assert_eq!(dep.get("package").unwrap().as_str(), Some("dep"));
        assert_eq!(dep.get("version").unwrap().as_str(), Some("1.0"));
        assert_eq!(dep.get("default-features").unwrap().as_bool(), Some(false));

        verify_roundtrip(&crate_root, key, &item);
    }

    #[test]
    fn paths_with_forward_slashes_are_left_as_is() {
        let crate_root = dunce::canonicalize(Path::new("/")).expect("root exists");
        let path = crate_root.join("sibling/crate");
        let relpath = "sibling/crate";
        let dep = Dependency::new("dep").set_path(path);
        let key = dep.toml_key();
        let item = dep.to_toml(&crate_root);

        let table = item.as_inline_table().unwrap();
        let got = table.get("path").unwrap().as_str().unwrap();
        assert_eq!(got, relpath);

        verify_roundtrip(&crate_root, key, &item);
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

        verify_roundtrip(&crate_root, key, &item);
    }

    fn verify_roundtrip(crate_root: &Path, key: &str, item: &toml_edit::Item) {
        let roundtrip = Dependency::from_toml(crate_root, key, item).unwrap();
        let round_key = roundtrip.toml_key();
        let round_item = roundtrip.to_toml(crate_root);
        assert_eq!(key, round_key);
        assert_eq!(item.to_string(), round_item.to_string());
    }
}
