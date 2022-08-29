use cargo::CliResult;

pub mod rm;

pub fn builtin() -> [clap::Command<'static>; 1] {
    [rm::cli()]
}

pub fn builtin_exec(cmd: &str) -> Option<fn(&mut cargo::Config, &clap::ArgMatches) -> CliResult> {
    let f = match cmd {
        "rm" => rm::exec,
        _ => return None,
    };
    Some(f)
}
