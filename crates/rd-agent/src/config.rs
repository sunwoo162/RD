use crate::model::Settings;
use fs2::FileExt;
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Prevent simultaneous CLI writers from saving stale copies of config.json.
pub struct ConfigLock(File);

impl ConfigLock {
    pub fn acquire(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let file = OpenOptions::new().create(true).truncate(false).read(true).write(true)
            .open(dir.join("config.lock"))
            .map_err(|e| format!("설정 잠금 파일을 열 수 없습니다: {}", e))?;
        file.try_lock_exclusive()
            .map_err(|_| "다른 RD 명령이 설정을 변경하고 있습니다. 잠시 후 다시 시도하세요".to_string())?;
        Ok(Self(file))
    }
}

impl Drop for ConfigLock {
    fn drop(&mut self) { let _ = FileExt::unlock(&self.0); }
}

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

#[cfg(windows)]
fn replace_atomic(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::{os::windows::ffi::OsStrExt, iter::once};
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let from: Vec<u16> = source.as_os_str().encode_wide().chain(once(0)).collect();
    let to: Vec<u16> = target.as_os_str().encode_wide().chain(once(0)).collect();
    if unsafe {
        MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)
    } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_atomic(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(source, target)
}

/// Write and flush a unique sibling file, then atomically replace config.json.
/// Readers observe either the old complete settings or the new complete settings.
pub fn save(dir: &Path, settings: &Settings) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let target = dir.join("config.json");
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?.as_nanos();
    let temporary = dir.join(format!(".config-{}-{}.tmp", std::process::id(), nonce));
    let data = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new().write(true).create_new(true)
            .open(&temporary).map_err(|e| e.to_string())?;
        file.write_all(&data).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        replace_atomic(&temporary, &target).map_err(|e| format!("설정 저장 실패: {}", e))
    })();
    if result.is_err() { let _ = fs::remove_file(&temporary); }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_saves_replace_existing_config_without_losing_data() {
        let temp = tempfile::tempdir().unwrap();
        for i in 0..30 {
            let settings = Settings { interval_minutes: 1 + i, ..Settings::default() };
            save(temp.path(), &settings).unwrap();
            assert_eq!(load(temp.path()).unwrap().interval_minutes, 1 + i);
        }
        assert!(!fs::read_dir(temp.path()).unwrap().any(|entry| {
            entry.unwrap().file_name().to_string_lossy().ends_with(".tmp")
        }));
    }

    #[test]
    fn subsequent_save_updates_existing_config() {
        let temp = tempfile::tempdir().unwrap();
        let settings = Settings { interval_minutes: 42, ..Settings::default() };
        save(temp.path(), &settings).unwrap();
        let mut updated = settings.clone();
        updated.interval_minutes = 84;
        save(temp.path(), &updated).unwrap();
        assert_eq!(load(temp.path()).unwrap().interval_minutes, 84);
    }
}
