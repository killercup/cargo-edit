
use std::fmt;

use cargo_edit::Manifest;
use list_error::ListError;
use pad::{Alignment, PadStr};
use toml;

enum Source {
    Version(String),
    Git(String),
    Path(String),
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Source::Version(ref v) => v.to_owned(),
            Source::Git(ref g) => format!("git: {}", g),
            Source::Path(ref g) => format!("path: {}", g),
        })
    }
}

struct Dependency {
    name: String,
    version: Source,
    optional: bool,
}

/// A set of dependencies parsed
pub struct Dependencies(Vec<Dependency>, usize);

impl fmt::Display for Dependencies {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output: Vec<_> = self.0.iter().map(|dep| {
            format!("{name} {version}{optional}",
                    name = dep.name.pad_to_width_with_alignment(self.1, Alignment::Left),
                    version = dep.version,
                    optional = if dep.optional { " (optional)" } else { "" })
        }).collect();

        write!(f, "{}", output.join("\n"))
    }
}

/// Parse the manifest to extract the dependencies.
pub fn list_section(manifest: &Manifest, section: &str) -> Result<Dependencies, ListError> {
    let list = try!(manifest.data
        .get(section)
        .and_then(|field| field.as_table())
        .ok_or_else(|| ListError::SectionMissing(String::from(section))));

    let name_max_len = list.keys().map(String::len).max().unwrap_or(0);

    let deps = list.iter().map(|(name, val)| {
        let version = match *val {
            toml::Value::String(ref version) => Source::Version(version.to_owned()),
            toml::Value::Table(_) => {
                try!(val.get("version")
                    .and_then(|field| field.as_str().map(|s| Source::Version(s.to_owned())))
                    .or_else(|| val.get("git").map(|g| Source::Git(g.to_string())))
                    .or_else(|| val.get("path").map(|p| Source::Path(p.to_string())))
                    .ok_or_else(|| ListError::VersionMissing(name.clone(), section.to_owned())))
            }
            _ => Source::Version(String::new()),
        };

        let optional = if let toml::Value::Table(_) = *val {
            val.get("optional")
                .and_then(|field| field.as_bool())
                .unwrap_or(false)
        } else {
            false
        };

        Ok(Dependency {
            name: name.to_owned(),
            version,
            optional,
        })
    }).collect::<Result<Vec<Dependency>, ListError>>()?;

    Ok(Dependencies(deps, name_max_len))
}

#[cfg(test)]
mod test {
    use cargo_edit::Manifest;
    use super::list_section;

    static DEFAULT_CARGO_TOML: &'static str = r#"[package]
authors = ["Some Guy"]
name = "lorem-ipsum"
version = "0.1.0"

[dependencies]
foo-bar = "0.1"
lorem-ipsum = "0.4.2""#;

    #[test]
    fn basic_listing() {
        let manifile: Manifest = DEFAULT_CARGO_TOML.parse().unwrap();

        assert_eq!(list_section(&manifile, "dependencies").unwrap().to_string(),
                   "\
foo-bar     0.1
lorem-ipsum 0.4.2");
    }

    #[test]
    #[should_panic]
    fn unknown_section() {
        let manifile: Manifest = DEFAULT_CARGO_TOML.parse().unwrap();

        list_section(&manifile, "lol-dependencies").unwrap();
    }
}
