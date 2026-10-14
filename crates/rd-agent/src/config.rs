use crate::model::Settings;
use std::{env, fs, io::Write, path::{Path, PathBuf}};

pub fn data_dir() -> Result<PathBuf, String> {
    if let Some(dir) = env::var_os("RD_CONFIG_DIR") {
        return Ok(PathBuf::from(dir));
    }
    #[cfg(windows)]
    {
        let base = env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA를 찾지 못했습니다")?;
        Ok(PathBuf::from(base).join("RD"))
    }
    #[cfg(not(windows))]
    {
        let home = env::var_os("HOME").ok_or("HOME을 찾지 못했습니다")?;
        Ok(PathBuf::from(home).join(".config").join("rd"))
    }
}

pub fn load(dir: &Path) -> Result<Settings, String> {
    let path = dir.join("config.json");
    if !path.exists() { return Ok(Settings::default()); }
    let text = fs::read_to_string(&path).map_err(|e| format!("설정 읽기 실패: {}", e))?;
    let config: Settings = serde_json::from_str(&text).map_err(|e| format!("설정 형식 오류: {}", e))?;
    if config.version != 1 { return Err(format!("지원하지 않는 설정 버전: {}", config.version)); }
    Ok(config)
}

pub fn save(dir: &Path, settings: &Settings) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let target = dir.join("config.json");
    let temporary = dir.join(format!(".config-{}.tmp", std::process::id()));
    let data = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
    file.write_all(&data).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(&temporary, &target).map_err(|e| format!("설정 저장 실패: {}", e))?;
    Ok(())
}
