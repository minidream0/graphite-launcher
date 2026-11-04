"""Copy transitive native DLL imports after cmake --install and windeployqt."""
import re
import shutil
import subprocess
import sys
from pathlib import Path


def imports(binary, objdump):
    result = subprocess.run([str(objdump), '-p', str(binary)], capture_output=True, text=True, check=True)
    return re.findall(r'DLL Name:\s*(\S+)', result.stdout)


def main():
    destination, runtime = (Path(value).resolve() for value in sys.argv[1:])
    objdump = runtime / 'objdump.exe'
    assert destination.is_dir() and objdump.is_file()
    candidates = {p.name.lower(): p for p in runtime.glob('*.dll')}
    pending = list(destination.rglob('*.exe')) + list(destination.rglob('*.dll'))
    visited = set()
    copied = []
    while pending:
        binary = pending.pop()
        if binary in visited:
            continue
        visited.add(binary)
        for name in imports(binary, objdump):
            source = candidates.get(name.lower())
            if source is None:
                continue
            target = destination / source.name
            if not target.exists():
                shutil.copy2(source, target)
                copied.append(source.name)
            pending.append(target)
    # All MSYS2-provided imports must resolve locally; system DLLs remain in Windows.
    for binary in visited:
        for name in imports(binary, objdump):
            if name.lower() in candidates:
                assert (destination / candidates[name.lower()].name).is_file(), (binary, name)
    print(f'Runtime closure checked: {len(visited)} binaries, {len(copied)} DLLs copied.')


if __name__ == '__main__':
    main()
