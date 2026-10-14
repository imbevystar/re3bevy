@echo off
setlocal
rust-lld.exe -flavor link %*
if errorlevel 1 exit /b %errorlevel%
if not "%CARGO_PKG_NAME%"=="rustgta3" exit /b 0
if "%CARGO_BIN_NAME%"=="rustgta3" goto publish
if "%CARGO_BIN_NAME%"=="rustgta3newgame" goto publish
exit /b 0
:publish
powershell.exe -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "%~dp0update-dist.ps1" %*
exit /b %errorlevel%
