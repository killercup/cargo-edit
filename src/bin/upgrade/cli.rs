use cargo_edit::CargoResult;
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(bin_name = "cargo")]
pub enum Command {
    Upgrade(crate::upgrade::UpgradeArgs),
}

impl Command {
    pub fn exec(self) -> CargoResult<()> {
        match self {
            Self::Upgrade(add) => add.exec(),
        }
    }
}

#[test]
fn verify_app() {
    use clap::IntoApp;
    Command::into_app().debug_assert()
}
