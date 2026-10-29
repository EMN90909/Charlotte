@echo off
setlocal EnableExtensions
rem Charlotte native Rust + CEF browser build. Scope: browser\ only.
cd /d "%~dp0"
if errorlevel 1 (echo Cannot enter Charlotte browser directory. & exit /b 2)

set "MODE=%~1"
if "%MODE%"=="" set "MODE=--bundle"
if not "%~2"=="" goto usage_error
if /I "%MODE%"=="--help" goto usage
if /I "%MODE%"=="-h" goto usage
if /I "%MODE%"=="--check" goto check
if /I "%MODE%"=="--test" goto test
if /I "%MODE%"=="--compile-only" goto compile
if /I "%MODE%"=="--bundle" goto bundle
if /I "%MODE%"=="--verify" goto verify
goto usage_error

:preflight
where cargo >nul 2>nul
if errorlevel 1 (
    echo Rust cargo is missing. Install Rust stable from https://rustup.rs/
    exit /b 2
)
exit /b 0

:preflight_bundle
call :preflight
if errorlevel 1 exit /b 2
if not defined CEF_PATH (
    echo Set CEF_PATH to a matching, extracted CEF distribution.
    echo An unbundled Rust executable cannot launch Chromium.
    exit /b 2
)
if not exist "%CEF_PATH%\" (
    echo CEF_PATH is not a directory: "%CEF_PATH%"
    exit /b 2
)
where bundle-cef-app >nul 2>nul
if errorlevel 1 (
    echo Missing bundle-cef-app from the CEF Rust build utilities.
    echo Install the version matching cef 156.2.0+156.0.14.
    exit /b 2
)
exit /b 0

:check
call :preflight
if errorlevel 1 exit /b 2
cargo check --all-targets
if errorlevel 1 exit /b 1
echo Charlotte cargo check passed.
exit /b 0

:test
call :preflight
if errorlevel 1 exit /b 2
cargo test --all-targets
if errorlevel 1 exit /b 1
echo Charlotte cargo tests passed.
exit /b 0

:compile
call :preflight
if errorlevel 1 exit /b 2
cargo build --release --bins
if errorlevel 1 exit /b 1
echo Rust binaries in browser\target\release - NOT a runnable CEF bundle.
exit /b 0

:bundle
call :preflight_bundle
if errorlevel 1 exit /b 2
cargo build --release --bins
if errorlevel 1 exit /b 1
call scripts\build.bat
if errorlevel 1 exit /b 1
echo Charlotte CEF bundle output: browser\target\bundle
exit /b 0

:verify
call :preflight_bundle
if errorlevel 1 exit /b 2
cargo check --all-targets
if errorlevel 1 exit /b 1
cargo test --all-targets
if errorlevel 1 exit /b 1
cargo build --release --bins
if errorlevel 1 exit /b 1
call scripts\build.bat
if errorlevel 1 exit /b 1
echo Charlotte verification and CEF bundle completed: browser\target\bundle
exit /b 0

:usage_error
echo Unknown build option or too many arguments: %*
:usage
echo Charlotte native browser build (Windows)
echo   build.bat                 Build release binaries and runnable CEF bundle
echo   build.bat --bundle        Same as above
echo   build.bat --verify        Cargo check, tests, release build and bundle
echo   build.bat --compile-only  Rust binaries only, without CEF bundle
echo   build.bat --check         Cargo check all targets
echo   build.bat --test          Cargo tests
echo   build.bat --help          Show this help
echo.
echo Requirements: Rust stable + MSVC build tools, matching CEF distribution,
echo CEF_PATH, and bundle-cef-app. No Node.js or website build is required.
if /I "%MODE%"=="--help" exit /b 0
if /I "%MODE%"=="-h" exit /b 0
exit /b 2
