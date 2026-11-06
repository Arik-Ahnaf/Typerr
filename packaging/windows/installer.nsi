Unicode true
RequestExecutionLevel user

!define APP_NAME "Typerr"
!define APP_VERSION "@VERSION@"
!define APP_PUBLISHER "Arik Ahnaf"
!define APP_URL "https://github.com/Arik-Ahnaf/Typerr"
!define APP_EXE "typerr.exe"

Name "${APP_NAME} ${APP_VERSION}"
OutFile "@OUTPUT@"
InstallDir "$LOCALAPPDATA\Programs\Typerr"
InstallDirRegKey HKCU "Software\Typerr" "InstallDir"
Icon "@ICON@"
UninstallIcon "@ICON@"
BrandingText "Typerr"
ShowInstDetails show
ShowUninstDetails show

VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "ProductVersion" "${APP_VERSION}"
VIAddVersionKey "FileDescription" "${APP_NAME} installer"
VIAddVersionKey "FileVersion" "${APP_VERSION}"
VIAddVersionKey "CompanyName" "${APP_PUBLISHER}"

!include "MUI2.nsh"
!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Typerr" SEC_APP
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  RMDir /r "$INSTDIR\notices"
  File /r "@PAYLOAD@\*"
  IfFileExists "$INSTDIR\settings.json" settings_done
  File /oname=settings.json "@SETTINGS@"
settings_done:
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Typerr" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Typerr" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Typerr" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Typerr" "Publisher" "${APP_PUBLISHER}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Typerr" "URLInfoAbout" "${APP_URL}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Typerr" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Typerr" "DisplayIcon" "$INSTDIR\${APP_EXE}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Typerr" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Typerr" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Typerr" "NoRepair" 1
  CreateDirectory "$SMPROGRAMS\Typerr"
  CreateShortcut "$SMPROGRAMS\Typerr\Typerr.lnk" "$INSTDIR\${APP_EXE}"
  CreateShortcut "$SMPROGRAMS\Typerr\Uninstall Typerr.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\Typerr.lnk" "$INSTDIR\${APP_EXE}"
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  Delete "$DESKTOP\Typerr.lnk"
  RMDir /r "$SMPROGRAMS\Typerr"
  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\README.md"
  RMDir /r "$INSTDIR\notices"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Typerr"
  DeleteRegKey HKCU "Software\Typerr"
SectionEnd
