use cargo::CliResult;

pub mod remove;

pub fn builtin() -> [clap::Command<'static>; 1] {
    [remove::cli()]
}

pub fn builtin_exec(cmd: &str) -> Option<fn(&mut cargo::Config, &clap::ArgMatches) -> CliResult> {
    let f = match cmd {
        "remove" => remove::exec,
        _ => return None,
    };
    Some(f)
}
