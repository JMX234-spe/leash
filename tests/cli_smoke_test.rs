use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help_displays_subcommands() {
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("run"))
        .stdout(predicate::str::contains("checkpoints"))
        .stdout(predicate::str::contains("rewind"))
        .stdout(predicate::str::contains("log"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("leash 0.1.0"));
}

#[test]
fn test_init_stub() {
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("leash init: not implemented yet"));
}
