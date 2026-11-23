# Verify the Windows installable bundle includes the browser, independent
# updater, native resources and original branding assets.
[CmdletBinding()]
param([string]$DistRoot = (Join-Path $PSScriptRoot '..\dist'))
$ErrorActionPreference = 'Stop'
$dist = (Resolve-Path -LiteralPath $DistRoot).Path
$bundle = Join-Path $dist '64-bit\windows\win8-11'
$expected = @(
    'Charlotte.exe',
    'charlotte-updater.exe',
    'charlotte-settings.exe',
    'charlotte-installer-x64.exe',
    'installer_channels.json',
    'assets\icon.ico',
    'assets\icon.png',
    'assets\icon-256.png',
    'resources\windows\app.manifest',
    'resources\windows\charlotte.rc'
)
foreach ($relative in $expected) {
    $path = Join-Path $bundle $relative
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing required Windows bundle resource: $relative"
    }
    if ((Get-Item -LiteralPath $path).Length -le 0) {
        throw "Empty Windows bundle resource: $relative"
    }
}
$checksumPath = Join-Path $dist 'SHA256SUMS.txt'
if (!(Test-Path -LiteralPath $checksumPath)) {
    throw 'Missing complete bundle SHA256SUMS.txt'
}
$count = 0
foreach ($line in (Get-Content -LiteralPath $checksumPath)) {
    if ($line -notmatch '^([a-fA-F0-9]{64})  (.+)$') { continue }
    $expect = $Matches[1].ToLowerInvariant()
    $relative = $Matches[2].Replace('/', '\')
    if ($relative.Contains('..')) { throw "Invalid checksum path: $relative" }
    $path = Join-Path $dist $relative
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Checksum-listed file is missing: $relative"
    }
    $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expect) { throw "Checksum mismatch for $relative" }
    $count++
}
if ($count -lt $expected.Count) { throw "Too few checksummed files: $count" }
Write-Host "[PASS] Windows x64 bundle resources and $count SHA-256 checksums verified."
