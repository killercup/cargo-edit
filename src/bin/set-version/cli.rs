use cargo_edit::CargoResult;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(bin_name = "cargo")]
pub enum Command {
    SetVersion(crate::set_version::VersionArgs),
}

impl Command {
    pub fn exec(self) -> CargoResult<()> {
        match self {
            Self::SetVersion(add) => add.exec(),
        }
    }
}

#[test]
fn verify_app() {
    use clap::CommandFactory;
    Command::command().debug_assert()
}
