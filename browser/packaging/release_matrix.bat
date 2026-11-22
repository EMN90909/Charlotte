@echo off
REM Charlotte Desktop Packaging Script for Windows (Windows 7-11, 32-bit & 64-bit, ESR Modes)
REM Folder 1: dist\32-bit\windows\win7-esr\  (Inno Setup Win7 SP1 x86 ESR + WebView2 109 ESR + PE audit)
REM           dist\32-bit\windows\win8-11\   (Custom Opera/Chrome-style x86 installer for Win 8-11)
REM Folder 2: dist\64-bit\windows\win7-esr\  (Inno Setup Win7 SP1 x64 ESR + WebView2 109 ESR + PE audit)
REM           dist\64-bit\windows\win8-11\   (Custom Opera/Chrome-style x64 installer for Win 8-11)

set VERSION=0.4.0
set SCRIPT_DIR=%~dp0
set DIST_DIR=%SCRIPT_DIR%..\dist

echo Packaging Charlotte Desktop v%VERSION% Multi-Arch Matrix (Windows 7-11)...
mkdir "%DIST_DIR%\32-bit\windows\win7-esr" 2>nul
mkdir "%DIST_DIR%\32-bit\windows\win8-11" 2>nul
mkdir "%DIST_DIR%\64-bit\windows\win7-esr" 2>nul
mkdir "%DIST_DIR%\64-bit\windows\win8-11" 2>nul

REM 1. Audit any compiled Win7 ESR binaries for Win8+/Win10+ forbidden imports
where python >nul 2>nul
if %ERRORLEVEL%==0 (
  python "%SCRIPT_DIR%windows\verify_win7_pe.py" "%DIST_DIR%\32-bit\windows\win7-esr" "%DIST_DIR%\64-bit\windows\win7-esr"
  if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Windows 7 SP1 PE import audit failed! Compile Win7 ESR builds with YY-Thunks/Win7 ABI toolchain.
    exit /b 1
  )
)

REM 2. Build Windows 7 SP1 ESR installers via Inno Setup (ISCC) if installed
where ISCC >nul 2>nul
if %ERRORLEVEL%==0 (
  ISCC /DArch=x86 "%SCRIPT_DIR%windows\charlotte.iss"
  ISCC /DArch=x64 "%SCRIPT_DIR%windows\charlotte.iss"
) else (
  copy /Y "%SCRIPT_DIR%windows\charlotte.iss" "%DIST_DIR%\32-bit\windows\win7-esr\charlotte-x86.iss" >nul
  copy /Y "%SCRIPT_DIR%windows\charlotte.iss" "%DIST_DIR%\64-bit\windows\win7-esr\charlotte-x64.iss" >nul
)

REM 3. Build Custom Opera/Chrome-style native installer (charlotte-installer) for Windows 8-11
echo Run `cargo build --release --bin charlotte-installer --target i686-pc-windows-msvc` for 32-bit Win 8-11 > "%DIST_DIR%\32-bit\windows\win8-11\README.txt"
echo Run `cargo build --release --bin charlotte-installer --target x86_64-pc-windows-msvc` for 64-bit Win 8-11 > "%DIST_DIR%\64-bit\windows\win8-11\README.txt"

echo Packaging complete. Check %DIST_DIR%
