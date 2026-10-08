use std::process::Command;

#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute cargo run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bulk-telnet-ssh"));
    assert!(stdout.contains("--config"));
    assert!(stdout.contains("--concurrency"));
    assert!(stdout.contains("--timeout"));
}
