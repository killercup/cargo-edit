use std::process::Command;

#[test]
fn test_from_shell() {
    Command::new("tests/cargo_add.sh")
        .output()
        .and_then(|output| {
            if !output.status.success() {
                panic!("Shell test failed:\n{}",
                       String::from_utf8_lossy(&output.stdout));
            } else {
                Ok(())
            }
        })
        .unwrap();
}
