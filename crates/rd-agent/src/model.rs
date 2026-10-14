use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    Observe,
    Manual,
    Auto,
}

impl SyncMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "observe" => Some(Self::Observe),
            "manual" => Some(Self::Manual),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: String,
    pub url: String,
    pub path: PathBuf,
    pub branch: String,
    pub mode: SyncMode,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub version: u32,
    pub interval_minutes: u64,
    pub repositories: Vec<Repository>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { version: 1, interval_minutes: 15, repositories: Vec::new() }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SyncStatus {
    Current,
    Updated { commits: u64 },
    UpdateAvailable { commits: u64 },
    LocalChanges,
    Diverged,
    LocalAhead,
    WrongBranch { actual: String },
    GitOperationInProgress,
    RemoteChanged,
    RemoteMissing,
    Paused,
    ObserveOnly,
    Busy,
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Current => write!(f, "최신 상태"),
            Self::Updated { commits } => write!(f, "{}개 커밋 반영 완료", commits),
            Self::UpdateAvailable { commits } => write!(f, "{}개 원격 커밋 업데이트 가능", commits),
            Self::LocalChanges => write!(f, "로컬 변경 사항 보호: 자동 업데이트 중단"),
            Self::Diverged => write!(f, "로컬/원격 브랜치 분기: 업데이트 중단"),
            Self::LocalAhead => write!(f, "원격에 없는 로컬 커밋 존재: 업데이트 중단"),
            Self::WrongBranch { actual } => write!(f, "다른 브랜치 사용 중({}): 업데이트 중단", actual),
            Self::GitOperationInProgress => write!(f, "Git 작업 진행 중: 업데이트 중단"),
            Self::RemoteChanged => write!(f, "등록 시 원격 주소와 현재 origin 주소가 다름"),
            Self::RemoteMissing => write!(f, "등록한 원격 브랜치를 찾을 수 없음"),
            Self::Paused => write!(f, "일시 정지됨"),
            Self::ObserveOnly => write!(f, "관찰 전용: 자동 수정 없음"),
            Self::Busy => write!(f, "다른 RD 작업이 진행 중"),
        }
    }
}
