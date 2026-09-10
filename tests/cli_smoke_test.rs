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

#[test]
fn test_run_echo_hola() {
    let shell = if cfg!(windows) { "sh" } else { "bash" };
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.args(["run", "--", shell, "-c", "echo hola"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hola"));
}

#[test]
fn test_run_interactive_python() {
    let python = if cfg!(windows) { "python" } else { "python3" };
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.args([
        "run",
        "--",
        python,
        "-c",
        "import sys; val = sys.stdin.readline().strip(); print(f'REPL_ECHO: {val}')",
    ])
    .write_stdin("leash_interactive_test\r\n")
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "REPL_ECHO: leash_interactive_test",
    ));
}

#[test]
fn test_run_python_repl() {
    let python = if cfg!(windows) { "python" } else { "python3" };
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.args(["run", "--", python])
        .write_stdin("x = 100 + 234\r\nprint(f'CALC_RESULT={x}')\r\nexit()\r\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("CALC_RESULT=334"));
}

#[test]
fn test_run_propagates_non_zero_exit_code() {
    let shell = if cfg!(windows) { "sh" } else { "bash" };
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.args(["run", "--", shell, "-c", "exit 42"])
        .assert()
        .code(42);
}
