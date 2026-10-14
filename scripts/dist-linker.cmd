@echo off
setlocal
rust-lld.exe -flavor link %*
if errorlevel 1 exit /b %errorlevel%
if not "%CARGO_PKG_NAME%"=="rust-gta-3" exit /b 0
if "%CARGO_BIN_NAME%"=="rust-gta-3" goto publish
if "%CARGO_BIN_NAME%"=="rust-gta-3-newgame" goto publish
exit /b 0
:publish
powershell.exe -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "%~dp0update-dist.ps1" %*
exit /b %errorlevel%
