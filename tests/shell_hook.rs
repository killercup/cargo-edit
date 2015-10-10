use std::env;
use std::process::Command;

#[test]
fn test_from_shell() {
    if let Ok(_) = env::var("COVERAGE_TESTING") {
        println!("Coverage doesn't work with sub shells. Skipping.");
        return;
    }

    Command::new("tests/cargo_add.sh")
    .output().and_then(|output| {
        if !output.status.success() {
            panic!("Shell test failed:\n{}", String::from_utf8_lossy(&output.stdout));
        } else { Ok(()) }
    }).unwrap();
}
