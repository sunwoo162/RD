use rd_agent::{config, git, model::{Repository, Settings, SyncMode}, sync::{self, SyncIntent}};
use std::{collections::HashMap, env, path::{Path, PathBuf}, process, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

fn usage() {
    println!("RD Agent v0.1\n\
  rd-agent add <github-url> <folder> [--branch <name>] [--auto]\n\
  rd-agent list\n\
  rd-agent check [id]\n\
  rd-agent sync [id]       # 직접 승인하는 1회 업데이트\n\
  rd-agent mode <id> <observe|manual|auto>\n\
  rd-agent pause <id> | resume <id> | remove <id>\n\
  rd-agent watch           # 백그라운드 동기화 엔진(콘솔 모드)\n\n\
기본값: 15분 주기, Manual 모드. 자동 Pull은 --auto나 mode auto로 명시한 저장소에만 적용됩니다.\n\
등록 해제는 GitHub 저장소와 로컬 파일을 삭제하지 않습니다.");
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

fn command(args: &[String], config_dir: &Path, config: &mut Settings) -> Result<(), String> {
    match args.first().map(String::as_str).unwrap_or("help") {
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
            for repo in repos {
                let intent = if args[0] == "sync" { SyncIntent::Sync } else { SyncIntent::Check };
                match sync::inspect(repo, config_dir, intent) {
                    Ok(status) => println!("[{}] {}", repo.path.display(), status),
                    Err(error) => eprintln!("[{}] 오류: {}", repo.path.display(), error),
                }
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
        "help" | "--help" | "-h" => usage(),
        other => return Err(format!("알 수 없는 명령: {}", other)),
    }
    Ok(())
}

fn watch(config_dir: &Path) -> Result<(), String> {
    use fs2::FileExt;
    use std::fs::{self, OpenOptions};
    fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let file = OpenOptions::new().create(true).read(true).write(true)
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
        let mut config = config::load(&dir)?;
        command(&args, &dir, &mut config)
    });
    if let Err(e) = result {
        eprintln!("오류: {}", e);
        process::exit(1);
    }
}
