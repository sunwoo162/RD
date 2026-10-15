# RD v0.3 · On-demand Manager

## 실행 원칙

- \`rd-tray.exe\`는 백그라운드에서 Rust만 사용합니다. React/Tauri UI는 상주하지 않습니다.
- \`rd-manager.exe\`는 프로젝트 관리가 필요할 때만 실행하고 닫으면 종료됩니다.
- \`rd-agent.exe\`는 Git 동기화 명령을 실행하는 프로세스로, 작업 후 종료됩니다.
- 새로운 레포 등록은 **보호 모드(manual)**가 기본입니다. 명시적 설정 없이는 자동 Pull이 활성화되지 않습니다.
- 트레이 앱을 종료해도 로컬 코드와 GitHub 저장소는 그대로 유지됩니다.

## 사용 방법

1. RD Windows Portable 워크플로우에서 ZIP을 내려받습니다.
2. 압축을 푼 뒤 같은 폴더에 세 실행 파일이 있는지 확인합니다.
3. \`rd-manager.exe\`를 열고 GitHub 저장소 주소, 새 로컬 폴더, 브랜치를 입력합니다.
4. RD 트레이를 실행하면 로그인된 세션에서 선택된 저장소를 주기적으로 확인합니다.
5. 자동 시작은 Preferences에서 **명시적으로** 켤 수 있습니다.

UI 실행 파일은 WebView2를 필요로 할 수 있지만, 트레이 Agent에는 WebView2가 필요하지 않습니다.
Private 저장소는 사용자의 기존 Git 인증 및 자격 증명 도우미에 의해 접근 가능한 경우에만 Clone됩니다.

## 개발

\`\`\`powershell
cd apps/rd-manager
npm install
npm run tauri dev
\`\`\`

개발 중에도 관리 UI 명령을 실행하려면 \`rd-agent.exe\`가 관리자 UI 실행 파일과 동일한 폴더에 있어야 합니다. 개발 중에는 우선 CLI를 직접 실행해도 됩니다.

## 측정

Windows에서 \`rd-tray.exe\`를 실행한 후:

\`\`\`powershell
.\scripts\measure-agent.ps1 -DurationSeconds 300 -OutputPath rd-performance.json
\`\`\`

이 스크립트는 프로세스 Working Set 및 전체 논리 프로세서 기준 CPU 사용률을 샘플링합니다. **30MB 이하, 평균 CPU 0.1% 미만은 미검증 목표**입니다. GUI가 꺼진 유휴 상태와 동기화 작업 중은 따로 측정해야 합니다. CI 빌드 성공은 실제 성능 목표 통과를 의미하지 않습니다.

## 제한

- Tauri Manager는 아직 실사용 QA 전입니다.
- 로컬 수정/이력 분기 등으로 업데이트가 보류되면 직접 원인을 확인해야 합니다.
- 브랜치 교체, 강제 Reset, 자동 Push는 수행하지 않습니다.
- 여러 GUI/CLI 프로세스에서 **동시에 설정 파일을 변경하는 경쟁 상황은 아직 추가 검증이 필요**합니다.
