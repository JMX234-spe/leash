use leash::checkpoint::GitCheckpointBackend;

#[test]
fn test_checkpoint_backend_stub() {
    let backend = GitCheckpointBackend::new();
    let result = backend.list_checkpoints(None);
    assert!(result.is_err());
}
