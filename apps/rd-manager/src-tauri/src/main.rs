#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rd_agent::{config, model::{Repository, Settings}};
use std::process::Command;

#[tauri::command]
fn list_repositories() -> Result<Vec<Repository>, String> {
    let dir = config::data_dir()?;
    Ok(config::load(&dir)?.repositories)
}

#[tauri::command]
fn get_preferences() -> Result<Settings, String> {
    config::load(&config::data_dir()?)
}

#[tauri::command]
fn open_repository_folder(id: String) -> Result<(), String> {
    let settings = config::load(&config::data_dir()?)?;
    let repo = settings.repositories.iter().find(|r| r.id == id).ok_or("등록되지 않은 프로젝트입니다")?;
    if !repo.path.is_dir() { return Err("프로젝트 폴더가 존재하지 않습니다".into()); }
    #[cfg(windows)]
    {
        Command::new("explorer.exe").arg(&repo.path).spawn().map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("폴더 열기는 현재 Windows에서 지원합니다".into())
    }
}

// Run only allowlisted CLI operations and pass arguments without a shell.
// The CLI owns Git validation and persistence; UI does not write config.json.
#[tauri::command]
async fn run_action(
    action: String,
    id: Option<String>,
    url: Option<String>,
    folder: Option<String>,
    branch: Option<String>,
    mode: Option<String>,
) -> Result<String, String> {
    let args = match action.as_str() {
        "add" => {
            let url = url.ok_or("GitHub 주소를 입력하세요")?;
            let folder = folder.ok_or("저장할 폴더를 입력하세요")?;
            let mut argv = vec!["add".to_string(), url, folder];
            if let Some(branch) = branch.filter(|b| !b.trim().is_empty()) {
                argv.extend(["--branch".into(), branch]);
            }
            // Never opt in to unattended updates during registration.
            argv
        }
        "add_auto" => {
            let url = url.ok_or("GitHub 주소를 입력하세요")?;
            vec!["add-auto".into(), url]
        }
        "root" => {
            let path = folder.ok_or("기본 저장 폴더를 선택하세요")?;
            vec!["root".into(), path]
        }
        "interval" => {
            let minutes = mode.ok_or("주기를 선택하세요")?;
            if !["5", "15", "30", "60"].contains(&minutes.as_str()) {
                return Err("잘못된 자동 확인 주기입니다".into());
            }
            vec!["interval".into(), minutes]
        }
        "check" | "sync" | "pause" | "resume" | "remove" => {
            let id = id.ok_or("저장소 ID가 필요합니다")?;
            vec![action, id]
        }
        "mode" => {
            let id = id.ok_or("저장소 ID가 필요합니다")?;
            let mode = mode.ok_or("동기화 모드가 필요합니다")?;
            if !["manual", "auto", "observe"].contains(&mode.as_str()) {
                return Err("지원하지 않는 동기화 모드입니다".into());
            }
            vec!["mode".into(), id, mode]
        }
        "autostart" => {
            let mode = mode.ok_or("자동 시작 설정 값이 필요합니다")?;
            if !["enable", "disable", "status"].contains(&mode.as_str()) {
                return Err("지원하지 않는 자동 시작 설정입니다".into());
            }
            vec!["autostart".into(), mode]
        }
        _ => return Err("허용되지 않은 작업입니다".into()),
    };
    tauri::async_runtime::spawn_blocking(move || {
        let current = std::env::current_exe().map_err(|e| e.to_string())?;
        let binary = current.with_file_name(if cfg!(windows) { "rd-agent.exe" } else { "rd-agent" });
        if !binary.is_file() {
            return Err("rd-agent 실행 파일이 없습니다. RD 포터블 폴더를 확인하세요.".into());
        }
        let result = Command::new(binary).args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "never")
            .output().map_err(|e| e.to_string())?;
        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);
        if !result.status.success() {
            // Avoid displaying raw stderr: it can contain a remote URL with credentials.
            return Err("요청한 작업을 완료하지 못했습니다. 저장소 접근 권한과 로컬 상태를 확인하세요.".into());
        }
        let combined = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            format!("{}\n{}", stdout.trim(), stderr.trim())
        };
        Ok(combined)
    }).await.map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|_| {
            // Running the Manager does not require running the heavier UI forever.
            // The tray remains alive when the Manager window is closed.
            if cfg!(windows) && !cfg!(debug_assertions) {
                if let Ok(manager) = std::env::current_exe() {
                    let tray = manager.with_file_name("rd-tray.exe");
                    if tray.is_file() {
                        let _ = Command::new(tray).spawn();
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![list_repositories, get_preferences, open_repository_folder, run_action])
        .run(tauri::generate_context!())
        .expect("RD Manager 실행 실패");
}
