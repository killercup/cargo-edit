use std::error::Error;

use pad::{PadStr, Alignment};
use toml;

use manifest::Manifest;
use list_error::ListError;

#[allow(deprecated)] // connect -> join
pub fn list_section(manifest: &Manifest, section: &str) -> Result<String, Box<Error>> {
    let section = String::from(section);
    let mut output = vec![];

    let list = try!(
        manifest.data.get(&section)
        .and_then(|field| field.as_table() )
        .ok_or(ListError::SectionMissing(section))
    );

    let name_max_len = list.keys().map(|k| k.len()).max().unwrap_or(0);

    for (name, val) in list {
        let version = match *val {
            toml::Value::String(ref version) => version.to_owned(),
            toml::Value::Table(_) => {
                let v = try!(
                    val.lookup("version")
                    .and_then(|field| field.as_str())
                    .or_else(|| val.lookup("git").map(|_| "git"))
                    .ok_or(ListError::VersionMissing(name.clone()))
                );
                String::from(v)
            },
            _ => String::from("")
        };

        output.push(format!("{name} {version}",
            name = name.pad_to_width_with_alignment(name_max_len, Alignment::Left),
            version = version));
    }

    Ok(output.connect("\n"))
}

#[cfg(test)]
mod test {
    use manifest::Manifest;
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
        let manifile = Manifest::from_str(DEFAULT_CARGO_TOML).unwrap();

        assert_eq!(
            list_section(&manifile, "dependencies").unwrap(), "\
foo-bar     0.1
lorem-ipsum 0.4.2"
        );
    }

    #[test]
    #[should_panic]
    fn unknown_section() {
        let manifile = Manifest::from_str(DEFAULT_CARGO_TOML).unwrap();

        list_section(&manifile, "lol-dependencies").unwrap();
    }
}
