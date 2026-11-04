# Building Graphite on Windows x64

This snapshot contains source, not compiler toolchains or node_modules. The current release was built on the maintainer's Windows machine. These portable instructions have been checked against the files, but a second clean-machine build has not been performed.

## Rust host and UI

Install Node.js compatible with the Vite version in package-lock.json, npm, Rust stable for `x86_64-pc-windows-msvc`, and Visual Studio C++ Build Tools with the Windows SDK. WebView2 Runtime is required to run the application. Python 3.11+ is used by the packaging scripts.

From this source directory in PowerShell:

```powershell
Set-Location graphite-web
npm ci
npm test
npm run build
cargo build --release --locked --manifest-path src-tauri/Cargo.toml
```

Output: `graphite-web/src-tauri/target/release/graphite.exe`.

Use the standard cargo invocation above; the original workstation's private MSYS2/SDK layout is not required for the Rust build. `graphite-web/BUILD-RUST.ps1` also has a standard-cargo fallback. Do not copy an unrelated local build_rust.ps1 beside it.

## C++ engine

Use the included modified `PrismLauncher` source, not an unmodified official EXE: Graphite depends on its service protocol. The libnbtplusplus source is included; no submodule checkout is needed for that directory in this archive.

Install a consistent MSYS2 UCRT64 environment with clang/clang++, CMake, Ninja, pkg-config, Qt6 (Core, Widgets, Concurrent, Network, NetworkAuth, XML, OpenGL, Test), ECM, cmark, libarchive, tomlplusplus and zlib. A JDK with java, javac and jar is required. See `PrismLauncher/CMakeLists.txt` for dependency detection and upstream build documentation: https://prismlauncher.org/wiki/development/build-instructions/

Before a public custom build, replace or disable upstream API defaults in CMakeLists.txt. Do not confuse an OAuth client ID with a private client secret.

From this source directory, with paths adjusted to installed tools:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\PrismLauncher\design\BUILD.ps1 -MsysRoot C:\msys64 -JdkRoot C:\path\to\jdk -Destination C:\path\to\engine-stage
```

The script configures/builds/tests/installs the engine. It excludes the existing FileSystem test as written in that script; it does not claim every upstream test passes. Deploy Qt and runtime dependencies after installation:

```powershell
& C:\msys64\ucrt64\bin\windeployqt.exe C:\path\to\engine-stage\prismlauncher.exe
python .\PrismLauncher\design\package_runtime.py C:\path\to\engine-stage C:\msys64\ucrt64\bin
Copy-Item .\graphite-web\authlib-injector-1.2.7.jar C:\path\to\engine-stage\
```

Keep the engine's installed jars, Qt plugins, license files and other runtime resources together. The supplied `../portable/engine` is the reference runtime layout for the current binary. For Java agent source/build details, use `third-party/authlib-injector-1.2.7/README.en.md` and its Gradle wrapper.

## Rust integration tests and running

```powershell
$env:GRAPHITE_TEST_BACKEND='C:\path\to\engine-stage\prismlauncher.exe'
cargo test --locked --manifest-path .\graphite-web\src-tauri\Cargo.toml
```

Some native tests create isolated fixture data; the live Modrinth update fixture is ignored by default. For manual running, put the engine runtime in an `engine` directory beside graphite.exe, or set GRAPHITE_BACKEND to the engine EXE. GRAPHITE_DATA can point to a separate writable test directory; otherwise data is created beside the app.

To run `smoky-native-check.cjs`, build the Rust release first and set GRAPHITE_QA_ENGINE to your absolute engine EXE path (for example ../portable/engine/prismlauncher.exe after resolving the path). Close other Graphite windows before the test because the app has a single-instance guard. Without this environment variable the script falls back to its original engine-offline fixture directory. The installer-check.py is the original workstation harness: it expects sibling work/outputs directories and refuses to overwrite an already registered installation. Adapt its paths in an isolated test environment before using it.

## Packaging

From this source directory after building the host:

```powershell
python .\graphite-web\package_rust.py --engine C:\path\to\engine-stage --output ..\release --native-source .\PrismLauncher
```

This creates clean runtime/source archives from source and the specified engine, never from a user's `data`. It expects npm dependency licenses in node_modules; Rust license collection uses the original optional local registry location, so independently audit dependency notices for a clean-machine release.

For the existing installer builder, prepare an installer-tools directory containing `nsis/nsis-3.08/makensis.exe` and Microsoft's `MicrosoftEdgeWebview2Setup.exe`, then run:

```powershell
python .\graphite-web\build_installer.py --output ..\release --tools C:\path\to\installer-tools
```

Official tool sources: https://nsis.sourceforge.io/Download and https://developer.microsoft.com/en-us/microsoft-edge/webview2/

Keep this supplementary root documentation and third-party source with future source archives: package_rust.py currently archives graphite-web, PrismLauncher and its explicit QA files, not every file in this source root. Do not accidentally omit the additional notices/source when publishing a new release.
