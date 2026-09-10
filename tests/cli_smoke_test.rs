use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

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
fn test_init_creates_policy_file() {
    let temp_dir = tempdir().unwrap();

    // First init should succeed
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized Leash configuration"));

    let policy_path = temp_dir.path().join(".leash").join("policy.yaml");
    assert!(policy_path.exists());
    let content = std::fs::read_to_string(&policy_path).unwrap();
    assert!(content.contains("block-destructive-rm"));

    // Second init without --force should fail with exit code 1
    let mut cmd2 = Command::cargo_bin("leash").unwrap();
    cmd2.current_dir(temp_dir.path())
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    // Init with --force should succeed
    let mut cmd3 = Command::cargo_bin("leash").unwrap();
    cmd3.current_dir(temp_dir.path())
        .args(["init", "--force"])
        .assert()
        .success();
}

#[test]
fn test_run_denied_command_blocked() {
    let temp_dir = tempdir().unwrap();

    // Initialize policy
    let mut init_cmd = Command::cargo_bin("leash").unwrap();
    init_cmd
        .current_dir(temp_dir.path())
        .arg("init")
        .assert()
        .success();

    // Command matching deny rule
    let mut run_cmd = Command::cargo_bin("leash").unwrap();
    run_cmd
        .current_dir(temp_dir.path())
        .args(["run", "--", "rm", "-rf", "/"])
        .assert()
        .code(126)
        .stderr(predicate::str::contains("[LEASH BLOCKED]"))
        .stderr(predicate::str::contains("block-destructive-rm"));
}

#[test]
fn test_run_ask_command_non_interactive_treated_as_deny() {
    let temp_dir = tempdir().unwrap();

    let mut init_cmd = Command::cargo_bin("leash").unwrap();
    init_cmd
        .current_dir(temp_dir.path())
        .arg("init")
        .assert()
        .success();

    // In automated runners (non-interactive stdin), 'ask' rules are treated as deny
    // to prevent hanging pipelines.
    let mut run_cmd = Command::cargo_bin("leash").unwrap();
    run_cmd
        .current_dir(temp_dir.path())
        .args(["run", "--", "git", "push", "origin", "main", "--force"])
        .assert()
        .code(126)
        .stderr(predicate::str::contains("[LEASH PROMPT]"))
        .stderr(predicate::str::contains(
            "Non-interactive session detected; treating 'ask' policy as deny.",
        ));
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
