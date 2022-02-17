use cargo_add::ops::cargo_add::CargoResult;
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(bin_name = "cargo")]
pub enum Command {
    Add(crate::commands::add::AddArgs),
}

impl Command {
    pub fn exec(self) -> CargoResult<()> {
        match self {
            Self::Add(add) => add.exec(),
        }
    }
}

#[test]
fn verify_app() {
    use clap::CommandFactory;
    Command::command().debug_assert()
}
