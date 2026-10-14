use crate::{git, model::{Repository, SyncMode, SyncStatus}};
use fs2::FileExt;
use std::{fs::{self, File, OpenOptions}, path::Path};

struct RepoLock(File);
impl RepoLock {
    fn acquire(dir: &Path, id: &str) -> Result<Option<Self>, String> {
        fs::create_dir_all(dir.join("locks")).map_err(|e| e.to_string())?;
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err("잘못된 저장소 ID입니다".into());
        }
        let file = OpenOptions::new().create(true).read(true).write(true)
            .open(dir.join("locks").join(format!("{}.lock", id))).map_err(|e| e.to_string())?;
        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self(file))),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}
impl Drop for RepoLock {
    fn drop(&mut self) { let _ = FileExt::unlock(&self.0); }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncIntent {
    Check,
    Sync,
    Scheduled,
}

pub fn inspect(repo: &Repository, config_dir: &Path, intent: SyncIntent) -> Result<SyncStatus, String> {
    if !repo.enabled { return Ok(SyncStatus::Paused); }
    git::validate_branch(&repo.branch)?;
    if !repo.path.is_dir() { return Err(format!("로컬 폴더 없음: {}", repo.path.display())); }
    let _lock = match RepoLock::acquire(config_dir, &repo.id)? {
        Some(lock) => lock,
        None => return Ok(SyncStatus::Busy),
    };

    let actual = git::current_branch(&repo.path)?;
    if actual != repo.branch { return Ok(SyncStatus::WrongBranch { actual }); }
    if git::operation_in_progress(&repo.path)? { return Ok(SyncStatus::GitOperationInProgress); }

    let origin = git::git(&repo.path, &["remote", "get-url", "origin"])?;
    if origin != repo.url { return Ok(SyncStatus::RemoteChanged); }

    let sha = match git::remote_sha(&repo.path, &repo.branch)? {
        Some(sha) => sha,
        None => return Ok(SyncStatus::RemoteMissing),
    };
    if git::origin_sha(&repo.path, &repo.branch)?.as_deref() != Some(sha.as_str()) {
        let hooks_dir = config_dir.join("empty-hooks");
        fs::create_dir_all(&hooks_dir).map_err(|e| e.to_string())?;
        git::fetch_branch(&repo.path, &repo.branch, &hooks_dir)?;
    }

    let (ahead, behind) = git::ahead_behind(&repo.path, &repo.branch)?;
    if ahead > 0 && behind > 0 { return Ok(SyncStatus::Diverged); }
    if ahead > 0 { return Ok(SyncStatus::LocalAhead); }
    if git::dirty(&repo.path)? { return Ok(SyncStatus::LocalChanges); }
    if behind == 0 { return Ok(SyncStatus::Current); }

    let apply = match intent {
        SyncIntent::Check => false,
        SyncIntent::Sync => true,
        SyncIntent::Scheduled => repo.mode == SyncMode::Auto,
    };
    if !apply { return Ok(SyncStatus::UpdateAvailable { commits: behind }); }
    if repo.mode == SyncMode::Observe { return Ok(SyncStatus::ObserveOnly); }

    if git::current_branch(&repo.path)? != repo.branch {
        return Ok(SyncStatus::WrongBranch { actual: git::current_branch(&repo.path)? });
    }
    if git::operation_in_progress(&repo.path)? { return Ok(SyncStatus::GitOperationInProgress); }
    if git::dirty(&repo.path)? { return Ok(SyncStatus::LocalChanges); }
    let (ahead_now, behind_now) = git::ahead_behind(&repo.path, &repo.branch)?;
    if ahead_now > 0 { return Ok(SyncStatus::Diverged); }
    if behind_now == 0 { return Ok(SyncStatus::Current); }

    let hooks_dir = config_dir.join("empty-hooks");
    fs::create_dir_all(&hooks_dir).map_err(|e| e.to_string())?;
    git::fast_forward(&repo.path, &repo.branch, &hooks_dir)?;
    Ok(SyncStatus::Updated { commits: behind_now })
}
