param(
    [Parameter(Mandatory=$true)][string]$MsysRoot,
    [Parameter(Mandatory=$true)][string]$JdkRoot,
    [string]$Destination = "$PSScriptRoot/../../../outputs/Prism-Graphite"
)
$ErrorActionPreference = 'Stop'
$source = Split-Path $PSScriptRoot
$build = Join-Path $source 'build-graphite'
$env:PATH = "$MsysRoot/ucrt64/bin;$JdkRoot/bin;$env:PATH"
$env:JAVA_HOME = $JdkRoot
& cmake -S $source -B $build -G Ninja -DCMAKE_BUILD_TYPE=Release "-DCMAKE_INSTALL_PREFIX=$Destination" -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ '-DCMAKE_CXX_FLAGS=-Wno-error=macro-redefined' -DENABLE_LTO=OFF -DBUILD_TESTING=ON -DLauncher_BUILD_PLATFORM=graphite-local -DLauncher_BUILD_ARTIFACT= -DLauncher_UPDATER_GITHUB_REPO= -DLauncher_CURSEFORGE_API_KEY:STRING= -DLauncher_ENABLE_JAVA_DOWNLOADER=ON "-DJava_JAVA_EXECUTABLE=$JdkRoot/bin/java.exe" "-DJava_JAVAC_EXECUTABLE=$JdkRoot/bin/javac.exe" "-DJava_JAR_EXECUTABLE=$JdkRoot/bin/jar.exe"
if ($LASTEXITCODE) { throw 'CMake configure failed' }
& cmake --build $build --parallel 6
if ($LASTEXITCODE) { throw 'Build failed' }
& ctest --test-dir $build -E '^FileSystem$' --output-on-failure
if ($LASTEXITCODE) { throw 'Tests failed' }
& cmake --install $build
if ($LASTEXITCODE) { throw 'Install failed' }
Write-Host "Native binaries installed to $Destination. Deploy Qt and runtime DLLs before distribution."
