Unicode true
!include "MUI2.nsh"
!ifndef APP_VERSION
 !error "APP_VERSION required"
!endif
!ifndef SOURCE_DIR
 !error "SOURCE_DIR required"
!endif
!ifndef OUTPUT_EXE
 !error "OUTPUT_EXE required"
!endif
Name "RepoDock ${APP_VERSION}"
OutFile "${OUTPUT_EXE}"
InstallDir "$LOCALAPPDATA\Programs\RepoDock\${APP_VERSION}"
RequestExecutionLevel user
SetCompressor /SOLID lzma
ShowInstDetails show
ShowUninstDetails show

!define MUI_FINISHPAGE_RUN "$INSTDIR\rd-manager.exe"
!define MUI_FINISHPAGE_RUN_TEXT "RepoDock ${APP_VERSION} 실행"
!define MUI_FINISHPAGE_RUN_NOTCHECKED
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Korean"
!insertmacro MUI_LANGUAGE "English"

Section "RepoDock"
 SetShellVarContext current
 SetOutPath "$INSTDIR"
 File "${SOURCE_DIR}\rd-manager.exe"
 File "${SOURCE_DIR}\rd-agent.exe"
 File "${SOURCE_DIR}\rd-tray.exe"
 File "${SOURCE_DIR}\README.md"
 File "${SOURCE_DIR}\LICENSE"
 WriteUninstaller "$INSTDIR\Uninstall.exe"
 CreateDirectory "$SMPROGRAMS\RepoDock"
 CreateShortCut "$SMPROGRAMS\RepoDock\RepoDock ${APP_VERSION}.lnk" "$INSTDIR\rd-manager.exe"
 CreateShortCut "$DESKTOP\RepoDock ${APP_VERSION}.lnk" "$INSTDIR\rd-manager.exe"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RepoDock-${APP_VERSION}" "DisplayName" "RepoDock ${APP_VERSION}"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RepoDock-${APP_VERSION}" "DisplayVersion" "${APP_VERSION}"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RepoDock-${APP_VERSION}" "Publisher" "RepoDock"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RepoDock-${APP_VERSION}" "InstallLocation" "$INSTDIR"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RepoDock-${APP_VERSION}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
 WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RepoDock-${APP_VERSION}" "NoModify" 1
 WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RepoDock-${APP_VERSION}" "NoRepair" 1
SectionEnd

Section "Uninstall"
 SetShellVarContext current
 ; Never delete shared configuration or user repositories.
 Delete "$DESKTOP\RepoDock ${APP_VERSION}.lnk"
 Delete "$SMPROGRAMS\RepoDock\RepoDock ${APP_VERSION}.lnk"
 RMDir "$SMPROGRAMS\RepoDock"
 Delete "$INSTDIR\rd-manager.exe"
 Delete "$INSTDIR\rd-agent.exe"
 Delete "$INSTDIR\rd-tray.exe"
 Delete "$INSTDIR\README.md"
 Delete "$INSTDIR\LICENSE"
 Delete "$INSTDIR\Uninstall.exe"
 RMDir "$INSTDIR"
 DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\RepoDock-${APP_VERSION}"
SectionEnd
