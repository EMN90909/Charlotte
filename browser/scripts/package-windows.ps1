$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
  cargo build --release
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $bundle = Get-Command bundle-cef-app -ErrorAction SilentlyContinue
  if (-not $bundle) { cargo install cef --version 156.2.0+156.0.14 --bin bundle-cef-app --locked }
  bundle-cef-app Charlotte -o target/bundle --release
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $iscc = Get-Command iscc -ErrorAction SilentlyContinue
  if (-not $iscc) { throw 'Inno Setup 6 is required for a Windows Setup.exe. Install it, then rerun this script.' }
  & $iscc.Source packaging/windows/charlotte.iss
} finally { Pop-Location }
