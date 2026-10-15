use rd_agent::{config, model::{Repository, Settings, SyncMode}};
use std::process::Command;

fn fixture() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let id = "test-repo".to_string();
    let settings = Settings {
        version: 1,
        interval_minutes: 15,
        default_root: None,
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

#[test]
fn stored_workspace_and_interval_survive_reload() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("RepoDock");
    let agent = env!("CARGO_BIN_EXE_rd-agent");
    let first = Command::new(agent).env("RD_CONFIG_DIR", directory.path())
        .arg("root").arg(&root).output().unwrap();
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    let second = Command::new(agent).env("RD_CONFIG_DIR", directory.path())
        .args(["interval", "30"]).output().unwrap();
    assert!(second.status.success(), "{}", String::from_utf8_lossy(&second.stderr));
    let stored = config::load(directory.path()).unwrap();
    assert_eq!(stored.interval_minutes, 30);
    assert_eq!(stored.default_root, Some(root.canonicalize().unwrap().to_string_lossy()
        .strip_prefix(r"\\?\").map(std::path::PathBuf::from).unwrap_or_else(|| root.canonicalize().unwrap())));
    assert!(stored.repositories.is_empty());
}

#[test]
fn interval_rejects_invalid_values_without_changing_settings() {
    let temp = tempfile::tempdir().unwrap();
    for minutes in ["0", "1", "999", "invalid"] {
        let result = Command::new(env!("CARGO_BIN_EXE_rd-agent"))
            .env("RD_CONFIG_DIR", temp.path())
            .args(["interval", minutes]).output().unwrap();
        assert!(!result.status.success());
    }
    assert_eq!(config::load(temp.path()).unwrap().interval_minutes, 15);
}

#[test]
fn adding_without_workspace_does_not_start_clone() {
    let temp = tempfile::tempdir().unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_rd-agent"))
        .env("RD_CONFIG_DIR", temp.path())
        .args(["add-auto", "https://github.com/example/project.git"])
        .output().unwrap();
    assert!(!result.status.success());
    assert!(config::load(temp.path()).unwrap().repositories.is_empty());
}
