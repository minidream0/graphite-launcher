from pathlib import Path
import subprocess,time,json,hashlib,winreg,uuid,ctypes
from ctypes import wintypes

root=Path(__file__).resolve().parent.parent
version=json.loads((root/'outputs/Graphite/release.json').read_text())['version']
setup=root/f'outputs/Graphite-Setup-{version}.exe'
target=root/'work'/f'qa installer {int(time.time())}'
key=r'Software\Microsoft\Windows\CurrentVersion\Uninstall\Graphite'
def registered():
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER,key,0,winreg.KEY_READ|winreg.KEY_WOW64_32KEY) as k:return winreg.QueryValueEx(k,'InstallLocation')[0]
    except FileNotFoundError:return None
assert registered() is None,'Do not replace a real user installation during QA'
assert not target.exists()
def install():
    # NSIS /D must be the final unquoted argument, including when the directory has spaces.
    return subprocess.run(f'"{setup}" /S /D={target}',timeout=180).returncode
user32=ctypes.WinDLL('user32',use_last_error=True)
user32.CreateWindowExW.argtypes=[wintypes.DWORD,wintypes.LPCWSTR,wintypes.LPCWSTR,wintypes.DWORD,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_int,wintypes.HWND,wintypes.HMENU,wintypes.HINSTANCE,ctypes.c_void_p]
user32.CreateWindowExW.restype=wintypes.HWND
user32.DestroyWindow.argtypes=[wintypes.HWND]
# An unrelated window title must not block installation into a fresh directory.
title_probe=user32.CreateWindowExW(0,'STATIC','Graphite',0x80000000,0,0,1,1,None,None,None,None)
assert title_probe,ctypes.get_last_error()
try:first_exit=install()
finally:user32.DestroyWindow(title_probe)
assert first_exit==0,f'Clean installation returned {first_exit}'
assert (target/'Graphite.exe').is_file() and Path(registered())==target
assert not (target/'data').exists(),'Installer shipped user data'
assert (target/f'source/Graphite-source-{version}.zip').is_file()
manifest=json.loads((target/'release.json').read_text())
assert hashlib.sha256((target/'Graphite.exe').read_bytes()).hexdigest()==manifest['launcherSha256']
print('PASS clean per-user installation despite unrelated Graphite window, source inclusion and runtime hashes',flush=True)
pipe='graphite-'+uuid.uuid4().hex
guard=root/'work'/('qa-installer-guard-'+uuid.uuid4().hex);guard.mkdir()
worker=subprocess.Popen([str(target/'engine/prismlauncher.exe'),'--dir',str(guard),'--design-service',pipe],creationflags=subprocess.CREATE_NO_WINDOW,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
try:
    for _ in range(100):
        try:
            connection=open('\\\\.\\pipe\\'+pipe,'r+b',buffering=0);break
        except OSError:time.sleep(.1)
    else:raise AssertionError('Worker did not start')
    assert install()==2,'Installer must reject a running engine'
    connection.write(b'{"method":"shutdown","params":{}}\n');connection.close();worker.wait(timeout=15)
    print('PASS installer blocks replacement of a running native engine',flush=True)
finally:
    if worker.poll() is None:worker.terminate();worker.wait(timeout=15)
sentinel=target/'data/instances/test/minecraft/saves/QA/level.dat';sentinel.parent.mkdir(parents=True);sentinel.write_bytes(b'world-preservation-check')
profiles=target/'data/graphite-offline-accounts.json';profiles.write_text('{"version":1,"profiles":[],"selected":null}')
unknown=target/'keep-me.txt';unknown.write_text('user file')
before={p:p.read_bytes() for p in [sentinel,profiles,unknown]}
assert install()==0
assert all(p.read_bytes()==value for p,value in before.items())
print('PASS repeat installation retains worlds, profile database and unknown files',flush=True)
subprocess.run([str(target/'Uninstall.exe'),'/S'],timeout=60,check=True)
for _ in range(200):
    if registered() is None and not (target/'Graphite.exe').exists():break
    time.sleep(.1)
assert registered() is None and not (target/'Graphite.exe').exists()
assert not (target/'engine/prismlauncher.exe').exists()
assert all(p.read_bytes()==value for p,value in before.items())
shortcut=Path.home()/'AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Graphite/Graphite.lnk'
assert not shortcut.exists()
print('PASS uninstall removes runtime, registration and shortcut while retaining all data',flush=True)
print('Fixture:',target)
