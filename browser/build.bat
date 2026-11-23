@echo off
setlocal EnableExtensions EnableDelayedExpansion
rem =========================================================================
rem Charlotte Native Browser Build & Packaging for Windows (Windows 7–11)
rem Multi-architecture: 32-bit (x86) and 64-bit (x64), ESR modes & Evergreen
rem Outputs all executables and installer exes directly into dist\
rem Scope: browser\ only.
rem =========================================================================

cd /d "%~dp0"
if errorlevel 1 (
    echo [ERROR] Cannot enter Charlotte browser directory.
    exit /b 2
)

set "MODE=%~1"
if "%MODE%"=="" set "MODE=--all"

if /I "%MODE%"=="--help" goto usage
if /I "%MODE%"=="-h" goto usage
if /I "%MODE%"=="--check" goto check
if /I "%MODE%"=="--test" goto test
if /I "%MODE%"=="--compile-only" goto compile
if /I "%MODE%"=="--all" goto build_all
if /I "%MODE%"=="--dist" goto build_all
if /I "%MODE%"=="--verify" goto verify
goto usage_error

:preflight
where cargo >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Rust cargo is missing. Install Rust stable from https://rustup.rs/
    exit /b 2
)
exit /b 0

:check
call :preflight
if errorlevel 1 exit /b 2
echo === Running Charlotte Cargo Check ===
cargo check --all-targets
if errorlevel 1 (
    echo [ERROR] Cargo check failed.
    exit /b 1
)
echo [OK] Charlotte cargo check passed.
exit /b 0

:test
call :preflight
if errorlevel 1 exit /b 2
echo === Running Charlotte Test Suite ===
cargo test --all-targets
if errorlevel 1 (
    echo [ERROR] Cargo test failed.
    exit /b 1
)
echo [OK] Charlotte cargo tests passed.
exit /b 0

:compile
call :preflight
if errorlevel 1 exit /b 2
echo === Compiling Charlotte Release Binaries ===
cargo build --release --bins
if errorlevel 1 (
    echo [ERROR] Cargo build failed.
    exit /b 1
)
echo [OK] Release binaries compiled into target\release\
exit /b 0

:verify
call :preflight
if errorlevel 1 exit /b 2
echo === Verifying Charlotte (Check + Tests + Build) ===
cargo check --all-targets
if errorlevel 1 exit /b 1
cargo test --all-targets
if errorlevel 1 exit /b 1
goto build_all

:build_all
call :preflight
if errorlevel 1 exit /b 2

echo =========================================================================
echo Building Charlotte Native Browser Release Binaries
echo =========================================================================
cargo build --release --bins
if errorlevel 1 (
    echo [ERROR] Release compilation failed.
    exit /b 1
)

echo =========================================================================
echo Packaging Multi-Architecture Matrix into dist\
echo   - Folder 1: dist\32-bit\ (Windows 7 SP1 x86 ESR + Win 8-11 x86)
echo   - Folder 2: dist\64-bit\ (Windows 7 SP1 x64 ESR + Win 8-11 x64)
echo =========================================================================

set "DIST=%~dp0dist"
set "DIR32=%DIST%\32-bit"
set "DIR64=%DIST%\64-bit"

mkdir "%DIR32%\windows\win7-esr" 2>nul
mkdir "%DIR32%\windows\win8-11" 2>nul
mkdir "%DIR32%\linux" 2>nul
mkdir "%DIR64%\windows\win7-esr" 2>nul
mkdir "%DIR64%\windows\win8-11" 2>nul
mkdir "%DIR64%\linux" 2>nul

rem 1. Copy compiled 64-bit binaries
if exist "%~dp0target\release\Charlotte.exe" (
    echo Copying Charlotte.exe to dist\64-bit\windows\win8-11\ and win7-esr\...
    copy /Y "%~dp0target\release\Charlotte.exe" "%DIR64%\windows\win8-11\Charlotte.exe" >nul
    copy /Y "%~dp0target\release\Charlotte.exe" "%DIR64%\windows\win7-esr\Charlotte.exe" >nul
)
if exist "%~dp0target\release\charlotte-settings.exe" (
    echo Copying charlotte-settings.exe to dist\64-bit\windows\win8-11\...
    copy /Y "%~dp0target\release\charlotte-settings.exe" "%DIR64%\windows\win8-11\charlotte-settings.exe" >nul
)
if exist "%~dp0target\release\charlotte-installer.exe" (
    echo Copying charlotte-installer.exe to dist\64-bit\windows\win8-11\charlotte-installer-x64.exe...
    copy /Y "%~dp0target\release\charlotte-installer.exe" "%DIR64%\windows\win8-11\charlotte-installer-x64.exe" >nul
)

rem 2. Copy 32-bit binaries if present from 32-bit target build
if exist "%~dp0target\i686-pc-windows-msvc\release\Charlotte.exe" (
    echo Copying 32-bit Charlotte.exe to dist\32-bit\windows\...
    copy /Y "%~dp0target\i686-pc-windows-msvc\release\Charlotte.exe" "%DIR32%\windows\win8-11\Charlotte.exe" >nul
    copy /Y "%~dp0target\i686-pc-windows-msvc\release\Charlotte.exe" "%DIR32%\windows\win7-esr\Charlotte.exe" >nul
)
if exist "%~dp0target\i686-pc-windows-msvc\release\charlotte-installer.exe" (
    echo Copying 32-bit charlotte-installer.exe to dist\32-bit\windows\win8-11\charlotte-installer-x86.exe...
    copy /Y "%~dp0target\i686-pc-windows-msvc\release\charlotte-installer.exe" "%DIR32%\windows\win8-11\charlotte-installer-x86.exe" >nul
)

rem 3. Copy Inno Setup Win7 ESR scripts and PE auditor
copy /Y "%~dp0packaging\windows\charlotte.iss" "%DIR32%\windows\win7-esr\charlotte-x86.iss" >nul
copy /Y "%~dp0packaging\windows\charlotte.iss" "%DIR64%\windows\win7-esr\charlotte-x64.iss" >nul
copy /Y "%~dp0packaging\windows\verify_win7_pe.py" "%DIR32%\windows\win7-esr\verify_win7_pe.py" >nul
copy /Y "%~dp0packaging\windows\verify_win7_pe.py" "%DIR64%\windows\win7-esr\verify_win7_pe.py" >nul

rem 4. Copy channel manifests
echo {"version":"0.4.0","arch":"x64","installer":"charlotte-installer-x64.exe","cloud_server":"https://api-charlotte-server.onrender.com","channels":{"win8-esr":"Microsoft.WebView2.FixedVersionRuntime.109.0.1518.78.x64","stable":"Microsoft.WebView2.Evergreen.x64","esr":"Microsoft.WebView2.FixedVersionRuntime.ESR.x64"}} > "%DIR64%\windows\win8-11\installer_channels.json"
echo {"version":"0.4.0","arch":"x86","installer":"charlotte-installer-x86.exe","cloud_server":"https://api-charlotte-server.onrender.com","channels":{"win8-esr":"Microsoft.WebView2.FixedVersionRuntime.109.0.1518.78.x86","stable":"Microsoft.WebView2.Evergreen.x86","esr":"Microsoft.WebView2.FixedVersionRuntime.ESR.x86"}} > "%DIR32%\windows\win8-11\installer_channels.json"

rem 5. Run Inno Setup Compiler if installed on PATH
where ISCC >nul 2>nul
if %ERRORLEVEL%==0 (
    echo Building Windows 7 SP1 ESR Inno Setup Installers...
    ISCC /DArch=x64 /DAppVersion=0.4.0 /O"%DIR64%\windows\win7-esr" "%~dp0packaging\windows\charlotte.iss"
    ISCC /DArch=x86 /DAppVersion=0.4.0 /O"%DIR32%\windows\win7-esr" "%~dp0packaging\windows\charlotte.iss"
) else (
    echo [INFO] Inno Setup Compiler (ISCC) not found on PATH.
    echo        Inno Setup scripts prepared at:
    echo          %DIR64%\windows\win7-esr\charlotte-x64.iss
    echo          %DIR32%\windows\win7-esr\charlotte-x86.iss
)

rem 6. Audit Windows 7 SP1 PE imports if Python is available
where python >nul 2>nul
if %ERRORLEVEL%==0 (
    echo Running Windows 7 SP1 PE Import Audit...
    python "%~dp0packaging\windows\verify_win7_pe.py" "%DIR32%\windows\win7-esr" "%DIR64%\windows\win7-esr"
)

rem 7. Generate Checksums using PowerShell
powershell -Command "Get-ChildItem -Path '%DIST%' -Recurse -File | Where-Object { $_.Name -ne 'SHA256SUMS.txt' } | Get-FileHash -Algorithm SHA256 | ForEach-Object { '{0}  {1}' -f $_.Hash.ToLower(), $_.Path.Substring('%DIST%'.Length + 1) } | Out-File -Encoding utf8 '%DIST%\SHA256SUMS.txt'" 2>nul

echo =========================================================================
echo Charlotte Windows Build and Packaging Complete!
echo Outputs generated in:
echo   - 64-bit Windows: %DIR64%\windows\
echo   - 32-bit Windows: %DIR32%\windows\
echo   - Checksums:      %DIST%\SHA256SUMS.txt
echo =========================================================================
exit /b 0

:usage_error
echo [ERROR] Unknown option: %MODE%
:usage
echo Charlotte Native Browser Build for Windows (Windows 7–11)
echo   build.bat                 Build release binaries and package complete dist\ (Default)
echo   build.bat --all           Same as above: build and package all executables to dist\
echo   build.bat --dist          Same as --all
echo   build.bat --compile-only  Cargo release binaries only (without packaging dist\)
echo   build.bat --check         Cargo check all targets
echo   build.bat --test          Cargo test all targets
echo   build.bat --help          Print this help
exit /b 0
