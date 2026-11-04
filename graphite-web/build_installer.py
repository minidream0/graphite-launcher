"""Build a per-user NSIS installer from the clean portable release, never from installed user data."""
from pathlib import Path
import argparse, json, shutil, subprocess

p=argparse.ArgumentParser()
p.add_argument('--output',type=Path,required=True)
p.add_argument('--tools',type=Path,required=True)
a=p.parse_args()
out=a.output.resolve();tools=a.tools.resolve();runtime=out/'Graphite'
manifest=json.loads((runtime/'release.json').read_text());version=manifest['version']
assert manifest['runtime']=='rust-webview2' and not (runtime/'data').exists()
source=out/'Graphite-Rust-source.zip'
assert source.is_file()
files=sorted(f for f in runtime.rglob('*') if f.is_file())
assert files and not any(f.name in {'accounts.json','graphite-offline-accounts.json','graphite-servers.json'} for f in files)
def q(s):
    return str(s).replace('$','$$').replace('"','$\\"')
def rel(f):return str(f.relative_to(runtime))
dirs=sorted({str(f.parent.relative_to(runtime)) for f in files if f.parent!=runtime},key=lambda s:(-s.count('\\'),s))
script=out/'Graphite-installer.nsi'
text=r'''Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
!include "WinVer.nsh"
Name "Graphite"
OutFile "@SETUP@"
InstallDir "$LOCALAPPDATA\Programs\Graphite"
InstallDirRegKey HKCU "Software\Graphite" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
ShowInstDetails show
ShowUninstDetails show
!define MUI_ICON "@ICON@"
!define MUI_UNICON "@ICON@"
!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Russian"
!insertmacro MUI_LANGUAGE "English"

Function .onInit
 ${IfNot} ${RunningX64}
 ${OrIfNot} ${AtLeastWin10}
   IfSilent +2
   MessageBox MB_OK|MB_ICONSTOP "Graphite требует 64-битную Windows 10 или 11."
   SetErrorLevel 5
   Abort
 ${EndIf}
FunctionEnd

!macro CheckClosed PREFIX
Function ${PREFIX}CheckClosed
 retry:
 ; Check only the target runtime; an Explorer folder can also be titled Graphite.
 IfFileExists "$INSTDIR\Graphite.exe" 0 engine
 System::Call 'kernel32::CreateFileW(w "$INSTDIR\Graphite.exe", i 0x40000000, i 1, p 0, i 3, i 0, p 0) p.r0'
 ${If} $0 == -1
   IfSilent blocked
   MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Закройте Graphite перед продолжением." IDRETRY retry
   Abort
 ${EndIf}
 System::Call 'kernel32::CloseHandle(p r0)'
 engine:
 IfFileExists "$INSTDIR\engine\prismlauncher.exe" 0 done
 System::Call 'kernel32::CreateFileW(w "$INSTDIR\engine\prismlauncher.exe", i 0x40000000, i 1, p 0, i 3, i 0, p 0) p.r0'
 ${If} $0 == -1
   IfSilent blocked
   MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Завершите Minecraft и Graphite: движок ещё используется." IDRETRY retry
   Abort
 ${EndIf}
 System::Call 'kernel32::CloseHandle(p r0)'
 done:
 Return
 blocked:
 SetErrorLevel 2
 Abort
FunctionEnd
!macroend
!insertmacro CheckClosed ""
!insertmacro CheckClosed "un."

Section "Graphite"
 Call CheckClosed
 SetShellVarContext current
 ; Runtime detection must cover machine-wide and per-user WebView2 installations.
 SetRegView 32
 ReadRegStr $0 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
 ${If} $0 == ""
 ${OrIf} $0 == "0.0.0.0"
   ReadRegStr $0 HKCU "Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
 ${EndIf}
 ${If} $0 == ""
 ${OrIf} $0 == "0.0.0.0"
   InitPluginsDir
   SetOutPath "$PLUGINSDIR"
   File "@WEBVIEW@"
   ExecWait '"$PLUGINSDIR\MicrosoftEdgeWebview2Setup.exe" /silent /install' $1
   ReadRegStr $0 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
   ${If} $0 == ""
   ${OrIf} $0 == "0.0.0.0"
     ReadRegStr $0 HKCU "Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
   ${EndIf}
   ${If} $0 == ""
   ${OrIf} $0 == "0.0.0.0"
     IfSilent +2
     MessageBox MB_OK|MB_ICONSTOP "Не установлен Microsoft WebView2. Проверьте подключение к Интернету и повторите установку."
     SetErrorLevel 3
     Abort
   ${EndIf}
 ${EndIf}
 SetOverwrite on
 ClearErrors
@FILES@
 SetOutPath "$INSTDIR\source"
 File /oname=Graphite-source-@VERSION@.zip "@SOURCE@"
 IfErrors install_failed
 SetOutPath "$INSTDIR"
 WriteUninstaller "$INSTDIR\Uninstall.exe"
 IfErrors install_failed
 WriteRegStr HKCU "Software\Graphite" "InstallDir" "$INSTDIR"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Graphite" "DisplayName" "Graphite"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Graphite" "DisplayVersion" "@VERSION@"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Graphite" "DisplayIcon" "$INSTDIR\Graphite.exe"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Graphite" "InstallLocation" "$INSTDIR"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Graphite" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Graphite" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
 WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Graphite" "NoModify" 1
 WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Graphite" "NoRepair" 1
 CreateDirectory "$SMPROGRAMS\Graphite"
 CreateShortcut "$SMPROGRAMS\Graphite\Graphite.lnk" "$INSTDIR\Graphite.exe"
 Goto installed
 install_failed:
 IfSilent +2
 MessageBox MB_OK|MB_ICONSTOP "Не удалось записать файлы Graphite. Проверьте свободное место и права доступа."
 SetErrorLevel 4
 Abort
 installed:
SectionEnd

Section "Uninstall"
 Call un.CheckClosed
 SetShellVarContext current
 SetRegView 32
 ; Only packaged files are removed: data, worlds, profiles and unknown files are retained.
@DELETE@
 Delete "$INSTDIR\source\Graphite-source-@VERSION@.zip"
 RMDir "$INSTDIR\source"
 Delete "$INSTDIR\Uninstall.exe"
 RMDir "$INSTDIR"
 ReadRegStr $0 HKCU "Software\Graphite" "InstallDir"
 ${If} $0 == $INSTDIR
   DeleteRegKey HKCU "Software\Graphite"
   DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Graphite"
   Delete "$SMPROGRAMS\Graphite\Graphite.lnk"
   RMDir "$SMPROGRAMS\Graphite"
 ${EndIf}
SectionEnd
'''
commands=[]
for f in files:
    folder=f.parent.relative_to(runtime)
    commands.append(' SetOutPath "$INSTDIR'+('\\'+q(folder) if str(folder)!='.' else '')+'"')
    commands.append(f' File "{q(f)}"')
    commands.append(' IfErrors install_failed')
deletes=[f' Delete "$INSTDIR\\{q(rel(f))}"' for f in files]+[f' RMDir "$INSTDIR\\{q(d)}"' for d in dirs]
values={'SETUP':out/f'Graphite-Setup-{version}.exe','ICON':Path(__file__).parent/'src-tauri/icons/icon.ico','WEBVIEW':tools/'MicrosoftEdgeWebview2Setup.exe','FILES':'\n'.join(commands),'DELETE':'\n'.join(deletes),'VERSION':version,'SOURCE':source}
for k,v in values.items():text=text.replace('@'+k+'@',str(v) if k in {'FILES','DELETE'} else q(v))
script.write_text(text,encoding='utf-8-sig')
subprocess.run([str(tools/'nsis/nsis-3.08/makensis.exe'),'/V2',str(script)],check=True)
shutil.copy2(out/f'Graphite-Setup-{version}.exe',out/'Graphite-Setup.exe')
print(f'Built Graphite-Setup-{version}.exe: {(out/f"Graphite-Setup-{version}.exe").stat().st_size:,} bytes; no user data')
