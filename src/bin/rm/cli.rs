use cargo_edit::CargoResult;
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(bin_name = "cargo")]
pub enum Command {
    Rm(crate::rm::RmArgs),
}

impl Command {
    pub fn exec(self) -> CargoResult<()> {
        match self {
            Self::Rm(add) => add.exec(),
        }
    }
}

#[test]
fn verify_app() {
    use clap::IntoApp;
    Command::into_app().debug_assert()
}
