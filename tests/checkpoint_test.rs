use git2::{Repository, Signature};
use leash::checkpoint::GitCheckpointBackend;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn setup_git_repo(path: &Path) -> Repository {
    let repo = Repository::init(path).expect("Failed to init git repo");
    let mut config = repo.config().expect("Failed to get config");
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    config.set_bool("core.autocrlf", false).unwrap();

    // Create an initial commit so HEAD is valid
    let initial_file = path.join("README.md");
    fs::write(&initial_file, "# Test Repo\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new("README.md")).unwrap();
    let tree_id = index.write_tree().unwrap();
    {
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();
    }

    repo
}

#[test]
fn test_create_and_restore_checkpoint_restores_working_tree_state() {
    let temp_dir = tempdir().unwrap();
    let repo = setup_git_repo(temp_dir.path());
    let backend = GitCheckpointBackend::discover(temp_dir.path()).expect("Failed to discover repo");

    let target_file = temp_dir.path().join("code.rs");

    // Write initial content and create checkpoint
    fs::write(&target_file, "fn main() { println!(\"version 1\"); }\n").unwrap();
    let checkpoint_v1 = backend
        .create_checkpoint("sess-001", "checkpoint v1")
        .expect("Failed to create checkpoint 1");

    assert!(!checkpoint_v1.id.is_empty());
    assert_eq!(checkpoint_v1.session_id, "sess-001");
    assert_eq!(checkpoint_v1.description, "checkpoint v1");

    // Verify checkpoint is listed
    let checkpoints_after_v1 = backend.list_checkpoints(None).unwrap();
    assert_eq!(checkpoints_after_v1.len(), 1);
    assert_eq!(checkpoints_after_v1[0].id, checkpoint_v1.id);

    // Modify file and add another file, then create checkpoint 2
    fs::write(&target_file, "fn main() { println!(\"version 2\"); }\n").unwrap();
    let extra_file = temp_dir.path().join("extra.txt");
    fs::write(&extra_file, "extra content").unwrap();

    let checkpoint_v2 = backend
        .create_checkpoint("sess-001", "checkpoint v2")
        .expect("Failed to create checkpoint 2");

    let checkpoints_after_v2 = backend.list_checkpoints(None).unwrap();
    assert_eq!(checkpoints_after_v2.len(), 2);
    assert_eq!(checkpoints_after_v2[0].id, checkpoint_v2.id);

    // Make unwanted destructive changes
    fs::write(&target_file, "fn corrupted() { panic!(); }\n").unwrap();
    let junk_file = temp_dir.path().join("junk.tmp");
    fs::write(&junk_file, "garbage").unwrap();

    assert_eq!(
        fs::read_to_string(&target_file).unwrap(),
        "fn corrupted() { panic!(); }\n"
    );
    assert!(junk_file.exists());

    // Rewind to checkpoint 2
    backend
        .restore_checkpoint(&checkpoint_v2.id)
        .expect("Failed to restore checkpoint 2");

    // Verify state restored to checkpoint 2
    let content_cp2 = fs::read_to_string(&target_file)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(content_cp2, "fn main() { println!(\"version 2\"); }\n");
    assert!(extra_file.exists());
    assert!(
        !junk_file.exists(),
        "Untracked junk file should be removed on rewind"
    );

    // Rewind back to checkpoint 1
    backend
        .restore_checkpoint(&checkpoint_v1.id)
        .expect("Failed to restore checkpoint 1");

    // Verify state restored to checkpoint 1
    let content_cp1 = fs::read_to_string(&target_file)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(content_cp1, "fn main() { println!(\"version 1\"); }\n");
    assert!(
        !extra_file.exists(),
        "extra_file did not exist at cp1 and should be removed"
    );

    // Verify user HEAD branch was never changed
    let head = repo.head().unwrap();
    assert_eq!(head.shorthand().unwrap(), "master");
    let head_commit = head.peel_to_commit().unwrap();
    assert_eq!(head_commit.summary().unwrap(), "Initial commit");
}

#[test]
fn test_list_checkpoints_filters_by_session_id() {
    let temp_dir = tempdir().unwrap();
    let _repo = setup_git_repo(temp_dir.path());
    let backend = GitCheckpointBackend::discover(temp_dir.path()).unwrap();

    let file = temp_dir.path().join("file.txt");
    fs::write(&file, "sess 1").unwrap();
    let checkpoint_a = backend.create_checkpoint("session-a", "task a").unwrap();

    fs::write(&file, "sess 2").unwrap();
    let checkpoint_b = backend.create_checkpoint("session-b", "task b").unwrap();

    let list_all = backend.list_checkpoints(None).unwrap();
    assert_eq!(list_all.len(), 2);

    let list_a = backend.list_checkpoints(Some("session-a")).unwrap();
    assert_eq!(list_a.len(), 1);
    assert_eq!(list_a[0].id, checkpoint_a.id);

    let list_b = backend.list_checkpoints(Some("session-b")).unwrap();
    assert_eq!(list_b.len(), 1);
    assert_eq!(list_b[0].id, checkpoint_b.id);

    let list_none = backend.list_checkpoints(Some("session-c")).unwrap();
    assert!(list_none.is_empty());
}

#[test]
fn test_create_and_restore_checkpoint_works_in_unborn_repository() {
    let temp_dir = tempdir().unwrap();
    let repo = Repository::init(temp_dir.path()).expect("Failed to init git repo");
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    config.set_bool("core.autocrlf", false).unwrap();

    let backend = GitCheckpointBackend::discover(temp_dir.path()).unwrap();
    let file = temp_dir.path().join("first_code.rs");
    fs::write(&file, "fn first() {}\n").unwrap();

    let checkpoint = backend
        .create_checkpoint("session-unborn", "initial workdir")
        .expect("Failed to create checkpoint on unborn repo");
    assert!(!checkpoint.id.is_empty());

    let recorded_checkpoints = backend.list_checkpoints(None).unwrap();
    assert_eq!(recorded_checkpoints.len(), 1);
    assert_eq!(recorded_checkpoints[0].id, checkpoint.id);

    fs::write(&file, "fn corrupted() {}\n").unwrap();
    backend
        .restore_checkpoint(&checkpoint.id)
        .expect("Failed to restore checkpoint on unborn repo");

    let restored = fs::read_to_string(&file).unwrap().replace("\r\n", "\n");
    assert_eq!(restored, "fn first() {}\n");
}

#[test]
fn test_checkpoint_snapshot_excludes_leash_directory() {
    let temp_dir = tempdir().unwrap();
    let repo = setup_git_repo(temp_dir.path());
    let backend = GitCheckpointBackend::discover(temp_dir.path()).unwrap();

    let leash_dir = temp_dir.path().join(".leash");
    fs::create_dir_all(&leash_dir).unwrap();
    let policy_file = leash_dir.join("policy.yaml");
    let log_file = leash_dir.join("log.jsonl");
    fs::write(&policy_file, "version: 1\n").unwrap();
    fs::write(&log_file, "{\"event\":\"initial\"}\n").unwrap();

    let code_file = temp_dir.path().join("main.rs");
    fs::write(&code_file, "fn main() { 1 }\n").unwrap();

    let checkpoint = backend
        .create_checkpoint("sess-leash", "test exclude .leash")
        .unwrap();

    let commit_obj = repo.revparse_single(&checkpoint.id).unwrap();
    let commit = commit_obj.peel_to_commit().unwrap();
    let tree = commit.tree().unwrap();
    assert!(
        tree.get_name(".leash").is_none(),
        "Checkpoint git tree must NOT contain .leash directory"
    );

    fs::write(
        &log_file,
        "{\"event\":\"initial\"}\n{\"event\":\"second\"}\n",
    )
    .unwrap();
    fs::write(&code_file, "fn main() { 2 }\n").unwrap();

    backend.restore_checkpoint(&checkpoint.id).unwrap();

    let code = fs::read_to_string(&code_file)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(code, "fn main() { 1 }\n");

    assert!(
        policy_file.exists(),
        ".leash/policy.yaml must not be deleted on rewind"
    );
    assert!(
        log_file.exists(),
        ".leash/log.jsonl must not be deleted on rewind"
    );
    let log_content = fs::read_to_string(&log_file).unwrap().replace("\r\n", "\n");
    assert_eq!(
        log_content, "{\"event\":\"initial\"}\n{\"event\":\"second\"}\n",
        "Audit log must remain append-only and not be reverted by rewind"
    );
}

#[test]
fn test_restore_checkpoint_creates_automatic_safety_backup() {
    let temp_dir = tempdir().unwrap();
    let _repo = setup_git_repo(temp_dir.path());
    let backend = GitCheckpointBackend::discover(temp_dir.path()).unwrap();

    let target_file = temp_dir.path().join("code.rs");
    fs::write(&target_file, "version 1\n").unwrap();
    let checkpoint_v1 = backend.create_checkpoint("s1", "v1 commit").unwrap();

    fs::write(&target_file, "version 2 with uncommitted extra\n").unwrap();

    backend.restore_checkpoint(&checkpoint_v1.id).unwrap();

    let checkpoints = backend.list_checkpoints(None).unwrap();
    let backup_checkpoint = checkpoints
        .iter()
        .find(|checkpoint| {
            checkpoint
                .description
                .contains("pre-rewind backup: before restoring")
        })
        .expect("Must have created automatic pre-rewind safety checkpoint");

    assert_eq!(backup_checkpoint.session_id, "rewind-backup");
    assert!(backup_checkpoint.description.contains(&checkpoint_v1.id));
}

#[test]
fn test_leash_log_preserved_when_checkout_fails() {
    let temp_dir = tempdir().unwrap();
    let _repo = setup_git_repo(temp_dir.path());
    let backend = GitCheckpointBackend::discover(temp_dir.path()).unwrap();

    let leash_dir = temp_dir.path().join(".leash");
    fs::create_dir_all(&leash_dir).unwrap();
    let log_file = leash_dir.join("log.jsonl");
    fs::write(&log_file, "{\"audit\":\"vital_audit_trail_entry\"}\n").unwrap();

    let test_file = temp_dir.path().join("tracked_file.txt");
    fs::write(&test_file, "initial content\n").unwrap();
    let checkpoint = backend
        .create_checkpoint("s1", "initial checkpoint")
        .unwrap();

    fs::write(&test_file, "modified content to be overwritten\n").unwrap();

    let mut opts = fs::OpenOptions::new();
    opts.write(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        opts.share_mode(0);
    }
    let _locked_file = opts.open(&test_file).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&test_file).unwrap().permissions();
        perms.set_mode(0o400);
        let _ = fs::set_permissions(&test_file, perms);
        let mut dir_perms = fs::metadata(temp_dir.path()).unwrap().permissions();
        dir_perms.set_mode(0o555);
        let _ = fs::set_permissions(temp_dir.path(), dir_perms);
    }

    let restore_err = backend.restore_checkpoint(&checkpoint.id);
    assert!(
        restore_err.is_err(),
        "Checkout must fail due to locked file"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut dir_perms = fs::metadata(temp_dir.path()).unwrap().permissions();
        dir_perms.set_mode(0o755);
        let _ = fs::set_permissions(temp_dir.path(), dir_perms);
    }

    assert!(
        log_file.exists(),
        "Audit log must survive even when restore fails"
    );
    let content = fs::read_to_string(&log_file).unwrap();
    assert_eq!(content, "{\"audit\":\"vital_audit_trail_entry\"}\n");
}
