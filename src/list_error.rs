use std::fmt;
use std::error::Error;

#[derive(Debug)]
pub enum ListError {
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
