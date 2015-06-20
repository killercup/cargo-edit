use std::fmt;
use std::error::Error;

use toml;

use manifest::Manifest;

#[derive(Debug)]
enum ListError {
    SectionMissing(String),
    VersionMissing(String),
}

impl Error for ListError {
    fn description(&self) -> &'static str {
        /*let desc: String = match *self {
            ListError::SectionMissing(ref name) => format!("Couldn't read section {}", name),
            ListError::VersionMissing(ref name) => format!("Couldn't read version of {}", name),
        };
        &desc*/
        match *self {
            ListError::SectionMissing(_) => "Couldn't read section",
            ListError::VersionMissing(_) => "Couldn't read version",
        }
    }
}

impl fmt::Display for ListError {
    fn fmt(&self, format: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        format.write_str(self.description())
    }
}


pub fn list_section(manifest: &Manifest, section: &str) -> Result<(), Box<Error>> {
    let section = String::from(section);

    let list = try!(
        manifest.data.get(&section)
        .and_then(|field| field.as_table() )
        .ok_or(ListError::SectionMissing(section))
    );

    for (name, val) in list {
        let version = match *val {
            toml::Value::String(ref version) => version.to_string(),
            toml::Value::Table(_) => {
                let v = try!(
                    val.lookup("version")
                    .and_then(|field| field.as_str())
                    .ok_or(ListError::VersionMissing(name.clone()))
                );
                String::from(v)
            },
            _ => String::from("")
        };

        println!("{name} ({version})", name = name, version = version);
    }

    Ok(())
}
