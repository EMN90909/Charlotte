$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
  & "$PSScriptRoot/build.bat"
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $iscc = Get-Command iscc -ErrorAction SilentlyContinue
  if (-not $iscc) { throw 'Inno Setup 6 is required for a Windows Setup.exe. Install it, then rerun this script.' }
  & $iscc.Source packaging/windows/charlotte.iss
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally { Pop-Location }
