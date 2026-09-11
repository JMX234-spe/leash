use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn init_git_repo(path: &std::path::Path) {
    let run_git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(path)
            .status()
            .expect("Failed to execute git command");
        assert!(status.success());
    };

    run_git(&["init"]);
    run_git(&["config", "user.name", "CLI Tester"]);
    run_git(&["config", "user.email", "clitester@example.com"]);
    run_git(&["config", "core.autocrlf", "false"]);

    let initial_file = path.join("init.txt");
    std::fs::write(&initial_file, "init\n").unwrap();
    run_git(&["add", "init.txt"]);
    run_git(&["commit", "-m", "init commit"]);
}

#[test]
fn test_cli_help_displays_all_subcommands() {
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
fn test_cli_version_outputs_crate_version() {
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("leash 0.1.0"));
}

#[test]
fn test_cli_init_generates_default_policy_and_respects_force_flag() {
    let temp_dir = tempdir().unwrap();

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
fn test_cli_run_blocks_denied_command_with_exit_126() {
    let temp_dir = tempdir().unwrap();

    let mut init_cmd = Command::cargo_bin("leash").unwrap();
    init_cmd
        .current_dir(temp_dir.path())
        .arg("init")
        .assert()
        .success();

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
fn test_cli_run_treats_ask_as_deny_in_non_interactive_session() {
    let temp_dir = tempdir().unwrap();

    let mut init_cmd = Command::cargo_bin("leash").unwrap();
    init_cmd
        .current_dir(temp_dir.path())
        .arg("init")
        .assert()
        .success();

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
fn test_cli_run_executes_basic_command_in_pty() {
    let temp_dir = tempdir().unwrap();
    init_git_repo(temp_dir.path());

    let shell = if cfg!(windows) { "sh" } else { "bash" };
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.current_dir(temp_dir.path())
        .args(["run", "--", shell, "-c", "echo hola"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hola"));
}

#[test]
fn test_cli_run_handles_interactive_stdin_and_stdout() {
    let temp_dir = tempdir().unwrap();
    init_git_repo(temp_dir.path());

    let python = if cfg!(windows) { "python" } else { "python3" };
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.current_dir(temp_dir.path())
        .args([
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
fn test_cli_run_supports_multi_turn_repl_session() {
    let temp_dir = tempdir().unwrap();
    init_git_repo(temp_dir.path());

    let python = if cfg!(windows) { "python" } else { "python3" };
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.current_dir(temp_dir.path())
        .args(["run", "--", python])
        .write_stdin("x = 100 + 234\r\nprint(f'CALC_RESULT={x}')\r\nexit()\r\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("CALC_RESULT=334"));
}

#[test]
fn test_cli_run_propagates_non_zero_exit_code() {
    let temp_dir = tempdir().unwrap();
    init_git_repo(temp_dir.path());

    let shell = if cfg!(windows) { "sh" } else { "bash" };
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.current_dir(temp_dir.path())
        .args(["run", "--", shell, "-c", "exit 42"])
        .assert()
        .code(42);
}

#[test]
fn test_cli_checkpoints_and_rewind_restores_filesystem_state() {
    let temp_dir = tempdir().unwrap();

    init_git_repo(temp_dir.path());

    let mut init_cmd = Command::cargo_bin("leash").unwrap();
    init_cmd
        .current_dir(temp_dir.path())
        .arg("init")
        .assert()
        .success();

    let app_file = temp_dir.path().join("app.txt");
    std::fs::write(&app_file, "version 1.0\n").unwrap();

    let shell = if cfg!(windows) { "sh" } else { "bash" };
    let mut run_cmd = Command::cargo_bin("leash").unwrap();
    run_cmd
        .current_dir(temp_dir.path())
        .args(["run", "--", shell, "-c", "echo hello"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Created pre-execution checkpoint"));

    let mut cp_cmd = Command::cargo_bin("leash").unwrap();
    let output = cp_cmd
        .current_dir(temp_dir.path())
        .arg("checkpoints")
        .assert()
        .success()
        .stdout(predicate::str::contains("before:"))
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    let first_line = output_str
        .lines()
        .find(|l| l.contains("before:"))
        .expect("No checkpoint row found");
    let checkpoint_id = first_line.split_whitespace().next().unwrap();

    std::fs::write(&app_file, "version 2.0 corrupted\n").unwrap();
    let unwanted_file = temp_dir.path().join("unwanted.junk");
    std::fs::write(&unwanted_file, "bad").unwrap();

    let mut rewind_cmd = Command::cargo_bin("leash").unwrap();
    rewind_cmd
        .current_dir(temp_dir.path())
        .args(["rewind", checkpoint_id, "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully rewound"));

    let restored = std::fs::read_to_string(&app_file)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(restored, "version 1.0\n");
    assert!(!unwanted_file.exists());
}

#[test]
fn test_cli_commands_outside_git_repo_show_clean_error() {
    let empty_temp = tempdir().unwrap();

    let mut run_cmd = Command::cargo_bin("leash").unwrap();
    run_cmd
        .current_dir(empty_temp.path())
        .args(["run", "--", "echo", "hello"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "[LEASH ERROR] Leash requires a git repository. Run 'git init' first.",
        ));

    let mut cp_cmd = Command::cargo_bin("leash").unwrap();
    cp_cmd
        .current_dir(empty_temp.path())
        .arg("checkpoints")
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "[LEASH ERROR] Leash requires a git repository. Run 'git init' first.",
        ));

    let mut rw_cmd = Command::cargo_bin("leash").unwrap();
    rw_cmd
        .current_dir(empty_temp.path())
        .args(["rewind", "dummy123", "--yes"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "[LEASH ERROR] Leash requires a git repository. Run 'git init' first.",
        ));
}

#[test]
fn test_cli_log_indicates_empty_when_no_records_exist() {
    let temp_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("leash").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("log")
        .assert()
        .success()
        .stdout(predicate::str::contains("No session log entries found."));
}

#[test]
fn test_cli_log_records_full_lifecycle_and_supports_json_and_tail() {
    let temp_dir = tempdir().unwrap();
    init_git_repo(temp_dir.path());

    let mut run_cmd = Command::cargo_bin("leash").unwrap();
    run_cmd
        .current_dir(temp_dir.path())
        .args(["run", "--", "echo", "log_test_command"])
        .assert()
        .success();

    let log_file = temp_dir.path().join(".leash").join("log.jsonl");
    assert!(log_file.exists(), ".leash/log.jsonl should be created");

    let mut log_cmd = Command::cargo_bin("leash").unwrap();
    let log_output = log_cmd
        .current_dir(temp_dir.path())
        .arg("log")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let log_str = String::from_utf8_lossy(&log_output);
    assert!(log_str.contains("START"));
    assert!(log_str.contains("log_test_command"));
    assert!(log_str.contains("POLICY"));
    assert!(log_str.contains("allow"));
    assert!(log_str.contains("CHECKPOINT"));
    assert!(log_str.contains("END"));
    assert!(log_str.contains("exit code: 0"));

    let mut json_cmd = Command::cargo_bin("leash").unwrap();
    json_cmd
        .current_dir(temp_dir.path())
        .args(["log", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"event\":\"session_started\""))
        .stdout(predicate::str::contains("\"event\":\"command_evaluated\""))
        .stdout(predicate::str::contains("\"event\":\"checkpoint_created\""))
        .stdout(predicate::str::contains("\"event\":\"session_ended\""));

    let mut cp_cmd = Command::cargo_bin("leash").unwrap();
    let cp_output = cp_cmd
        .current_dir(temp_dir.path())
        .arg("checkpoints")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let cp_str = String::from_utf8_lossy(&cp_output);
    let cp_line = cp_str
        .lines()
        .find(|l| l.contains("before: echo log_test_command"))
        .expect("Checkpoint line not found");
    let checkpoint_id = cp_line.split_whitespace().next().unwrap();

    let mut rw_cmd = Command::cargo_bin("leash").unwrap();
    rw_cmd
        .current_dir(temp_dir.path())
        .args(["rewind", checkpoint_id, "--yes"])
        .assert()
        .success();

    let mut tail_cmd = Command::cargo_bin("leash").unwrap();
    tail_cmd
        .current_dir(temp_dir.path())
        .args(["log", "-n", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("REWIND"))
        .stdout(predicate::str::contains("restored to"));
}
