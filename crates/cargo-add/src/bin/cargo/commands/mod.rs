use cargo_add::ops::cargo_add::CargoResult;

pub mod add;

pub fn builtin() -> [clap::Command<'static>; 1] {
    [add::cli()]
}

pub fn builtin_exec(cmd: &str) -> Option<fn(&clap::ArgMatches) -> CargoResult<()>> {
    let f = match cmd {
        "add" => add::exec,
        _ => return None,
    };
    Some(f)
}
