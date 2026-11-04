from pathlib import Path
import argparse
import hashlib
import json
import shutil
import zipfile
import tomllib

parser=argparse.ArgumentParser()
parser.add_argument('--engine',required=True,type=Path)
parser.add_argument('--output',required=True,type=Path)
parser.add_argument('--native-source',type=Path)
args=parser.parse_args()
web=Path(__file__).resolve().parent
out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
runtime=(out/'Graphite').resolve()
assert runtime.parent==out and runtime.name=='Graphite'
engine=args.engine.resolve()
assert not engine.is_relative_to(runtime), 'Engine source must be outside the output directory'
assert (engine/'prismlauncher.exe').is_file()
if runtime.exists():
    manifest=runtime/'release.json'
    assert manifest.is_file() and json.loads(manifest.read_text())['runtime']=='rust-webview2'
    assert not (runtime/'data').exists()
    shutil.rmtree(runtime)
runtime.mkdir()
assert not list(engine.rglob('accounts.json'))
exe=web/'src-tauri/target/release/graphite.exe'
assert exe.is_file()
shutil.copy2(exe,runtime/'Graphite.exe')
shutil.copytree(engine,runtime/'engine')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(engine/'prismlauncher.exe')==sha(runtime/'engine/prismlauncher.exe')
version=json.loads((web/'package.json').read_text(encoding='utf-8-sig'))['version']
shutil.copy2(web/'RELEASE.md',runtime/'README-Graphite.md')
licenses=runtime/'licenses';licenses.mkdir()
for package in ['react','react-dom','three','@phosphor-icons/react','@fontsource-variable/ibm-plex-sans']:
    root=web/'node_modules'/package
    for pattern in ['LICENSE*','OFL*','COPYING*']:
        for file in root.glob(pattern):
            if file.is_file():shutil.copy2(file,licenses/(package.replace('/','-')+'-'+file.name))
registry=web.parent/'rust-tools/cargo/registry/src'
if registry.exists():
    packages=tomllib.loads((web/'src-tauri/Cargo.lock').read_text())['package']
    for package in packages:
        roots=list(registry.glob(f"*/{package['name']}-{package['version']}"))
        for root in roots:
            for pattern in ['LICENSE*','COPYING*','NOTICE*']:
                for file in root.glob(pattern):
                    if file.is_file():
                        destination=licenses/'rust'/root.name;destination.mkdir(parents=True,exist_ok=True)
                        shutil.copy2(file,destination/file.name)
manifest={'version':version,'runtime':'rust-webview2','electron':False,'nodeRuntime':False,'prismSha256':sha(engine/'prismlauncher.exe'),'launcherSha256':sha(runtime/'Graphite.exe'),'runtimeFiles':['Graphite.exe','engine','README-Graphite.md','PERFORMANCE.md','release.json','licenses']}
(runtime/'release.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
performance='''# Замеры Graphite 0.7.0

Исторические замеры версии 0.7.0: Rust + системный WebView2; интерфейс React/Tailwind. Для 0.8.0 замеры не повторялись. Текущий SHA-256 движка записан в release.json. Это веб-отрисовка внутри нативной оболочки.

## Сравнение оболочек

Один локальный парный прогон, одинаковый оптимизированный интерфейс и одна тестовая Vanilla-сборка, без запущенной игры. Суммировалась приватная выделенная память всего дерева процессов, включая Prism и браузерные процессы. После 8 секунд прогрева выполнено три замера каждого варианта; указана медиана. Приватная выделенная память не равна физическому потреблению RAM.

| Показатель | Electron | Rust/WebView2 |
|---|---:|---:|
| Приватная память процессов, МиБ | 371.1 | 279.6 |

В этом прогоне снижение составило около 25%. Это не универсальная гарантия: результаты зависят от WebView2, драйверов, числа сборок и текущих задач. Одного прогона недостаточно для достоверного сравнения времени старта.

## Работа интерфейса в простое

Отдельный тест одинаковой библиотеки из 200 искусственных тестовых сборок, по 12 секунд простоя. Выполнен в одном тестовом Electron-рендерере до и после оптимизации React; это сравнение интерфейса, а не разницы браузерных движков.

| Показатель | До | После |
|---|---:|---:|
| Запросы состояния за 12 секунд | 16 | 2 |
| Чтения библиотеки за 12 секунд | 3 | 0 |
| Задачи рендерера за период, мс | 45 | 2 |

Библиотека продолжает обновляться: период в простое увеличен до 15 секунд, при активности — 4 секунды, при возвращении в окно обновление происходит сразу. Прогресс активных задач проверяется каждые 750 мс. Панели подгружаются при открытии; одинаковые снимки состояния сохраняют ссылочную идентичность React.

Ускорение оболочки не означает увеличение FPS Minecraft. Длительная игровая сессия с пользовательским аккаунтом и все сочетания модов не проверены этими замерами.
'''
(runtime/'PERFORMANCE.md').write_text(performance,encoding='utf-8')
for name in ['runtime-comparison.json','performance-before.json','performance-after.json']:
    file=web.parent/name
    if file.exists():
        metrics=runtime/'verification';metrics.mkdir(exist_ok=True);shutil.copy2(file,metrics/name)
manifest['runtimeFiles'].append('verification')
(runtime/'release.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8')
assert not (runtime/'data').exists()
forbidden={'electron.exe','node.dll','resources.pak','chrome_100_percent.pak','snapshot_blob.bin','v8_context_snapshot.bin'}
assert not any(file.name.lower() in forbidden for file in runtime.rglob('*'))
archive=out/'Graphite-Rust-Windows-x64.zip'
with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED,compresslevel=6,strict_timestamps=False) as zipped:
    for file in runtime.rglob('*'):
        if file.is_file():zipped.write(file,file.relative_to(out))
shutil.copy2(archive,out/'Graphite-Workspace-Windows-x64.zip')
sources=out/'Graphite-Rust-source.zip'
with zipfile.ZipFile(sources,'w',zipfile.ZIP_DEFLATED,compresslevel=6,strict_timestamps=False) as zipped:
    for check in ['tauri-check.cjs','operations-check.cjs','operations-native-check.cjs','startup-preview.cjs','servers-check.cjs','smoky-check.cjs','smoky-native-check.cjs','porthole-check.cjs']:
        zipped.write(web.parent/check,check)
    roots=[web]+([args.native_source.resolve()] if args.native_source else [])
    for root in roots:
        for file in root.rglob('*'):
            rel=file.relative_to(root)
            if any(part in {'.git','node_modules','dist','target','data','build-graphite','__pycache__','.token-optimizer'} for part in rel.parts):continue
            if file.name.startswith('.env') or file.name in {'.npmrc','accounts.json','graphite-servers.json'} or file.suffix in {'.log','.pdb'}:continue
            if file.is_file() and file.resolve().is_relative_to(root.resolve()):zipped.write(file,Path(root.name)/rel)
shutil.copy2(sources,out/'Graphite-Workspace-source.zip')
shutil.copy2(runtime/'README-Graphite.md',out/'README.md')
print(json.dumps({'version':version,'runtime':'rust-webview2','launcherBytes':(runtime/'Graphite.exe').stat().st_size,'archiveBytes':archive.stat().st_size,'prismSha256':manifest['prismSha256']},ensure_ascii=False))
