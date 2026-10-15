use rd_agent::{config, git, link::{self, ExistingRepo}, model::{Repository, Settings, SyncMode}, sync::{self, SyncIntent}};
use std::{collections::HashMap, env, path::{Path, PathBuf}, process, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

fn usage() {
    println!("RD Agent v{}\n\
  rd-agent add <github-url> <folder> [--branch <name>] [--auto]\n\
  rd-agent add-auto <github-url>   # 기존 GitHub 저장소 발견 시 연결, 없으면 Clone\n\
  rd-agent attach <local-folder>   # 로컬 GitHub 프로젝트 연결(이동/복사 없음)\n\
  rd-agent root <folder> | interval <5|15|30|60>\n\
  rd-agent list\n\
  rd-agent check [id]\n\
  rd-agent sync [id]       # 직접 승인하는 1회 업데이트\n\
  rd-agent mode <id> <observe|manual|auto>\n\
  rd-agent pause <id> | resume <id> | remove <id>\n\
  rd-agent watch           # 콘솔 동기화 엔진\n  rd-agent autostart <enable|disable|status>  # Windows 로그인 자동 실행\n\n\
기본값: 15분 주기, Manual 모드. 자동 Pull은 --auto나 mode auto로 명시한 저장소에만 적용됩니다.\n\
등록 해제는 GitHub 저장소와 로컬 파일을 삭제하지 않습니다.", env!("CARGO_PKG_VERSION"));
}

#[cfg(windows)]
fn autostart(action: &str) -> Result<(), String> {
    let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let value = "RepoDock";
    let exe = env::current_exe().map_err(|e| e.to_string())?
        .with_file_name("rd-tray.exe");
    let quoted_path = format!("\"{}\"", exe.display());
    let invoke = |args: &[&str]| -> Result<bool, String> {
        let output = process::Command::new("reg.exe").args(args)
            .output().map_err(|e| format!("Windows 레지스트리 명령 실패: {}", e))?;
        Ok(output.status.success())
    };
    match action {
        "enable" => {
            if !exe.is_file() { return Err("rd-agent.exe와 같은 폴더에 rd-tray.exe가 필요합니다".into()); }
            // Run key command line has a documented 260-character limit.
            if quoted_path.len() > 260 { return Err("실행 파일 경로가 너무 깁니다 (260자 초과)".into()); }
            if !invoke(&["add", key, "/v", value, "/t", "REG_SZ", "/d", &quoted_path, "/f"])? {
                return Err("Windows 시작 프로그램 등록에 실패했습니다".into());
            }
            println!("로그인 자동 실행 활성화: {}", quoted_path);
        }
        "disable" => {
            let exists = invoke(&["query", key, "/v", value])?;
            if exists && !invoke(&["delete", key, "/v", value, "/f"])? {
                return Err("Windows 시작 프로그램 해제에 실패했습니다".into());
            }
            println!("로그인 자동 실행 비활성화");
        }
        "status" => {
            let exists = invoke(&["query", key, "/v", value])?;
            println!("로그인 자동 실행: {}", if exists { "등록됨" } else { "등록 안 됨" });
        }
        _ => return Err("사용법: autostart <enable|disable|status>".into()),
    }
    Ok(())
}

#[cfg(not(windows))]
fn autostart(_action: &str) -> Result<(), String> {
    Err("Windows 자동 시작만 지원합니다".into())
}

fn find_repo<'a>(config: &'a mut Settings, id: &str) -> Result<&'a mut Repository, String> {
    config.repositories.iter_mut().find(|repo| repo.id == id).ok_or(format!("등록되지 않은 ID: {}", id))
}

fn generate_id(settings: &Settings) -> Result<String, String> {
    let tick = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    let id = format!("{}-{}", process::id(), tick);
    if settings.repositories.iter().any(|r| r.id == id) { return Err("ID 생성 충돌".into()); }
    Ok(id)
}

// Only register checked Git roots; never change or move the worktree at attachment time.
fn register_existing(config_dir: &Path, config: &mut Settings, existing: ExistingRepo) -> Result<(), String> {
    let identity = git::github_identity(&existing.url)?;
    if let Some(registered) = config.repositories.iter().find(|r|
        git::github_identity(&r.url).ok().as_deref() == Some(identity.as_str())
    ) {
        println!("이미 RD에 연결된 프로젝트: {}", registered.path.display());
        return Ok(());
    }
    if config.repositories.iter().any(|r|
        link::canonical_path(&r.path).ok().as_deref() == Some(existing.path.as_path())
    ) {
        return Err("이미 다른 저장소로 관리 중인 로컬 폴더입니다".into());
    }
    let repo = Repository {
        id: generate_id(config)?,
        url: existing.url,
        path: existing.path,
        branch: existing.branch,
        mode: SyncMode::Auto,
        enabled: true,
    };
    config.repositories.push(repo);
    config::save(config_dir, config)?;
    println!("기존 GitHub 프로젝트 연결 완료 · 파일 이동/수정 없음 · 안전한 자동 업데이트 활성화");
    Ok(())
}

fn command(args: &[String], config_dir: &Path, config: &mut Settings) -> Result<(), String> {
    match args.first().map(String::as_str).unwrap_or("help") {
        "root" => {
            let path = PathBuf::from(args.get(1).ok_or("기본 저장 폴더를 선택하세요")?);
            if args.len() != 2 || !path.is_absolute() { return Err("절대 경로의 저장 폴더가 필요합니다".into()); }
            std::fs::create_dir_all(&path).map_err(|e| format!("폴더 생성 실패: {e}"))?;
            let canonical = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
            #[cfg(windows)]
            let canonical = git::git_compatible_path(canonical);
            config.default_root = Some(canonical);
            config::save(config_dir, config)?;
            println!("기본 저장 폴더 변경 완료 (기존 프로젝트 파일은 이동하지 않음)");
        }
        "interval" => {
            if args.len() != 2 { return Err("업데이트 주기는 5, 15, 30, 60분만 지원합니다".into()); }
            let minutes: u64 = args[1].parse().map_err(|_| "잘못된 업데이트 주기")?;
            if ![5, 15, 30, 60].contains(&minutes) { return Err("업데이트 주기는 5, 15, 30, 60분만 지원합니다".into()); }
            config.interval_minutes = minutes;
            config::save(config_dir, config)?;
            println!("자동 업데이트 확인 주기: {minutes}분");
        }
        "add-auto" => {
            if args.len() != 2 { return Err("사용법: add-auto <github-url>".into()); }
            let url = git::normalized_github_url(&args[1])?;
            let repo_name = git::validate_github_url(&url)?;
            let identity = git::github_identity(&url)?;
            if let Some(repo) = config.repositories.iter().find(|r|
                git::github_identity(&r.url).ok().as_deref() == Some(identity.as_str())
            ) {
                println!("이미 RD에 연결된 프로젝트: {}", repo.path.display());
                return Ok(());
            }
            let root = config.default_root.as_ref().ok_or("먼저 기본 저장 폴더를 설정하세요")?;
            // Search only inside the configured RepoDock folder, never all Documents.
            if let Some(existing) = link::find_workspace_match(root, &url)? {
                return register_existing(config_dir, config, existing);
            }
            let owner = url.strip_prefix("https://github.com/")
                .or_else(|| url.strip_prefix("git@github.com:"))
                .ok_or("GitHub URL 형식 오류")?
                .split('/').next().ok_or("소유자 이름 확인 실패")?;
            let mut target = root.join(&repo_name);
            if target.exists() {
                target = root.join(format!("{repo_name}-{owner}"));
            }
            for number in 2..=1000 {
                if !target.exists() { break; }
                target = root.join(format!("{repo_name}-{owner}-{number}"));
            }
            if target.exists() { return Err("사용 가능한 저장 폴더 이름이 없습니다".into()); }
            let (path, branch) = git::clone_repository(&url, &target, None)?;
            let repo = Repository {
                id: generate_id(config)?, url: url.clone(), path, branch, mode: SyncMode::Auto, enabled: true,
            };
            config.repositories.push(repo);
            config::save(config_dir, config)?;
            println!("프로젝트 연결 완료 · 안전한 자동 업데이트 활성화");
        }
        "attach" => {
            if args.len() != 2 { return Err("사용법: attach <local-folder>".into()); }
            let path = PathBuf::from(&args[1]);
            if !path.is_absolute() { return Err("기존 프로젝트 폴더의 절대 경로를 선택하세요".into()); }
            let existing = link::inspect_existing(&path)?;
            return register_existing(config_dir, config, existing);
        }
        "add" => {
            if args.len() < 3 { return Err("사용법: add <github-url> <folder> [--branch <name>] [--auto]".into()); }
            let url = &args[1];
            git::validate_github_url(url)?;
            let mut branch: Option<&str> = None;
            let mut mode = SyncMode::Manual;
            let mut index = 3;
            while index < args.len() {
                match args[index].as_str() {
                    "--branch" => {
                        branch = Some(args.get(index + 1).ok_or("--branch 값이 없습니다")?);
                        git::validate_branch(branch.unwrap())?;
                        index += 2;
                    }
                    "--auto" => { mode = SyncMode::Auto; index += 1; }
                    flag => return Err(format!("알 수 없는 옵션: {}", flag)),
                }
            }
            let target = PathBuf::from(&args[2]);
            if config.repositories.iter().any(|r| r.path == target) {
                return Err("이미 관리 중인 로컬 경로입니다".into());
            }
            let (path, branch) = git::clone_repository(url, &target, branch)?;
            if config.repositories.iter().any(|r| r.path == path) {
                return Err("이미 관리 중인 로컬 경로입니다".into());
            }
            let repo = Repository {
                id: generate_id(config)?, url: url.clone(), path, branch, mode, enabled: true,
            };
            println!("등록 완료: {} ({} / {:?})", repo.path.display(), repo.branch, repo.mode);
            println!("ID: {}", repo.id);
            config.repositories.push(repo);
            config::save(config_dir, config)?;
        }
        "list" => {
            if config.repositories.is_empty() { println!("등록된 저장소가 없습니다."); }
            for r in &config.repositories {
                println!("{} | {} | {} | {} | {:?} | {}", r.id, r.url, r.path.display(), r.branch, r.mode, if r.enabled { "on" } else { "paused" });
            }
        }
        "check" | "sync" => {
            let selected = args.get(1).map(String::as_str);
            let repos: Vec<_> = config.repositories.iter().filter(|r| selected.is_none() || selected == Some(r.id.as_str())).collect();
            if repos.is_empty() { return Err("해당 저장소가 없거나 등록된 저장소가 없습니다".into()); }
            let mut failures = 0;
            for repo in repos {
                let intent = if args[0] == "sync" { SyncIntent::Sync } else { SyncIntent::Check };
                match sync::inspect(repo, config_dir, intent) {
                    Ok(status) => println!("[{}] {}", repo.path.display(), status),
                    Err(error) => {
                        failures += 1;
                        eprintln!("[{}] 오류: {}", repo.path.display(), error);
                    }
                }
            }
            if failures > 0 {
                return Err(format!("{}개 저장소의 Git 작업이 실패했습니다", failures));
            }
        }
        "mode" => {
            if args.len() != 3 { return Err("사용법: mode <id> <observe|manual|auto>".into()); }
            let mode = SyncMode::parse(&args[2]).ok_or("모드는 observe / manual / auto만 허용됩니다")?;
            let repo = find_repo(config, &args[1])?;
            repo.mode = mode;
            config::save(config_dir, config)?;
            println!("동기화 모드 변경: {:?}", mode);
        }
        "pause" | "resume" => {
            let id = args.get(1).ok_or("저장소 ID가 필요합니다")?;
            let repo = find_repo(config, id)?;
            let enabled = args[0] == "resume";
            repo.enabled = enabled;
            config::save(config_dir, config)?;
            println!("{}", if enabled { "동기화 재개" } else { "동기화 중지" });
        }
        "remove" => {
            let id = args.get(1).ok_or("저장소 ID가 필요합니다")?;
            let old_len = config.repositories.len();
            config.repositories.retain(|repo| repo.id != *id);
            if config.repositories.len() == old_len { return Err("저장소 ID를 찾지 못했습니다".into()); }
            config::save(config_dir, config)?;
            println!("RD 관리 목록에서만 해제했습니다. 로컬 폴더는 그대로 유지됩니다.");
        }
        "watch" => return watch(config_dir),
        "autostart" => return autostart(args.get(1).map(String::as_str).unwrap_or("")),
        "help" | "--help" | "-h" => usage(),
        other => return Err(format!("알 수 없는 명령: {}", other)),
    }
    Ok(())
}

fn watch(config_dir: &Path) -> Result<(), String> {
    use fs2::FileExt;
    use std::fs::{self, OpenOptions};
    fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let file = OpenOptions::new().create(true).truncate(false).read(true).write(true)
        .open(config_dir.join("watch.lock")).map_err(|e| e.to_string())?;
    file.try_lock_exclusive().map_err(|_| "이미 RD Agent가 실행 중입니다")?;
    println!("RD Agent 감시 시작 (Ctrl+C로 종료)");
    let mut due: HashMap<String, Instant> = HashMap::new();
    loop {
        let config = config::load(config_dir)?;
        let interval = Duration::from_secs(config.interval_minutes.max(1) * 60);
        let now = Instant::now();
        for (index, repo) in config.repositories.iter().enumerate() {
            if !repo.enabled { continue; }
            let jitter = Duration::from_secs((index as u64 * 7) % interval.as_secs());
            let deadline = due.entry(repo.id.clone()).or_insert(now + jitter);
            if now >= *deadline {
                match sync::inspect(repo, config_dir, SyncIntent::Scheduled) {
                    Ok(status) => println!("[{}] {}", repo.path.display(), status),
                    Err(error) => eprintln!("[{}] 오류: {}", repo.path.display(), error),
                }
                *deadline = Instant::now() + interval;
            }
        }
        due.retain(|id, _| config.repositories.iter().any(|r| &r.id == id));
        thread::sleep(Duration::from_secs(10));
    }
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let result = config::data_dir().and_then(|dir| {
        // Acquire before loading, so simultaneous managers cannot overwrite
        // each other's changes with a stale copy of config.json.
        let mutating = matches!(args.first().map(String::as_str),
            Some("add" | "add-auto" | "attach" | "root" | "interval" | "mode" | "pause" | "resume" | "remove"));
        let _lock = if mutating { Some(config::ConfigLock::acquire(&dir)?) } else { None };
        let mut config = config::load(&dir)?;
        command(&args, &dir, &mut config)
    });
    if let Err(e) = result {
        eprintln!("오류: {}", e);
        process::exit(1);
    }
}
