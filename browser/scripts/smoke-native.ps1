# Launch the native CEF Windows bundle and check Rust UI and CSRF.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
$portFile = Join-Path $env:TEMP "charlotte-port-$PID.txt"
$profile = Join-Path $env:TEMP "charlotte-profile-$PID"
$env:CHARLOTTE_UI_PORT_FILE = $portFile
$env:CHARLOTTE_PROFILE_DIR = $profile
$process = $null
try {
  if (Test-Path $portFile) { Remove-Item $portFile -Force }
  New-Item -ItemType Directory -Path $profile -Force | Out-Null
  $app = Get-ChildItem -Path 'target/bundle' -Recurse -Filter 'Charlotte.exe' -File |
     Select-Object -First 1
  if (!$app) { throw 'CEF bundle has no Charlotte.exe' }
  $process = Start-Process -FilePath $app.FullName -PassThru
  $port = $null
  foreach ($i in 1..100) {
    Start-Sleep -Milliseconds 300
    if ($process.HasExited) { throw "CEF browser exited: $($process.ExitCode)" }
    if (Test-Path $portFile) {
      $port = (Get-Content $portFile -Raw).Trim()
      if ($port -match '^\d+$') {
        try {
          $chrome = Invoke-WebRequest "http://127.0.0.1:$port/chrome" -TimeoutSec 2
          if ($chrome.StatusCode -eq 200) { break }
        } catch { }
      }
    }
  }
  if (!$port -or $port -notmatch '^\d+$') { throw 'Native UI server did not start' }
  $chrome = Invoke-WebRequest "http://127.0.0.1:$port/chrome"
  $sidebar = Invoke-WebRequest "http://127.0.0.1:$port/sidebar"
  if ($chrome.Content -notmatch 'Charlotte' -or $sidebar.Content -notmatch 'Ask Charlotte' -or $chrome.Content -notmatch 'Toggle ad blocker') {
    throw 'CEF native UI HTML is missing'
  }
  $rendered = $false
  foreach ($i in 1..100) {
    $health = Invoke-RestMethod "http://127.0.0.1:$port/health" -TimeoutSec 2
    if ($health.cef_main_frame_loads -ge 1) { $rendered = $true; break }
    Start-Sleep -Milliseconds 300
  }
  if (!$rendered) { throw 'CEF never completed a main-frame load' }
  try {
    Invoke-WebRequest "http://127.0.0.1:$port/action" -Method POST -Body @{command='new'} | Out-Null
    throw 'CSRF request unexpectedly succeeded'
  } catch {
    if ($_.Exception.Response.StatusCode.value__ -ne 403) { throw }
  }
  Write-Host 'Native Windows CEF page render and Rust UI smoke passed.'
} finally {
  if ($process -and !$process.HasExited) {
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
  }
  Remove-Item $portFile -Force -ErrorAction SilentlyContinue
  Remove-Item $profile -Recurse -Force -ErrorAction SilentlyContinue
  Pop-Location
}
