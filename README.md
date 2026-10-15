# RD — RepoDock

> **Your code, already here.**

## 버전별 다운로드 (Windows 64비트)

원하는 **버전의 글자를 클릭하면 바로 다운로드**됩니다. 설치형 `Setup.exe`는 다운로드 후 실행해서 **다음 → 설치 → 완료** 순서로 설치하고, 포터블 ZIP은 압축을 풀고 `rd-manager.exe`를 실행하면 됩니다.

| 버전 | 간편 설치 (.exe) | 무설치 (.zip) | 상태 |
| --- | --- | --- | --- |
| **V1.1.0** | [V1.1.0 설치하기](https://github.com/sunwoo162/RD/releases/download/v1.1.0/RepoDock-Setup-v1.1.0.exe) | [V1.1.0 ZIP 다운로드](https://github.com/sunwoo162/RD/releases/download/v1.1.0/RD-v1.1.0-Windows-x64.zip) | **최신 정식** |
| **V1.0.0** | [V1.0.0 설치하기](https://github.com/sunwoo162/RD/releases/download/v1.0.0/RepoDock-Setup-v1.0.0.exe) | [V1.0.0 ZIP 다운로드](https://github.com/sunwoo162/RD/releases/download/v1.0.0/RD-v1.0.0-Windows-x64.zip) | 이전 정식 |
| V1.1.0 Beta 1 | [베타 설치하기](https://github.com/sunwoo162/RD/releases/download/v1.1.0-beta.1/RepoDock-Setup-v1.1.0-beta.1.exe) | [베타 ZIP 다운로드](https://github.com/sunwoo162/RD/releases/download/v1.1.0-beta.1/RD-v1.1.0-beta.1-Windows-x64.zip) | 실험 버전 |

[**모든 GitHub Releases 보기**](https://github.com/sunwoo162/RD/releases) · [V1.2 개발 상황 (아직 정식 출시 전)](https://github.com/sunwoo162/RD/pull/7)

- **간편 설치:** 설치 파일을 내려받아 직접 실행하세요. Windows가 웹페이지의 클릭만으로 프로그램을 자동 실행하는 것은 보안상 허용하지 않습니다.
- **버전별 독립 설치:** 각 버전은 `%LOCALAPPDATA%\Programs\RepoDock\v버전`에 설치되고, 바탕화면과 시작 메뉴에 버전명이 붙은 바로가기가 생깁니다.
- **안전한 버전 변경:** 프로젝트 폴더는 이동·삭제하지 않고, 기존 RD 설정은 `%LOCALAPPDATA%\RD`에 유지됩니다. 단 **모든 버전이 설정을 공유**하므로 기존 Manager/Tray를 종료하고 **한 번에 한 버전만 실행**하세요. 신버전 설정을 구버전으로 되돌리는 것은 보장하지 않습니다.
- **삭제:** Windows 설치된 앱 또는 설치 폴더의 `Uninstall.exe`에서 해당 버전만 제거하며 사용자 프로젝트와 RD 설정은 유지합니다.
- **참고:** 현재 설치형은 코드 서명이 없어 Windows SmartScreen 확인이 뜰 수 있습니다. 필요한 경우 릴리스 페이지에서 출처와 파일을 확인하세요. Git 및 WebView2 런타임은 별도로 필요합니다.

RD는 **사용자가 직접 등록한 GitHub 저장소만** 지정한 로컬 폴더로 Clone하고, 원격 커밋을 주기적으로 확인하며, 안전할 때만 자동 갱신하는 초경량 프로젝트 동기화 도구입니다.

**현재 단계: v1.1.0 Windows Portable.** 첫 저장 폴더 선택과 GitHub 주소 하나로 프로젝트를 자동 연결할 수 있습니다. Windows 실기기 및 CI 검증을 거쳤고, 사용 중인 로컬 파일은 기존 안전 정책으로 보호합니다. 코드 서명 및 24시간 이상 운영 테스트는 후속 검증이 필요합니다.

## V1.1 — GitHub URL 하나로 연결

- 처음 실행할 때 **문서** 등 상위 폴더를 한 번 선택하면 `RepoDock` 하위 폴더를 만들고 기억합니다.
- 이후 GitHub 주소만 입력하면 저장소 이름으로 폴더를 만들고 기본 브랜치를 자동 선택합니다.
- 같은 이름의 폴더가 이미 있으면 `저장소-소유자`, `저장소-소유자-2`로 충돌을 피합니다. 기존 폴더는 절대 덮어쓰지 않습니다.
- V1.1에서 새로 연결하는 프로젝트는 **Auto 모드**로 등록됩니다. 5/15/30/60분마다 원격 변경을 확인하고, 수정 중인 로컬 파일이나 브랜치 분기를 발견하면 업데이트를 중단합니다.
- `rd-manager.exe`를 열면 트레이 실행을 시도합니다. 관리 UI를 닫아도 백그라운드 트레이는 계속 실행됩니다. 로그인 자동 실행 설정은 사용자 선택 사항입니다.
- 설정에서 기본 폴더를 바꾸더라도 **이미 내려받은 프로젝트를 자동 이동하지 않습니다.**
- 기존 V1.0 CLI `add <url> <folder>` 및 Manual/Observe 정책은 계속 지원합니다.

```powershell
rd-agent root "$HOME\Documents\RepoDock"
rd-agent interval 15
rd-agent add-auto https://github.com/sunwoo162/RD.git
```

## 주요 기능

- 레포 URL + 폴더 경로로 Clone (기존 폴더 자동 덮어쓰기 금지)
- 여러 저장소 등록, 개수 제한 없음
- 주기적 원격 SHA 확인 + 변경된 브랜치만 Fetch
- Manual/Auto/Observe 동기화 정책
- 로컬 수정/미추적 파일, Git 작업, 현재 브랜치 불일치, 로컬 커밋 선행/분기 감지
- Fast-forward 업데이트만 허용, 강제 Reset/Clean/Push 금지
- 단일 스케줄러와 저장소별 작업 잠금
- Windows 전용 `rd-tray.exe`: WebView 없이 트레이에 상주, 필요할 때만 동기화 상태 확인
- `rd-manager.exe`: 프로젝트 등록, 검색, 안전한 동기화, 모드 전환, 일시 정지, 자동 시작 관리 (열 때만 WebView 실행)
- Windows 로그인 자동 시작: 명시적인 `autostart enable` 사용 시에만 활성화

## 개발 환경

- Git 및 Rust stable + Cargo
- 테스트: `cargo test --workspace`
- 릴리스 빌드: `cargo build --release -p rd-agent`
- Windows CLI 실행 파일: `target\release\rd-agent.exe`

## 명령 예시

```powershell
rd-agent add https://github.com/sunwoo162/ain.git "$HOME\Documents\Projects\ain"
rd-agent list
rd-agent check
rd-agent sync <ID>
rd-agent mode <ID> auto
rd-agent watch
rd-agent autostart enable
rd-agent autostart status
rd-agent autostart disable
```

| 명령 | 동작 |
| --- | --- |
| `add <url> <folder> [--branch <name>] [--auto]` | 신규 폴더 Clone 및 관리 목록 등록 |
| `list` | 등록된 저장소 목록 표시 |
| `check [id]` | Fetch + 상태 검사. **작업 파일 변경 없음** |
| `sync [id]` | 보호 조건 통과 시 승인된 1회 업데이트 |
| `mode <id> <observe\|manual\|auto>` | 동기화 정책 변경 |
| `pause <id>` / `resume <id>` | 동기화 감시 중단/재개 |
| `remove <id>` | RD 등록 해제 (로컬 파일 보존) |
| `watch` | 15분 기본 간격 감시 CLI 실행 |
| `autostart enable/disable/status` | Windows 로그인 자동 실행 등록·해제·조회 |

기본 모드는 `manual`이며 **Auto는 사용자가 명시적으로 선택한 저장소에만 적용**됩니다. `watch`는 콘솔 모드로 실행됩니다.

### Windows Portable (v1.1.0)

1. GitHub Actions의 **RD Windows Portable** 실행 결과에서 ZIP을 다운로드해 압축 해제합니다.
2. 동일한 폴더에 있는 `rd-agent.exe`로 저장소를 등록합니다.
3. `rd-manager.exe`를 실행하면 Tauri 관리 화면이 열립니다. 새 레포 등록, 보호/자동/관찰 모드, 일시 정지, 1회 갱신을 할 수 있습니다.
4. `rd-tray.exe`를 실행하면 시스템 트레이에 상주합니다. 더블클릭 또는 우클릭 메뉴로 관리 화면을 열고, **지금 변경 확인(파일 미수정)**, **설정 폴더 열기**, **RD 종료**를 사용할 수 있습니다.
5. 원하면 RD 관리 화면의 Preferences 또는 `rd-agent autostart enable`로 로그인 자동 실행을 켭니다. `disable`로 되돌릴 수 있습니다.

**자동 시작은 동의 없이 켜지지 않습니다.** 포터블 폴더를 옮겼다면 `autostart enable`을 다시 실행해 경로를 갱신해야 합니다. 프로그램은 관리자 권한을 요구하지 않지만 Git이 설치되어 있고 저장소 접근 권한이 있어야 합니다. 트레이는 관리 UI가 아니며 초기 버전은 설정 폴더를 여는 기능만 제공합니다.

설정 위치: Windows `%LOCALAPPDATA%\RD\config.json`, Linux/macOS `~/.config/rd/config.json`. 실험/테스트에서는 환경 변수 `RD_CONFIG_DIR`를 사용할 수 있습니다.

## 안전과 한계

- 외부 Git 작업과의 모든 경쟁 조건을 제거하는 것은 불가능합니다.
- `git merge --ff-only --no-overwrite-ignore`는 정상 Fast-forward에서 원격에서 삭제된 추적 파일을 로컬에서도 삭제할 수 있습니다.
- RD의 테스트는 PR의 GitHub Actions에서도 검증됩니다. CI 통과 전 실사용을 권장하지 않습니다.
- 이 프로젝트는 코드 백업 프로그램이나 양방향 동기화 서비스가 아닙니다.
- 성능 목표: 유휴 RAM 30MB 이하, 평균 CPU 0.1% 미만. 단시간 샘플 수치는 장시간 안정성 보장이 아니며 사용 환경에 따라 달라질 수 있습니다.
- Git 명령 제한 시간은 기본 180초이며 `RD_GIT_TIMEOUT_SECS` 환경 변수로 1~3600초 범위에서 변경할 수 있습니다.
- config.json은 임시 파일을 플러시한 뒤 같은 파일 시스템에서 원자적으로 교체합니다. 사용자 코드 파일에는 영향을 주지 않습니다.

추가 안전 원칙은 [동기화 정책](docs/SYNC_POLICY.md)을 참고하세요.

## 로드맵

- [x] Rust 동기화 엔진 + CLI 소스 작성
- [x] 로컬 변경 보호 + 분기 감지 + 자동화 테스트 코드 작성
- [x] 기존 Rust CI 통과 확인 (PR #1)
- [x] Windows 트레이·자동 시작 CI 검증
- [x] 필요할 때만 실행되는 Tauri 관리 UI 소스 작성
- [x] Windows 관리 UI 기동 및 렌더링 검증
- [ ] Windows 관리 UI 주요 버튼 클릭 자동화 테스트
- [x] Windows Portable ZIP 자동 빌드 및 Windows 실기기 실행 검증
- [x] Windows 메모리·CPU 측정 스크립트 작성
- [x] Windows 10/50/100개 로컬 Git 저장소 검사 성능 실험
- [ ] RAM·CPU 장시간 측정 및 반복 테스트
- [ ] 최종 PR 병합 및 공식 GitHub Releases 배포

관리 UI의 실행·성능 측정 방법은 [Manager 안내](docs/MANAGER.md)를, Windows 10/50/100개 저장소 성능 실측 결과는 [벤치마크 보고서](docs/BENCHMARKS.md)를 참고하세요.
