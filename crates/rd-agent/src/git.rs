use std::{ffi::OsStr, fs, path::{Path, PathBuf}, process::{Command, Output}, time::{SystemTime, UNIX_EPOCH}};

// Windows canonicalize() adds a \\?\ prefix. Git for Windows cannot reliably
// use these verbatim paths as clone destinations.
#[cfg(windows)]
fn git_compatible_path(path: PathBuf) -> PathBuf {
    let Some(text) = path.to_str() else { return path };
    let Some(rest) = text.strip_prefix(r"\\?\") else { return path };
    if let Some(unc) = rest.strip_prefix(r"UNC\") {
        PathBuf::from(format!(r"\\{}", unc))
    } else if rest.as_bytes().get(1) == Some(&b':') {
        PathBuf::from(rest)
    } else {
        path
    }
}

fn execute(cmd: &mut Command) -> Result<String, String> {
    let output: Output = cmd.env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .output()
        .map_err(|e| format!("Git 실행 실패(설치 확인 필요): {}", e))?;
    if !output.status.success() {
        let msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Git 작업 실패: {}", msg.chars().take(300).collect::<String>().trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo).args(args);
    execute(&mut cmd)
}

pub fn validate_github_url(url: &str) -> Result<String, String> {
    let suffix = url.strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))
        .ok_or("GitHub HTTPS 또는 SSH URL만 지원합니다")?;
    let suffix = suffix.strip_suffix(".git").unwrap_or(suffix);
    let parts: Vec<&str> = suffix.split('/').collect();
    if parts.len() != 2 || parts.iter().any(|p| p.is_empty() || *p == "." || *p == ".." || !p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')) {
        return Err("잘못된 GitHub 저장소 URL입니다".into());
    }
    Ok(parts[1].to_string())
}

pub fn clone_repository(url: &str, target: &Path, branch: Option<&str>) -> Result<(PathBuf, String), String> {
    validate_github_url(url)?;
    if target.exists() { return Err("대상 폴더가 이미 있습니다. 기존 파일을 덮어쓰지 않습니다".into()); }
    let parent = target.parent().ok_or("대상 폴더의 상위 경로가 없습니다")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let name = target.file_name().ok_or("대상 폴더 이름이 없습니다")?;
    let parent = fs::canonicalize(parent).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    let parent = git_compatible_path(parent);
    let target = parent.join(name);
    if target.exists() { return Err("대상 폴더가 이미 있습니다".into()); }
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let stage = parent.join(format!(".rd-clone-{}-{}", std::process::id(), nonce));
    let mut cmd = Command::new("git");
    let hooks_dir = crate::config::data_dir()?.join("empty-hooks");
    fs::create_dir_all(&hooks_dir).map_err(|e| e.to_string())?;
    cmd.arg("-c").arg(format!("core.hooksPath={}", hooks_dir.display()));
    cmd.arg("clone").arg("--no-tags");
    if let Some(b) = branch {
        validate_branch(b)?;
        cmd.arg("--branch").arg(b);
    }
    cmd.arg("--").arg(OsStr::new(url)).arg(&stage);
    if let Err(e) = execute(&mut cmd) {
        if stage.exists() { let _ = fs::remove_dir_all(&stage); }
        return Err(e);
    }
    let actual_branch = git(&stage, &["branch", "--show-current"])?;
    if actual_branch.is_empty() {
        let _ = fs::remove_dir_all(&stage);
        return Err("체크아웃된 브랜치가 없어 등록을 중단했습니다".into());
    }
    if let Err(e) = fs::rename(&stage, &target) {
        return Err(format!("폴더 이동 실패: {} (임시 경로: {})", e, stage.display()));
    }
    Ok((target, actual_branch))
}

pub fn validate_branch(branch: &str) -> Result<(), String> {
    if branch.is_empty() || branch.starts_with('-') || branch.ends_with('/') || branch.contains("..") || branch.contains("@{") || branch.contains(' ') || branch.contains('\\') || branch.contains(':') || branch.contains('?') || branch.contains('*') || branch.contains('[') || branch.contains('~') || branch.contains('^') || branch.chars().any(|c| c.is_control()) || branch.split('/').any(|s| s.is_empty() || s.starts_with('.') || s.ends_with(".lock")) {
        return Err("잘못된 Git 브랜치 이름입니다".into());
    }
    Ok(())
}

pub fn current_branch(path: &Path) -> Result<String, String> {
    git(path, &["branch", "--show-current"])
}

pub fn dirty(path: &Path) -> Result<bool, String> {
    Ok(!git(path, &["status", "--porcelain=v1", "--untracked-files=all"])?.is_empty())
}

pub fn operation_in_progress(path: &Path) -> Result<bool, String> {
    let names = ["MERGE_HEAD", "REBASE_HEAD", "rebase-apply", "rebase-merge", "CHERRY_PICK_HEAD", "REVERT_HEAD", "BISECT_LOG", "sequencer"];
    for name in names {
        let location = git(path, &["rev-parse", "--git-path", name])?;
        let location = PathBuf::from(location);
        let actual = if location.is_absolute() { location } else { path.join(location) };
        if actual.exists() { return Ok(true); }
    }
    Ok(false)
}

pub fn origin_sha(path: &Path, branch: &str) -> Result<Option<String>, String> {
    let output = Command::new("git").arg("-C").arg(path)
        .args(["rev-parse", "--verify", &format!("refs/remotes/origin/{}", branch)])
        .output().map_err(|e| e.to_string())?;
    if !output.status.success() { return Ok(None); }
    Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string()))
}

pub fn remote_sha(path: &Path, branch: &str) -> Result<Option<String>, String> {
    validate_branch(branch)?;
    let remote_ref = format!("refs/heads/{}", branch);
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(path).args(["ls-remote", "--heads", "origin", &remote_ref]);
    let output = execute(&mut cmd)?;
    if output.is_empty() { return Ok(None); }
    let sha = output.split_whitespace().next().ok_or("원격 응답이 올바르지 않습니다")?;
    if sha.len() != 40 && sha.len() != 64 { return Err("원격 커밋 ID가 올바르지 않습니다".into()); }
    Ok(Some(sha.to_string()))
}

pub fn fetch_branch(path: &Path, branch: &str, hooks_dir: &Path) -> Result<(), String> {
    validate_branch(branch)?;
    let refspec = format!("+refs/heads/{}:refs/remotes/origin/{}", branch, branch);
    let hooks_override = format!("core.hooksPath={}", hooks_dir.display());
    git(path, &["-c", &hooks_override, "fetch", "--no-tags", "origin", &refspec])?;
    Ok(())
}

pub fn ahead_behind(path: &Path, branch: &str) -> Result<(u64, u64), String> {
    let diff = format!("HEAD...refs/remotes/origin/{}", branch);
    let output = git(path, &["rev-list", "--left-right", "--count", &diff])?;
    let mut sides = output.split_whitespace();
    let ahead: u64 = sides.next().ok_or("커밋 비교 오류")?.parse().map_err(|_| "커밋 비교 오류")?;
    let behind: u64 = sides.next().ok_or("커밋 비교 오류")?.parse().map_err(|_| "커밋 비교 오류")?;
    Ok((ahead, behind))
}

pub fn fast_forward(path: &Path, branch: &str, hooks_dir: &Path) -> Result<(), String> {
    let remote_ref = format!("refs/remotes/origin/{}", branch);
    let hooks_override = format!("core.hooksPath={}", hooks_dir.display());
    git(path, &["-c", &hooks_override, "merge", "--ff-only", "--no-overwrite-ignore", "--no-edit", &remote_ref])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn windows_verbatim_paths_are_git_compatible() {
        assert_eq!(git_compatible_path(PathBuf::from(r"\\?\C:\work\repo")),
                   PathBuf::from(r"C:\work\repo"));
        assert_eq!(git_compatible_path(PathBuf::from(r"\\?\UNC\server\share\repo")),
                   PathBuf::from(r"\\server\share\repo"));
    }
    #[cfg(windows)]
    #[test]
    fn canonicalized_windows_parent_supports_git_clone() {
        let temp = tempfile::tempdir().unwrap();
        let bare = temp.path().join("remote.git");
        let init = Command::new("git").args(["init", "--bare"]).arg(&bare)
            .output().expect("Git must be installed");
        assert!(init.status.success(), "{}", String::from_utf8_lossy(&init.stderr));
        let parent = git_compatible_path(fs::canonicalize(temp.path()).unwrap());
        let work = parent.join("work");
        let clone = Command::new("git").arg("clone").arg(&bare).arg(&work)
            .output().expect("Git must be installed");
        assert!(clone.status.success(), "{}", String::from_utf8_lossy(&clone.stderr));
        assert!(work.join(".git").is_dir());
    }
    #[test]
    fn only_explicit_github_urls() {
        assert_eq!(validate_github_url("https://github.com/sunwoo162/ain.git").unwrap(), "ain");
        assert_eq!(validate_github_url("git@github.com:sunwoo162/RD").unwrap(), "RD");
        for bad in ["https://evil.com/a/b", "https://github.com/a/b/c", "https://github.com/a/../b", "https://name:secret@github.com/a/b", "file:///tmp/a"] {
            assert!(validate_github_url(bad).is_err(), "{} should be rejected", bad);
        }
    }
    #[test]
    fn branch_names_are_restricted() {
        assert!(validate_branch("feat/ui").is_ok());
        assert!(validate_branch("main").is_ok());
        for bad in ["--upload-pack=oops", "a..b", "a b", "refs/.hidden", "a.lock", "a//b", "a@{x"] {
            assert!(validate_branch(bad).is_err(), "{} should be rejected", bad);
        }
    }
}
