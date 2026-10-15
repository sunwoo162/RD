use rd_agent::{config, model::{Repository, Settings, SyncMode}};
use std::process::Command;

fn fixture() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let id = "test-repo".to_string();
    let settings = Settings {
        version: 1,
        interval_minutes: 15,
        repositories: vec![Repository {
            id: id.clone(),
            url: "https://github.com/example/not-needed-for-test".into(),
            path: dir.path().join("missing-working-tree"),
            branch: "main".into(),
            mode: SyncMode::Manual,
            enabled: true,
        }],
    };
    config::save(dir.path(), &settings).unwrap();
    (dir, id)
}

#[test]
fn failed_check_returns_nonzero_exit_code() {
    let (dir, id) = fixture();
    let output = Command::new(env!("CARGO_BIN_EXE_rd-agent"))
        .env("RD_CONFIG_DIR", dir.path())
        .args(["check", &id])
        .output().unwrap();
    assert!(!output.status.success(), "a failed check must not report success");
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("1개 저장소"), "{error}");
}

#[test]
fn concurrent_settings_command_is_rejected_without_losing_updates() {
    let (dir, id) = fixture();
    let lock = config::ConfigLock::acquire(dir.path()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rd-agent"))
        .env("RD_CONFIG_DIR", dir.path())
        .args(["mode", &id, "auto"])
        .output().unwrap();
    assert!(!output.status.success(), "config updates must respect the lock");
    assert_eq!(config::load(dir.path()).unwrap().repositories[0].mode, SyncMode::Manual);

    drop(lock);
    let output = Command::new(env!("CARGO_BIN_EXE_rd-agent"))
        .env("RD_CONFIG_DIR", dir.path())
        .args(["mode", &id, "auto"])
        .output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(config::load(dir.path()).unwrap().repositories[0].mode, SyncMode::Auto);
}
