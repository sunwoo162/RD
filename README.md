# RD — RepoDock

> **Your code, already here.**

RD는 **사용자가 직접 등록한 GitHub 저장소만** 지정한 로컬 폴더로 Clone하고, 원격 커밋을 주기적으로 확인하며, 안전할 때만 자동 갱신하는 초경량 프로젝트 동기화 도구입니다.

**현재 단계: v0.2 사전 검증용.** 초경량 Windows 네이티브 트레이와 옵트인 자동 시작을 구현 중입니다. Tauri 관리 UI와 정식 릴리스는 아직 없으며 RAM·CPU 목표도 실측 전입니다.

## 주요 기능

- 레포 URL + 폴더 경로로 Clone (기존 폴더 자동 덮어쓰기 금지)
- 여러 저장소 등록, 개수 제한 없음
- 주기적 원격 SHA 확인 + 변경된 브랜치만 Fetch
- Manual/Auto/Observe 동기화 정책
- 로컬 수정/미추적 파일, Git 작업, 현재 브랜치 불일치, 로컬 커밋 선행/분기 감지
- Fast-forward 업데이트만 허용, 강제 Reset/Clean/Push 금지
- 단일 스케줄러와 저장소별 작업 잠금
- Windows 전용 `rd-tray.exe`: WebView 없이 트레이에 상주, 필요할 때만 동기화 상태 확인
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

### Windows Portable (개발 버전)

1. GitHub Actions의 **RD Windows Portable** 실행 결과에서 ZIP을 다운로드해 압축 해제합니다.
2. 동일한 폴더에 있는 `rd-agent.exe`로 저장소를 등록합니다.
3. `rd-tray.exe`를 실행하면 시스템 트레이에 상주합니다. 우클릭으로 **지금 변경 확인(파일 미수정)**, **설정 폴더 열기**, **RD 종료**를 선택할 수 있습니다.
4. 원하면 터미널에서 `rd-agent autostart enable`을 입력해 현재 Windows 사용자 로그인 시 자동 시작하도록 설정합니다. `disable`로 되돌릴 수 있습니다.

**자동 시작은 동의 없이 켜지지 않습니다.** 포터블 폴더를 옮겼다면 `autostart enable`을 다시 실행해 경로를 갱신해야 합니다. 프로그램은 관리자 권한을 요구하지 않지만 Git이 설치되어 있고 저장소 접근 권한이 있어야 합니다. 트레이는 관리 UI가 아니며 초기 버전은 설정 폴더를 여는 기능만 제공합니다.

설정 위치: Windows `%LOCALAPPDATA%\RD\config.json`, Linux/macOS `~/.config/rd/config.json`. 실험/테스트에서는 환경 변수 `RD_CONFIG_DIR`를 사용할 수 있습니다.

## 안전과 한계

- 외부 Git 작업과의 모든 경쟁 조건을 제거하는 것은 불가능합니다.
- `git merge --ff-only --no-overwrite-ignore`는 정상 Fast-forward에서 원격에서 삭제된 추적 파일을 로컬에서도 삭제할 수 있습니다.
- RD의 테스트는 PR의 GitHub Actions에서도 검증됩니다. CI 통과 전 실사용을 권장하지 않습니다.
- 이 프로젝트는 코드 백업 프로그램이나 양방향 동기화 서비스가 아닙니다.
- 성능 목표: 유휴 RAM 30MB 이하, CPU 0.1% 미만 (**아직 미검증**).

추가 안전 원칙은 [동기화 정책](docs/SYNC_POLICY.md)을 참고하세요.

## 로드맵

- [x] Rust 동기화 엔진 + CLI 소스 작성
- [x] 로컬 변경 보호 + 분기 감지 + 자동화 테스트 코드 작성
- [x] 기존 Rust CI 통과 확인 (PR #1)
- [ ] Windows 트레이·자동 시작 CI 검증
- [ ] 필요할 때만 실행되는 Tauri 관리 UI
- [ ] Windows Portable ZIP 자동 빌드 검증 (릴리스 배포 전)
- [ ] RAM·CPU·100개 레포 성능 측정
