@echo off
setlocal EnableExtensions
cd /d "%~dp0\.."
if not "%~1"=="" (echo Cross-target packaging is not supported. & exit /b 2)
if not defined CEF_PATH (echo Set CEF_PATH to a matching CEF distribution exported by export-cef-dir. & exit /b 2)
if not exist "%CEF_PATH%\" (echo CEF_PATH is not a directory. & exit /b 2)
where bundle-cef-app >nul 2>nul
if errorlevel 1 (echo Install bundle-cef-app from cef 156.2.0+156.0.14 first. & exit /b 2)
bundle-cef-app Charlotte -o target\bundle --release
if errorlevel 1 exit /b %errorlevel%
echo Charlotte bundle written to target\bundle
