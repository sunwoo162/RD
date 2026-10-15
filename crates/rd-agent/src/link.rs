//! Safe, read-only detection of existing local GitHub repositories.
//! Detection never fetches, pulls, resets, cleans, moves, or writes worktrees.
use crate::git;
use std::{fs, path::{Path, PathBuf}};

#[derive(Debug, Clone)]
pub struct ExistingRepo {
    pub path: PathBuf,
    /// Actual configured origin (not the URL typed by the user).
    /// The sync guard compares this value exactly at update time.
    pub url: String,
    pub branch: String,
}

pub fn canonical_path(path: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path).map_err(|e| format!("로컬 경로 확인 실패: {e}"))?;
    #[cfg(windows)]
    let canonical = git::git_compatible_path(canonical);
    Ok(canonical)
}

/// Validate that path is the actual root of a local Git worktree, not a parent
/// directory, nested subdirectory or an unrelated folder. No network involved.
pub fn inspect_existing(path: &Path) -> Result<ExistingRepo, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("폴더 확인 실패: {e}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("연결할 실제 프로젝트 폴더를 선택하세요 (바로가기 제외)".into());
    }
    let path = canonical_path(path)?;
    let git_marker = path.join(".git");
    let marker = fs::symlink_metadata(&git_marker).map_err(|_| "선택한 폴더는 Git 저장소가 아닙니다".to_string())?;
    if marker.file_type().is_symlink() || !(marker.is_dir() || marker.is_file()) {
        return Err("확인할 수 없는 Git 메타데이터입니다".into());
    }
    let top = git::git(&path, &["rev-parse", "--show-toplevel"])
        .map_err(|_| "Git 프로젝트의 루트를 확인하지 못했습니다".to_string())?;
    if canonical_path(Path::new(&top))? != path {
        return Err("프로젝트 루트 폴더를 선택하세요".into());
    }
    let url = git::git(&path, &["remote", "get-url", "origin"])
        .map_err(|_| "origin 원격 저장소가 설정되지 않았습니다".to_string())?;
    git::github_identity(&url)
        .map_err(|_| "GitHub의 올바른 origin URL을 가진 저장소만 연결할 수 있습니다".to_string())?;
    let branch = git::current_branch(&path)
        .map_err(|_| "현재 브랜치를 확인할 수 없습니다".to_string())?;
    git::validate_branch(&branch)?;
    Ok(ExistingRepo { path, url, branch })
}

/// Search only direct child folders of the dedicated RepoDock root.
/// Never walk Documents as a whole or recurse into a repository's contents.
/// Ambiguous matches must be selected explicitly; never guess between them.
pub fn find_workspace_match(root: &Path, requested_url: &str) -> Result<Option<ExistingRepo>, String> {
    let identity = git::github_identity(requested_url)?;
    let canonical_root = canonical_path(root)?;
    let mut matched: Option<ExistingRepo> = None;
    let mut inspected = 0usize;
    for entry in fs::read_dir(&canonical_root).map_err(|e| format!("기본 저장 폴더 확인 실패: {e}"))? {
        inspected += 1;
        if inspected > 10_000 {
            return Err("프로젝트 폴더가 너무 많아 자동 판별을 중단했습니다".into());
        }
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        if !ty.is_dir() || ty.is_symlink() { continue; }
        let path = entry.path();
        if path.file_name().is_some_and(|n| n.to_string_lossy().starts_with(".rd-clone-")) { continue; }
        if !path.join(".git").exists() { continue; }
        // Canonicalize to avoid traversing junctions or other reparse points
        // outside the chosen workspace, even when not marked as symlinks.
        let Ok(real) = canonical_path(&path) else { continue };
        if real.parent() != Some(canonical_root.as_path()) { continue; }
        let Ok(existing) = inspect_existing(&path) else { continue };
        if git::github_identity(&existing.url).ok().as_deref() != Some(identity.as_str()) { continue; }
        if matched.is_some() {
            return Err("같은 GitHub 저장소가 여러 폴더에 있습니다. '기존 프로젝트 연결'에서 직접 폴더를 선택하세요".into());
        }
        matched = Some(existing);
    }
    Ok(matched)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git_ok(path: &Path, args: &[&str]) {
        let result = Command::new("git").arg("-C").arg(path).args(args).output().unwrap();
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    }

    fn repo(path: &Path, url: &str) {
        git_ok(path.parent().unwrap(), &["init", "-b", "main", path.to_str().unwrap()]);
        git_ok(path, &["-c", "user.name=QA", "-c", "user.email=qa@example.com",
            "commit", "--allow-empty", "-m", "initial"]);
        git_ok(path, &["remote", "add", "origin", url]);
    }

    #[test]
    fn finds_existing_repo_from_origin_without_modifying_it() {
        let temp = tempfile::tempdir().unwrap();
        let p = temp.path().join("arbitrary-name");
        repo(&p, "git@github.com:sunwoo162/RD.git");
        let before = git::git(&p, &["rev-parse", "HEAD"]).unwrap();
        let got = find_workspace_match(temp.path(), "https://github.com/Sunwoo162/rd/").unwrap().unwrap();
        assert_eq!(got.path, canonical_path(&p).unwrap());
        assert_eq!(got.url, "git@github.com:sunwoo162/RD.git");
        assert_eq!(got.branch, "main");
        assert_eq!(before, git::git(&p, &["rev-parse", "HEAD"]).unwrap());
        assert_eq!(git::git(&p, &["status", "--porcelain"]).unwrap(), "");
    }

    #[test]
    fn unrelated_and_non_git_directories_do_not_match() {
        let temp = tempfile::tempdir().unwrap();
        repo(&temp.path().join("RD"), "https://github.com/other/RD.git");
        fs::create_dir(temp.path().join("plain-folder")).unwrap();
        assert!(find_workspace_match(temp.path(), "https://github.com/sunwoo162/RD").unwrap().is_none());
    }

    #[test]
    fn ambiguous_copies_fail_closed_instead_of_picking_first() {
        let temp = tempfile::tempdir().unwrap();
        repo(&temp.path().join("one"), "https://github.com/example/repo.git");
        repo(&temp.path().join("two"), "git@github.com:example/repo");
        let err = find_workspace_match(temp.path(), "https://github.com/example/repo").unwrap_err();
        assert!(err.contains("여러 폴더"));
    }

    #[test]
    fn direct_project_folder_must_have_matching_github_origin_and_branch() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("repo");
        fs::create_dir(&dir).unwrap();
        assert!(inspect_existing(&dir).is_err());
        repo(&temp.path().join("another"), "https://example.net/other/thing.git");
        assert!(inspect_existing(&temp.path().join("another")).is_err());
        repo(&temp.path().join("valid"), "https://github.com/example/valid.git");
        assert_eq!(inspect_existing(&temp.path().join("valid")).unwrap().branch, "main");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_repositories_outside_root_are_not_discovered() {
        use std::os::unix::fs::symlink;
        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let external = outside.path().join("external");
        repo(&external, "https://github.com/example/external.git");
        symlink(&external, workspace.path().join("external")).unwrap();
        assert!(find_workspace_match(workspace.path(), "https://github.com/example/external").unwrap().is_none());
    }
}
