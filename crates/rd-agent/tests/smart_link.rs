//! Full CLI integration tests use local-only Git worktrees with a GitHub
//! origin URL. None of these tests access GitHub or modify a real user folder.
use rd_agent::{config, model::SyncMode};
use std::{fs, path::Path, process::{Command, Output}};
use tempfile::TempDir;

fn run_git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().unwrap();
    assert!(out.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn local_repo(path: &Path, url: &str) {
    fs::create_dir_all(path).unwrap();
    run_git(path, &["init", "-b", "main"]);
    run_git(path, &["-c", "user.name=RD QA", "-c", "user.email=qa@example.com",
        "commit", "--allow-empty", "-m", "local test repository"]);
    run_git(path, &["remote", "add", "origin", url]);
}

fn agent(temp: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rd-agent"))
        .env("RD_CONFIG_DIR", temp.path().join("config"))
        .args(args)
        .output().unwrap()
}

fn success(out: Output) -> String {
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn config_of(t: &TempDir) -> rd_agent::model::Settings {
    config::load(&t.path().join("config")).unwrap()
}

#[test]
fn autodetect_existing_repo_by_actual_origin_and_keep_dirty_files() {
    let t = tempfile::tempdir().unwrap();
    let work = t.path().join("RepoDock");
    fs::create_dir(&work).unwrap();
    let repo = work.join("not-the-github-repo-name");
    local_repo(&repo, "git@github.com:example/actual-name.git");
    fs::write(repo.join("in-progress.txt"), "user work must stay").unwrap();
    let sha = run_git(&repo, &["rev-parse", "HEAD"]);
    success(agent(&t, &["root", work.to_str().unwrap()]));
    let output = success(agent(&t, &["add-auto", "https://github.com/example/actual-name/"]));
    assert!(output.contains("기존 GitHub 프로젝트 연결 완료"), "{output}");
    let settings = config_of(&t);
    assert_eq!(settings.repositories.len(), 1);
    assert_eq!(settings.repositories[0].path, repo.canonicalize().unwrap());
    assert_eq!(settings.repositories[0].url, "git@github.com:example/actual-name.git");
    assert_eq!(settings.repositories[0].mode, SyncMode::Auto);
    assert_eq!(fs::read_to_string(repo.join("in-progress.txt")).unwrap(), "user work must stay");
    assert_eq!(run_git(&repo, &["rev-parse", "HEAD"]), sha);
    assert_eq!(fs::read_dir(&work).unwrap().count(), 1, "must not clone a second folder");
}

#[test]
fn repeated_registration_never_duplicates_even_with_other_url_form() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("RepoDock");
    fs::create_dir(&root).unwrap();
    local_repo(&root.join("existing"), "https://github.com/Example/Repo.git");
    success(agent(&t, &["root", root.to_str().unwrap()]));
    success(agent(&t, &["add-auto", "https://github.com/example/repo"]));
    let outcome = success(agent(&t, &["add-auto", "git@github.com:EXAMPLE/REPO.git"]));
    assert!(outcome.contains("이미 RD에 연결된 프로젝트"));
    assert_eq!(config_of(&t).repositories.len(), 1);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
}

#[test]
fn ambiguous_matches_do_not_clone_or_register_anything() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("RepoDock");
    fs::create_dir(&root).unwrap();
    local_repo(&root.join("one"), "https://github.com/example/repo.git");
    local_repo(&root.join("two"), "git@github.com:example/repo");
    success(agent(&t, &["root", root.to_str().unwrap()]));
    let out = agent(&t, &["add-auto", "https://github.com/example/repo"]);
    assert!(!out.status.success());
    assert_eq!(config_of(&t).repositories.len(), 0);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
}

#[test]
fn explicit_attach_outside_workspace_never_moves_or_modifies_files() {
    let t = tempfile::tempdir().unwrap();
    let outside = t.path().join("my-original-project");
    local_repo(&outside, "https://github.com/example/my-original-project.git");
    fs::write(outside.join("UNCOMMITTED.md"), "keep me").unwrap();
    let before = run_git(&outside, &["rev-parse", "HEAD"]);
    let result = success(agent(&t, &["attach", outside.to_str().unwrap()]));
    assert!(result.contains("기존 GitHub 프로젝트 연결 완료"));
    let settings = config_of(&t);
    assert_eq!(settings.repositories.len(), 1);
    assert_eq!(settings.repositories[0].mode, SyncMode::Auto);
    assert_eq!(settings.repositories[0].path, outside.canonicalize().unwrap());
    assert_eq!(run_git(&outside, &["rev-parse", "HEAD"]), before);
    assert_eq!(fs::read_to_string(outside.join("UNCOMMITTED.md")).unwrap(), "keep me");
    assert!(settings.default_root.is_none(), "explicit attach must not change workspace preferences");
}

#[test]
fn explicit_attach_rejects_non_git_folder_without_persisting_it() {
    let t = tempfile::tempdir().unwrap();
    let folder = t.path().join("not-a-repo");
    fs::create_dir(&folder).unwrap();
    let out = agent(&t, &["attach", folder.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(config_of(&t).repositories.is_empty());
}
