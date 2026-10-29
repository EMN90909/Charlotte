@echo off
setlocal EnableExtensions
cd /d "%~dp0\.."
if not "%~1"=="" (cargo build --release --target %~1) else (cargo build --release)
if errorlevel 1 exit /b %errorlevel%
where bundle-cef-app >nul 2>nul
if errorlevel 1 cargo install cef --version 156.2.0+156.0.14 --bin bundle-cef-app --locked
if errorlevel 1 exit /b %errorlevel%
bundle-cef-app Charlotte -o target\bundle --release
if errorlevel 1 exit /b %errorlevel%
echo Charlotte bundle written to target\bundle
