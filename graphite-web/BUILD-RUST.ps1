param([switch]$Run,[switch]$Test,[string]$Backend='')
$ErrorActionPreference='Stop'
if($Backend){$env:GRAPHITE_BACKEND=(Resolve-Path -LiteralPath $Backend).Path;$env:GRAPHITE_TEST_BACKEND=$env:GRAPHITE_BACKEND}
$taskLocalHelper=Join-Path (Split-Path $PSScriptRoot) 'build_rust.ps1'
Push-Location $PSScriptRoot
try{
 npm run build
 if($LASTEXITCODE){exit $LASTEXITCODE}
 if(Test-Path -LiteralPath $taskLocalHelper){& $taskLocalHelper -Release:(!$Run) -Test:$Test}
 else{if($Test){cargo test --manifest-path src-tauri/Cargo.toml}else{cargo build --release --manifest-path src-tauri/Cargo.toml}}
 if($LASTEXITCODE){exit $LASTEXITCODE}
 if($Run){& src-tauri/target/debug/graphite.exe}
}finally{Pop-Location}
