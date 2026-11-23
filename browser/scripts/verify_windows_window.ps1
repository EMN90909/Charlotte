# Verify the RELEASE executable actually creates a visible Win32 window.
# Run on Windows after build.bat --dist:
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify_windows_window.ps1
[CmdletBinding()]
param(
    [string]$ExePath = (Join-Path $PSScriptRoot '..\dist\64-bit\windows\win8-11\Charlotte.exe'),
    [int]$TimeoutSeconds = 30
)
$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path -LiteralPath $ExePath -ErrorAction Stop).Path
$dir = Split-Path -Parent $exe
Write-Host "[CHECK] Launching $exe"
$proc = Start-Process -FilePath $exe -WorkingDirectory $dir -PassThru
$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
$visible = $false
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 300
    $proc.Refresh()
    if ($proc.HasExited) {
        throw "Charlotte exited with code $($proc.ExitCode) without showing a window. Check startup error dialog or Windows Event Log."
    }
    if ($proc.MainWindowHandle -ne 0) {
        $visible = $true
        break
    }
}
if (-not $visible) {
    Write-Warning "Charlotte is alive but no top-level GUI window appeared for PID $($proc.Id)."
    throw 'Native Windows window smoke test FAILED.'
}
Write-Host "[PASS] Charlotte created a Windows window. PID=$($proc.Id) HWND=$($proc.MainWindowHandle)"
Write-Host "[INFO] The test leaves Charlotte open; close it with the window Close button."
