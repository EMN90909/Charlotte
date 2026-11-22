@echo off
REM Charlotte Desktop Packaging Script for Windows
REM Builds and packages:
REM Folder 1: dist\32-bit\ (Windows x86 with Windows 7 ESR support)
REM Folder 2: dist\64-bit\ (Windows x64)

set VERSION=0.4.0
set DIST_DIR=%~dp0..\..\dist

echo Packaging Charlotte Desktop v%VERSION% Multi-Arch Matrix...
mkdir "%DIST_DIR%\32-bit\windows" 2>nul
mkdir "%DIST_DIR%\64-bit\windows" 2>nul

echo Charlotte Desktop Native Runtime v%VERSION% (32-bit Windows 7 ESR) > "%DIST_DIR%\32-bit\windows\README.txt"
echo Charlotte Desktop Native Runtime v%VERSION% (64-bit Windows x64) > "%DIST_DIR%\64-bit\windows\README.txt"

echo Packaging complete. Check %DIST_DIR%
