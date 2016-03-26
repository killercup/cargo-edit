use pad::{Alignment, PadStr};
use toml;

use cargo_edit::Manifest;
use list_error::ListError;

/// List the dependencies for manifest section
#[allow(deprecated)] // connect -> join
pub fn list_section(manifest: &Manifest, section: &str) -> Result<String, ListError> {
    let mut output = vec![];

    let list = try!(manifest.data
                            .get(section)
                            .and_then(|field| field.as_table())
                            .ok_or_else(|| ListError::SectionMissing(String::from(section))));

    let name_max_len = list.keys().map(|k| k.len()).max().unwrap_or(0);

    for (name, val) in list {
        let version = match *val {
            toml::Value::String(ref version) => version.clone(),
            toml::Value::Table(_) => {
                try!(val.lookup("version")
                        .and_then(|field| field.as_str().map(|s| s.to_owned()))
                        .or_else(|| val.lookup("git").map(|repo| format!("git: {}", repo)))
                        .or_else(|| val.lookup("path").map(|path| format!("path: {}", path)))
                        .ok_or_else(|| ListError::VersionMissing(name.clone(), section.to_owned())))
            }
            _ => String::from(""),
        };

        let optional = if let toml::Value::Table(_) = *val {
            val.lookup("optional")
               .and_then(|field| field.as_bool())
               .unwrap_or(false)
        } else {
            false
        };

        output.push(format!("{name} {version}{optional}",
                            name = name.pad_to_width_with_alignment(name_max_len,
                                                                    Alignment::Left),
                            version = version,
                            optional = if optional {
                                " (optional)"
                            } else {
                                ""
                            }));
    }

    Ok(output.connect("\n"))
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

        assert_eq!(list_section(&manifile, "dependencies").unwrap(),
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
