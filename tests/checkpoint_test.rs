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
fn test_checkpoint_lifecycle_and_rewind() {
    let temp = tempdir().unwrap();
    let repo = setup_git_repo(temp.path());
    let backend = GitCheckpointBackend::discover(temp.path()).expect("Failed to discover repo");

    let target_file = temp.path().join("code.rs");

    // 1. Write initial content and create checkpoint
    fs::write(&target_file, "fn main() { println!(\"version 1\"); }\n").unwrap();
    let cp1 = backend
        .create_checkpoint("sess-001", "checkpoint v1")
        .expect("Failed to create checkpoint 1");

    assert!(!cp1.id.is_empty());
    assert_eq!(cp1.session_id, "sess-001");
    assert_eq!(cp1.description, "checkpoint v1");

    // Verify checkpoint is listed
    let list1 = backend.list_checkpoints(None).unwrap();
    assert_eq!(list1.len(), 1);
    assert_eq!(list1[0].id, cp1.id);

    // 2. Modify file and add another file, then create checkpoint 2
    fs::write(&target_file, "fn main() { println!(\"version 2\"); }\n").unwrap();
    let extra_file = temp.path().join("extra.txt");
    fs::write(&extra_file, "extra content").unwrap();

    let cp2 = backend
        .create_checkpoint("sess-001", "checkpoint v2")
        .expect("Failed to create checkpoint 2");

    let list2 = backend.list_checkpoints(None).unwrap();
    assert_eq!(list2.len(), 2);
    assert_eq!(list2[0].id, cp2.id); // Most recent first

    // 3. Make unwanted destructive changes
    fs::write(&target_file, "fn corrupted() { panic!(); }\n").unwrap();
    let junk_file = temp.path().join("junk.tmp");
    fs::write(&junk_file, "garbage").unwrap();

    assert_eq!(
        fs::read_to_string(&target_file).unwrap(),
        "fn corrupted() { panic!(); }\n"
    );
    assert!(junk_file.exists());

    // 4. Rewind to checkpoint 2
    backend
        .restore_checkpoint(&cp2.id)
        .expect("Failed to restore checkpoint 2");

    // Verify state restored to cp2
    let content_cp2 = fs::read_to_string(&target_file)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(content_cp2, "fn main() { println!(\"version 2\"); }\n");
    assert!(extra_file.exists());
    assert!(
        !junk_file.exists(),
        "Untracked junk file should be removed on rewind"
    );

    // 5. Rewind back to checkpoint 1
    backend
        .restore_checkpoint(&cp1.id)
        .expect("Failed to restore checkpoint 1");

    // Verify state restored to cp1
    let content_cp1 = fs::read_to_string(&target_file)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(content_cp1, "fn main() { println!(\"version 1\"); }\n");
    assert!(
        !extra_file.exists(),
        "extra_file did not exist at cp1 and should be removed"
    );

    // 6. Verify user HEAD branch was never changed
    let head = repo.head().unwrap();
    assert_eq!(head.shorthand().unwrap(), "master");
    let head_commit = head.peel_to_commit().unwrap();
    assert_eq!(head_commit.summary().unwrap(), "Initial commit");
}

#[test]
fn test_list_checkpoints_session_filter() {
    let temp = tempdir().unwrap();
    let _repo = setup_git_repo(temp.path());
    let backend = GitCheckpointBackend::discover(temp.path()).unwrap();

    let file = temp.path().join("file.txt");
    fs::write(&file, "sess 1").unwrap();
    let cp1 = backend.create_checkpoint("session-a", "task a").unwrap();

    fs::write(&file, "sess 2").unwrap();
    let cp2 = backend.create_checkpoint("session-b", "task b").unwrap();

    let list_all = backend.list_checkpoints(None).unwrap();
    assert_eq!(list_all.len(), 2);

    let list_a = backend.list_checkpoints(Some("session-a")).unwrap();
    assert_eq!(list_a.len(), 1);
    assert_eq!(list_a[0].id, cp1.id);

    let list_b = backend.list_checkpoints(Some("session-b")).unwrap();
    assert_eq!(list_b.len(), 1);
    assert_eq!(list_b[0].id, cp2.id);

    let list_none = backend.list_checkpoints(Some("session-c")).unwrap();
    assert!(list_none.is_empty());
}

#[test]
fn test_checkpoint_in_unborn_repo() {
    let temp = tempdir().unwrap();
    let repo = Repository::init(temp.path()).expect("Failed to init git repo");
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    config.set_bool("core.autocrlf", false).unwrap();

    // Notice: NO commits made to repo yet! HEAD is unborn!
    let backend = GitCheckpointBackend::discover(temp.path()).unwrap();
    let file = temp.path().join("first_code.rs");
    fs::write(&file, "fn first() {}\n").unwrap();

    let cp = backend
        .create_checkpoint("session-unborn", "initial workdir")
        .expect("Failed to create checkpoint on unborn repo");
    assert!(!cp.id.is_empty());

    let list = backend.list_checkpoints(None).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, cp.id);

    // Modify file
    fs::write(&file, "fn corrupted() {}\n").unwrap();
    backend
        .restore_checkpoint(&cp.id)
        .expect("Failed to restore checkpoint on unborn repo");

    let restored = fs::read_to_string(&file).unwrap().replace("\r\n", "\n");
    assert_eq!(restored, "fn first() {}\n");
}

#[test]
fn test_checkpoint_excludes_leash_directory() {
    let temp = tempdir().unwrap();
    let repo = setup_git_repo(temp.path());
    let backend = GitCheckpointBackend::discover(temp.path()).unwrap();

    // Create .leash directory with policy and logs
    let leash_dir = temp.path().join(".leash");
    fs::create_dir_all(&leash_dir).unwrap();
    let policy_file = leash_dir.join("policy.yaml");
    let log_file = leash_dir.join("log.jsonl");
    fs::write(&policy_file, "version: 1\n").unwrap();
    fs::write(&log_file, "{\"event\":\"initial\"}\n").unwrap();

    let code_file = temp.path().join("main.rs");
    fs::write(&code_file, "fn main() { 1 }\n").unwrap();

    let cp = backend
        .create_checkpoint("sess-leash", "test exclude .leash")
        .unwrap();

    // 1. Verify that the checkpoint commit tree explicitly DOES NOT contain .leash
    let obj = repo.revparse_single(&cp.id).unwrap();
    let commit = obj.peel_to_commit().unwrap();
    let tree = commit.tree().unwrap();
    assert!(
        tree.get_name(".leash").is_none(),
        "Checkpoint git tree must NOT contain .leash directory"
    );

    // 2. Append to log and modify policy after checkpoint
    fs::write(
        &log_file,
        "{\"event\":\"initial\"}\n{\"event\":\"second\"}\n",
    )
    .unwrap();
    fs::write(&code_file, "fn main() { 2 }\n").unwrap();

    // 3. Restore checkpoint
    backend.restore_checkpoint(&cp.id).unwrap();

    // Verify code_file was restored
    let code = fs::read_to_string(&code_file)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(code, "fn main() { 1 }\n");

    // 4. Verify .leash files remain intact and logs were NOT reverted!
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
