@echo off
setlocal EnableExtensions
rem =========================================================================
rem Charlotte Native Browser - Windows Automated Dependency Installer,
rem Builder, Multi-Arch Packager (dist\), and Launcher (Windows 7-11)
rem Scope: browser\ only.
rem Usage:
rem   build.bat                 Check/download deps, build, package dist\, start Charlotte
rem   build.bat --all           Same as default
rem   build.bat --dist          Same as default (without auto-starting)
rem   build.bat --no-start      Build and package dist\ without launching Charlotte
rem   build.bat --compile-only  Check/download deps and compile release binaries only
rem   build.bat --check         Check/download deps and run cargo check
rem   build.bat --test          Check/download deps and run cargo test
rem   build.bat --help          Show usage help
rem =========================================================================

set "ROOT=%~dp0"
if "%ROOT:~-1%"=="\" set "ROOT=%ROOT:~0,-1%"
cd /d "%ROOT%"
if errorlevel 1 (
    call :fail "Cannot enter Charlotte browser directory: %ROOT%"
    exit /b 2
)

set "MODE=%~1"
if "%MODE%"=="" set "MODE=--all"
set "AUTO_START=1"
set "NO_PAUSE=0"
if /I "%~2"=="--no-pause" set "NO_PAUSE=1"
if /I "%~1"=="--no-start" set "AUTO_START=0"
if /I "%~1"=="--dist" set "AUTO_START=0"
if defined CI set "NO_PAUSE=1"

if /I "%MODE%"=="--help" goto :usage
if /I "%MODE%"=="-h" goto :usage
if /I "%MODE%"=="--check" goto :run_check
if /I "%MODE%"=="--test" goto :run_test
if /I "%MODE%"=="--compile-only" goto :run_compile
if /I "%MODE%"=="--all" goto :run_all
if /I "%MODE%"=="--dist" goto :run_all
if /I "%MODE%"=="--no-start" goto :run_all
if /I "%MODE%"=="--verify" goto :run_verify
goto :usage_error

rem =========================================================================
rem STEP 1: Verify & Automatically Download All Required Dependencies
rem =========================================================================
:ensure_deps
echo.
echo =========================================================================
echo [1/5] Checking and downloading required build dependencies...
echo =========================================================================

call :ensure_rust
if errorlevel 1 exit /b 1

call :ensure_msvc
if errorlevel 1 exit /b 1

call :ensure_webview2
if errorlevel 1 exit /b 1

call :ensure_crates
if errorlevel 1 exit /b 1

exit /b 0

rem --- 1A. Rust & Cargo (auto-installs rustup + stable >= 1.85 for edition 2024) ---
:ensure_rust
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

where cargo >nul 2>nul
if errorlevel 1 goto :install_rust
goto :verify_rust_version

:install_rust
echo [DEPS] Rust / Cargo not found. Downloading rustup-init.exe...
set "RUSTUP_EXE=%TEMP%\charlotte-rustup-init.exe"
call :download_file "https://win.rustup.rs/x86_64" "%RUSTUP_EXE%"
if errorlevel 1 (
    call :fail "Failed to download Rust installer from https://win.rustup.rs/x86_64. Check internet connection."
    exit /b 1
)
echo [DEPS] Installing Rust stable toolchain silently...
"%RUSTUP_EXE%" -y --default-toolchain stable --profile default
if errorlevel 1 (
    call :fail "rustup-init.exe failed to install Rust toolchain."
    exit /b 1
)
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo >nul 2>nul
if errorlevel 1 (
    call :fail "Cargo still not found on PATH after installing rustup. Please restart terminal and re-run build.bat."
    exit /b 1
)

:verify_rust_version
for /f "tokens=2 delims= " %%V in ('rustc --version 2^>nul') do set "RUSTC_VER=%%V"
echo [OK] Rust detected: %RUSTC_VER%

rem Edition 2024 requires Rust 1.85+. Auto-update if older.
powershell -NoProfile -ExecutionPolicy Bypass -Command "if (-not $env:RUSTC_VER) { exit 1 }; $v = [version]($env:RUSTC_VER.Split('-')[0]); if ($v -lt [version]'1.85.0') { exit 1 } else { exit 0 }" >nul 2>nul
if errorlevel 1 (
    echo [DEPS] Rust %RUSTC_VER% is older than 1.85.0 required for edition 2024. Updating via rustup...
    rustup update stable
    if errorlevel 1 (
        call :fail "rustup update stable failed. Rust 1.85.0+ is required."
        exit /b 1
    )
)

where rustup >nul 2>nul
if not errorlevel 1 (
    echo [DEPS] Ensuring Windows 64-bit and 32-bit Rust targets are installed...
    rustup target add x86_64-pc-windows-msvc >nul 2>nul
    rustup target add i686-pc-windows-msvc >nul 2>nul
)
exit /b 0

rem --- 1B. Visual Studio C++ Build Tools (MSVC linker + Windows SDK) ---
:ensure_msvc
where cl.exe >nul 2>nul
if not errorlevel 1 (
    echo [OK] MSVC C/C++ compiler detected on PATH.
    exit /b 0
)

call :init_vcvars
if not errorlevel 1 (
    echo [OK] Visual Studio C++ Build Tools initialized.
    exit /b 0
)

echo [DEPS] Visual Studio C++ Build Tools not found. Downloading installer...
set "VS_EXE=%TEMP%\charlotte-vs_BuildTools.exe"
call :download_file "https://aka.ms/vs/17/release/vs_BuildTools.exe" "%VS_EXE%"
if errorlevel 1 (
    call :fail "Failed to download Visual Studio C++ Build Tools from https://aka.ms/vs/17/release/vs_BuildTools.exe."
    exit /b 1
)
echo [DEPS] Installing Visual Studio C++ Build Tools (VCTools workload). This may take several minutes...
"%VS_EXE%" --quiet --wait --norestart --nocache --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended
if errorlevel 1 (
    call :fail "Visual Studio C++ Build Tools installation failed or was cancelled. Install 'Desktop development with C++' and re-run build.bat."
    exit /b 1
)

call :init_vcvars
if errorlevel 1 (
    call :fail "MSVC cl.exe/link.exe still unavailable after Build Tools installation. Please reboot or open a new command prompt and re-run build.bat."
    exit /b 1
)
echo [OK] Visual Studio C++ Build Tools ready.
exit /b 0

:init_vcvars
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" exit /b 1
set "VSINSTALL="
for /f "usebackq tokens=*" %%I in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2^>nul`) do (
    set "VSINSTALL=%%I"
)
if "%VSINSTALL%"=="" exit /b 1
if exist "%VSINSTALL%\VC\Auxiliary\Build\vcvars64.bat" (
    call "%VSINSTALL%\VC\Auxiliary\Build\vcvars64.bat" >nul 2>nul
    where cl.exe >nul 2>nul
    if not errorlevel 1 exit /b 0
)
if exist "%VSINSTALL%\Common7\Tools\VsDevCmd.bat" (
    call "%VSINSTALL%\Common7\Tools\VsDevCmd.bat" -arch=amd64 -host_arch=amd64 >nul 2>nul
    where cl.exe >nul 2>nul
    if not errorlevel 1 exit /b 0
)
exit /b 1

rem --- 1C. Microsoft Edge WebView2 Runtime ---
:ensure_webview2
reg query "HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" >nul 2>nul
if not errorlevel 1 (
    echo [OK] Microsoft Edge WebView2 Runtime detected.
    exit /b 0
)
reg query "HKCU\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" >nul 2>nul
if not errorlevel 1 (
    echo [OK] Microsoft Edge WebView2 Runtime detected for current user.
    exit /b 0
)
echo [DEPS] WebView2 Runtime not found. Downloading Evergreen Bootstrapper...
set "WV2_EXE=%TEMP%\charlotte-WebView2Setup.exe"
call :download_file "https://go.microsoft.com/fwlink/p/?LinkId=2124703" "%WV2_EXE%"
if errorlevel 1 (
    echo [WARN] Could not download WebView2 bootstrapper automatically. Continuing build...
    exit /b 0
)
echo [DEPS] Installing Microsoft Edge WebView2 Runtime silently...
"%WV2_EXE%" /silent /install
if errorlevel 1 (
    echo [WARN] WebView2 silent install returned non-zero exit code. Continuing build...
) else (
    echo [OK] WebView2 Runtime installed.
)
exit /b 0

rem --- 1D. Fetch crates.io dependencies upfront ---
:ensure_crates
echo [DEPS] Fetching crates.io dependencies via cargo fetch...
cargo fetch
if errorlevel 1 (
    call :fail "cargo fetch failed. Verify internet connectivity to crates.io and check Cargo.toml."
    exit /b 1
)
echo [OK] All crates.io dependencies downloaded and verified.
exit /b 0

rem --- Helper: Download URL to destination using curl.exe or PowerShell ---
:download_file
set "DL_URL=%~1"
set "DL_OUT=%~2"
where curl.exe >nul 2>nul
if not errorlevel 1 (
    curl.exe -fsSL -o "%DL_OUT%" "%DL_URL%"
    if not errorlevel 1 exit /b 0
)
powershell -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -UseBasicParsing -Uri $env:DL_URL -OutFile $env:DL_OUT"
if errorlevel 1 exit /b 1
exit /b 0

rem =========================================================================
rem BUILD MODES
rem =========================================================================
:run_check
call :ensure_deps
if errorlevel 1 exit /b 1
echo.
echo === Running Charlotte Cargo Check ===
cargo check --all-targets
if errorlevel 1 (
    call :fail "cargo check --all-targets failed."
    exit /b 1
)
echo [OK] Charlotte cargo check passed.
exit /b 0

:run_test
call :ensure_deps
if errorlevel 1 exit /b 1
echo.
echo === Running Charlotte Test Suite ===
cargo test --lib
if errorlevel 1 (
    call :fail "cargo test --lib failed."
    exit /b 1
)
echo [OK] Charlotte test suite passed.
exit /b 0

:run_compile
call :ensure_deps
if errorlevel 1 exit /b 1
echo.
echo === Compiling Charlotte Release Binaries ===
cargo build --release --bins
if errorlevel 1 (
    call :fail "cargo build --release --bins failed."
    exit /b 1
)
echo [OK] Release binaries compiled into target\release\
exit /b 0

:run_verify
call :ensure_deps
if errorlevel 1 exit /b 1
echo.
echo === Running Verification (Check + Tests) ===
cargo check --all-targets
if errorlevel 1 (
    call :fail "cargo check failed during --verify."
    exit /b 1
)
cargo test --lib
if errorlevel 1 (
    call :fail "cargo test failed during --verify."
    exit /b 1
)
goto :build_and_package

:run_all
call :ensure_deps
if errorlevel 1 exit /b 1

:build_and_package
echo.
echo =========================================================================
echo [2/5] Compiling Charlotte 64-bit Release Executables...
echo =========================================================================
cargo build --release --bins
if errorlevel 1 (
    call :fail "64-bit release compilation failed. Review the Rust compiler error above."
    exit /b 1
)

echo.
echo =========================================================================
echo [3/5] Compiling Optional 32-bit Release Executables (i686)...
echo =========================================================================
cargo build --release --bins --target i686-pc-windows-msvc
if errorlevel 1 (
    echo [WARN] 32-bit target i686-pc-windows-msvc build skipped or failed; 64-bit binaries succeeded.
) else (
    echo [OK] 32-bit release binaries compiled successfully.
)

echo.
echo =========================================================================
echo [4/5] Packaging Multi-Architecture Matrix into dist\
echo =========================================================================
call :package_dist
if errorlevel 1 exit /b 1

echo.
echo =========================================================================
echo [5/5] Build Complete!
echo   64-bit Windows: %ROOT%\dist\64-bit\windows\
echo   32-bit Windows: %ROOT%\dist\32-bit\windows\
echo   Checksums:      %ROOT%\dist\SHA256SUMS.txt
echo =========================================================================

if "%AUTO_START%"=="1" goto :launch_browser
exit /b 0

:package_dist
set "DIST=%ROOT%\dist"
set "DIR32=%DIST%\32-bit"
set "DIR64=%DIST%\64-bit"

if not exist "%DIR32%\windows\win7-esr" mkdir "%DIR32%\windows\win7-esr"
if not exist "%DIR32%\windows\win8-11" mkdir "%DIR32%\windows\win8-11"
if not exist "%DIR32%\linux" mkdir "%DIR32%\linux"
if not exist "%DIR64%\windows\win7-esr" mkdir "%DIR64%\windows\win7-esr"
if not exist "%DIR64%\windows\win8-11" mkdir "%DIR64%\windows\win8-11"
if not exist "%DIR64%\linux" mkdir "%DIR64%\linux"

if not exist "%ROOT%\target\release\Charlotte.exe" (
    call :fail "Expected compiled binary target\release\Charlotte.exe was not found."
    exit /b 1
)

echo [PACK] Staging 64-bit executables into dist\64-bit\windows\...
copy /Y "%ROOT%\target\release\Charlotte.exe" "%DIR64%\windows\win8-11\Charlotte.exe" >nul
if errorlevel 1 (
    call :fail "Failed copying Charlotte.exe to dist\64-bit\windows\win8-11\"
    exit /b 1
)
rem NEVER relabel modern MSVC binaries as Win7 ESR without the required ABI audit.
if exist "%DIR64%\windows\win7-esr\Charlotte.exe" del /Q "%DIR64%\windows\win7-esr\Charlotte.exe"
if exist "%ROOT%\target\release\charlotte-settings.exe" (
    copy /Y "%ROOT%\target\release\charlotte-settings.exe" "%DIR64%\windows\win8-11\charlotte-settings.exe" >nul
)
if exist "%ROOT%\target\release\charlotte-installer.exe" (
    copy /Y "%ROOT%\target\release\charlotte-installer.exe" "%DIR64%\windows\win8-11\charlotte-installer-x64.exe" >nul
)

rem The updater and all runtime resources must travel with Charlotte.exe.
if not exist "%ROOT%\target\release\charlotte-updater.exe" (
    call :fail "Missing target\release\charlotte-updater.exe (build all bins)."
    exit /b 1
)
copy /Y "%ROOT%\target\release\charlotte-updater.exe" "%DIR64%\windows\win8-11\charlotte-updater.exe" >nul
if errorlevel 1 exit /b 1
xcopy "%ROOT%\assets" "%DIR64%\windows\win8-11\assets" /E /I /Y /Q >nul
if errorlevel 1 exit /b 1
xcopy "%ROOT%\resources" "%DIR64%\windows\win8-11\resources" /E /I /Y /Q >nul
if errorlevel 1 exit /b 1
if not exist "%DIR64%\windows\win8-11\assets\icon.ico" exit /b 1
if not exist "%DIR64%\windows\win8-11\resources\windows\app.manifest" exit /b 1

if exist "%ROOT%\target\i686-pc-windows-msvc\release\Charlotte.exe" (
    echo [PACK] Staging 32-bit executables into dist\32-bit\windows\...
    copy /Y "%ROOT%\target\i686-pc-windows-msvc\release\Charlotte.exe" "%DIR32%\windows\win8-11\Charlotte.exe" >nul
    if exist "%DIR32%\windows\win7-esr\Charlotte.exe" del /Q "%DIR32%\windows\win7-esr\Charlotte.exe"
)
if exist "%ROOT%\target\i686-pc-windows-msvc\release\charlotte-settings.exe" (
    copy /Y "%ROOT%\target\i686-pc-windows-msvc\release\charlotte-settings.exe" "%DIR32%\windows\win8-11\charlotte-settings.exe" >nul
)
if exist "%ROOT%\target\i686-pc-windows-msvc\release\charlotte-installer.exe" (
    copy /Y "%ROOT%\target\i686-pc-windows-msvc\release\charlotte-installer.exe" "%DIR32%\windows\win8-11\charlotte-installer-x86.exe" >nul
)
if exist "%ROOT%\target\i686-pc-windows-msvc\release\charlotte-updater.exe" (
    copy /Y "%ROOT%\target\i686-pc-windows-msvc\release\charlotte-updater.exe" "%DIR32%\windows\win8-11\charlotte-updater.exe" >nul
    xcopy "%ROOT%\assets" "%DIR32%\windows\win8-11\assets" /E /I /Y /Q >nul
    xcopy "%ROOT%\resources" "%DIR32%\windows\win8-11\resources" /E /I /Y /Q >nul
)

copy /Y "%ROOT%\packaging\windows\charlotte.iss" "%DIR32%\windows\win7-esr\charlotte-x86.iss" >nul
copy /Y "%ROOT%\packaging\windows\charlotte.iss" "%DIR64%\windows\win7-esr\charlotte-x64.iss" >nul
copy /Y "%ROOT%\packaging\windows\verify_win7_pe.py" "%DIR32%\windows\win7-esr\verify_win7_pe.py" >nul
copy /Y "%ROOT%\packaging\windows\verify_win7_pe.py" "%DIR64%\windows\win7-esr\verify_win7_pe.py" >nul

rem Write channel manifests via PowerShell to avoid cmd.exe quoting issues
powershell -NoProfile -ExecutionPolicy Bypass -Command "$m64 = @{version='0.4.0';arch='x64';installer='charlotte-installer-x64.exe';cloud_server='https://api-charlotte-server.onrender.com';channels=@{'win8-esr'='Microsoft.WebView2.FixedVersionRuntime.109.0.1518.78.x64';stable='Microsoft.WebView2.Evergreen.x64';esr='Microsoft.WebView2.FixedVersionRuntime.ESR.x64'}}; $m64 | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 (Join-Path $env:DIR64 'windows\win8-11\installer_channels.json'); $m32 = @{version='0.4.0';arch='x86';installer='charlotte-installer-x86.exe';cloud_server='https://api-charlotte-server.onrender.com';channels=@{'win8-esr'='Microsoft.WebView2.FixedVersionRuntime.109.0.1518.78.x86';stable='Microsoft.WebView2.Evergreen.x86';esr='Microsoft.WebView2.FixedVersionRuntime.ESR.x86'}}; $m32 | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 (Join-Path $env:DIR32 'windows\win8-11\installer_channels.json')"

rem Build Inno Setup installers if ISCC is installed (passing explicit BundleDir)
call :find_iscc
if defined ISCC_EXE (
  if exist "%DIR64%\windows\win7-esr\Charlotte.exe" (
    echo [PACK] Building Windows 7 SP1 ESR Inno Setup installer x64...
    "%ISCC_EXE%" /DArch=x64 /DBundleDir="%DIR64%\windows\win7-esr" /O"%DIR64%\windows\win7-esr" "%ROOT%\packaging\windows\charlotte.iss"
    if errorlevel 1 echo [WARN] Inno Setup x64 build returned an error.
    if exist "%DIR32%\windows\win7-esr\Charlotte.exe" (
        echo [PACK] Building Windows 7 SP1 ESR Inno Setup installer x86...
        "%ISCC_EXE%" /DArch=x86 /DBundleDir="%DIR32%\windows\win7-esr" /O"%DIR32%\windows\win7-esr" "%ROOT%\packaging\windows\charlotte.iss"
        if errorlevel 1 echo [WARN] Inno Setup x86 build returned an error.
    )
  ) else (
    echo [INFO] Win7 ESR installer skipped: no independently audited Win7-compatible binary supplied.
  )
) else (
    echo [INFO] Inno Setup Compiler ISCC.exe not found. Scripts staged in dist\*\windows\win7-esr\.
)

rem Run Win7 PE import check as informational audit if python is available
where python >nul 2>nul
if not errorlevel 1 (
    echo [PACK] Running Windows 7 SP1 PE import audit...
    python "%ROOT%\packaging\windows\verify_win7_pe.py" "%DIR32%\windows\win7-esr" "%DIR64%\windows\win7-esr"
    if errorlevel 1 echo [WARN] Modern MSVC binary contains Win8+ APIs. Use Rust 1.75 Win7 or YY-Thunks for final Win7 SP1 release.
)

rem Generate SHA-256 checksums for dist\
powershell -NoProfile -ExecutionPolicy Bypass -Command "$root = $env:DIST; Get-ChildItem -Path $root -Recurse -File | Where-Object { $_.Name -ne 'SHA256SUMS.txt' } | Sort-Object FullName | ForEach-Object { $h = (Get-FileHash -Algorithm SHA256 -Path $_.FullName).Hash.ToLower(); $rel = $_.FullName.Substring($root.Length + 1).Replace('\','/'); '{0}  {1}' -f $h, $rel } | Set-Content -Encoding UTF8 (Join-Path $root 'SHA256SUMS.txt')"
if errorlevel 1 echo [WARN] Could not generate SHA256SUMS.txt.

exit /b 0

:find_iscc
set "ISCC_EXE="
for /f "delims=" %%P in ('where ISCC.exe 2^>nul') do set "ISCC_EXE=%%P"
if defined ISCC_EXE exit /b 0
set "PF86=%ProgramFiles(x86)%"
if exist "%PF86%\Inno Setup 6\ISCC.exe" (
    set "ISCC_EXE=%PF86%\Inno Setup 6\ISCC.exe"
    exit /b 0
)
if exist "%ProgramFiles%\Inno Setup 6\ISCC.exe" (
    set "ISCC_EXE=%ProgramFiles%\Inno Setup 6\ISCC.exe"
    exit /b 0
)
if exist "%LocalAppData%\Programs\Inno Setup 6\ISCC.exe" (
    set "ISCC_EXE=%LocalAppData%\Programs\Inno Setup 6\ISCC.exe"
    exit /b 0
)
exit /b 0

:launch_browser
set "APP_EXE=%ROOT%\dist\64-bit\windows\win8-11\Charlotte.exe"
if not exist "%APP_EXE%" (
    call :fail "Cannot start Charlotte: %APP_EXE% not found."
    exit /b 1
)
echo.
echo [START] Launching Charlotte Browser: %APP_EXE%
start "Charlotte Browser" "%APP_EXE%"
if errorlevel 1 (
    call :fail "Failed to launch %APP_EXE%."
    exit /b 1
)
echo [OK] Charlotte Browser started!
exit /b 0

:fail
echo.
echo =========================================================================
echo [BUILD ERROR] %~1
echo =========================================================================
echo [%DATE% %TIME%] BUILD ERROR: %~1>> "%ROOT%\build-error.log"
echo Full error recorded in: %ROOT%\build-error.log
if "%NO_PAUSE%"=="0" pause
exit /b 1

:usage_error
echo [ERROR] Unknown build option: %MODE%
:usage
echo Charlotte Native Browser Build for Windows (Windows 7-11)
echo   build.bat                 Check/download deps, compile, package dist\, and start Charlotte
echo   build.bat --all           Same as above
echo   build.bat --dist          Build and package dist\ without auto-starting
echo   build.bat --no-start      Same as --dist
echo   build.bat --compile-only  Check/download deps and compile release binaries only
echo   build.bat --check         Check/download deps and run cargo check
echo   build.bat --test          Check/download deps and run cargo test
echo   build.bat --help          Print this help
exit /b 0
