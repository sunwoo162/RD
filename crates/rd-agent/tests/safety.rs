use rd_agent::{git, model::{Repository, SyncMode, SyncStatus}, sync::{self, SyncIntent}};
use std::{fs, path::{Path, PathBuf}, process::Command};
use tempfile::TempDir;

fn cmd(dir: Option<&Path>, args: &[&str]) -> String {
    let mut command = Command::new("git");
    if let Some(dir) = dir { command.arg("-C").arg(dir); }
    let output = command.args(args).output().expect("git must be installed");
    assert!(output.status.success(), "git {:?} failed: {}", args, String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

// Git checkouts use CRLF for text files on some Windows runners.
fn read_normalized(path: &Path) -> String {
    fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}

struct Fixture {
    tmp: TempDir,
    seed: PathBuf,
    work: PathBuf,
    repo: Repository,
}

impl Fixture {
    fn new(mode: SyncMode) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let seed = tmp.path().join("seed");
        let bare = tmp.path().join("remote.git");
        let work = tmp.path().join("work");
        let seed_str = seed.to_str().unwrap();
        let bare_str = bare.to_str().unwrap();
        let work_str = work.to_str().unwrap();
        cmd(None, &["init", "--bare", "--initial-branch=main", bare_str]);
        cmd(None, &["init", "--initial-branch=main", seed_str]);
        cmd(Some(&seed), &["config", "user.name", "RD Tests"]);
        cmd(Some(&seed), &["config", "user.email", "rd@example.invalid"]);
        fs::write(seed.join("README.md"), "start\n").unwrap();
        cmd(Some(&seed), &["add", "."]);
        cmd(Some(&seed), &["commit", "-m", "initial"]);
        cmd(Some(&seed), &["remote", "add", "origin", bare_str]);
        cmd(Some(&seed), &["push", "-u", "origin", "main"]);
        cmd(None, &["clone", "--branch", "main", bare_str, work_str]);
        cmd(Some(&work), &["config", "user.name", "RD Tests"]);
        cmd(Some(&work), &["config", "user.email", "rd@example.invalid"]);
        let repo = Repository {
            id: "fixture-01".into(),
            url: bare_str.into(),
            path: work.clone(),
            branch: "main".into(),
            mode,
            enabled: true,
        };
        Self { tmp, seed, work, repo }
    }

    fn push(&self, contents: &str) {
        fs::write(self.seed.join("README.md"), contents).unwrap();
        cmd(Some(&self.seed), &["add", "."]);
        cmd(Some(&self.seed), &["commit", "-m", "new remote revision"]);
        cmd(Some(&self.seed), &["push", "origin", "main"]);
    }

    fn inspect(&self, intent: SyncIntent) -> SyncStatus {
        sync::inspect(&self.repo, self.tmp.path(), intent).unwrap()
    }
}

#[test]
fn manual_does_not_modify_worktree_without_approval() {
    let f = Fixture::new(SyncMode::Manual);
    f.push("remote\n");
    assert_eq!(f.inspect(SyncIntent::Check), SyncStatus::UpdateAvailable { commits: 1 });
    assert_eq!(read_normalized(&f.work.join("README.md")), "start\n");
    assert_eq!(f.inspect(SyncIntent::Sync), SyncStatus::Updated { commits: 1 });
    assert_eq!(read_normalized(&f.work.join("README.md")), "remote\n");
}

#[test]
fn auto_updates_clean_behind_branch() {
    let f = Fixture::new(SyncMode::Auto);
    f.push("remote\n");
    assert_eq!(f.inspect(SyncIntent::Scheduled), SyncStatus::Updated { commits: 1 });
    assert_eq!(f.inspect(SyncIntent::Check), SyncStatus::Current);
}

#[test]
fn dirty_tracked_file_blocks_update_without_loss() {
    let f = Fixture::new(SyncMode::Auto);
    f.push("remote\n");
    fs::write(f.work.join("README.md"), "my unfinished changes\n").unwrap();
    assert_eq!(f.inspect(SyncIntent::Scheduled), SyncStatus::LocalChanges);
    assert_eq!(read_normalized(&f.work.join("README.md")), "my unfinished changes\n");
}

#[test]
fn untracked_file_blocks_update_without_loss() {
    let f = Fixture::new(SyncMode::Auto);
    f.push("remote\n");
    fs::write(f.work.join("scratch.txt"), "keep me").unwrap();
    assert_eq!(f.inspect(SyncIntent::Scheduled), SyncStatus::LocalChanges);
    assert_eq!(fs::read_to_string(f.work.join("scratch.txt")).unwrap(), "keep me");
}

#[test]
fn diverged_branch_never_auto_merges() {
    let f = Fixture::new(SyncMode::Auto);
    fs::write(f.work.join("mine.txt"), "local\n").unwrap();
    cmd(Some(&f.work), &["add", "."]);
    cmd(Some(&f.work), &["commit", "-m", "local"]);
    f.push("remote\n");
    assert_eq!(f.inspect(SyncIntent::Scheduled), SyncStatus::Diverged);
    assert!(f.work.join("mine.txt").exists());
}

#[test]
fn wrong_branch_never_switches_checkout() {
    let f = Fixture::new(SyncMode::Auto);
    cmd(Some(&f.work), &["switch", "-c", "feat/wip"]);
    f.push("remote\n");
    assert_eq!(f.inspect(SyncIntent::Scheduled), SyncStatus::WrongBranch { actual: "feat/wip".into() });
    assert_eq!(git::current_branch(&f.work).unwrap(), "feat/wip");
}

#[test]
fn paused_repository_has_no_effect() {
    let mut f = Fixture::new(SyncMode::Auto);
    f.repo.enabled = false;
    f.push("remote\n");
    assert_eq!(f.inspect(SyncIntent::Check), SyncStatus::Paused);
    assert_eq!(read_normalized(&f.work.join("README.md")), "start\n");
}

#[test]
fn changed_origin_is_not_trusted() {
    let f = Fixture::new(SyncMode::Auto);
    cmd(Some(&f.work), &["remote", "set-url", "origin", "https://github.com/another/repo"]);
    assert_eq!(f.inspect(SyncIntent::Check), SyncStatus::RemoteChanged);
}

#[test]
fn observe_mode_never_modifies_files_even_when_requested() {
    let f = Fixture::new(SyncMode::Observe);
    f.push("remote\n");
    assert_eq!(f.inspect(SyncIntent::Sync), SyncStatus::ObserveOnly);
    assert_eq!(read_normalized(&f.work.join("README.md")), "start\n");
}

#[test]
fn repo_lock_denies_simultaneous_update() {
    use fs2::FileExt;
    use std::fs::OpenOptions;
    let f = Fixture::new(SyncMode::Auto);
    let dir = f.tmp.path().join("locks");
    fs::create_dir_all(&dir).unwrap();
    let lock = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(dir.join("fixture-01.lock")).unwrap();
    lock.lock_exclusive().unwrap();
    assert_eq!(f.inspect(SyncIntent::Check), SyncStatus::Busy);
    FileExt::unlock(&lock).unwrap();
}
