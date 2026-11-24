@echo off
rem Installed files are read-only; each Windows user keeps configuration and caches under %LOCALAPPDATA%\Calendar.
set "DATA=%LOCALAPPDATA%\Calendar"
if not exist "%DATA%" mkdir "%DATA%"
if not exist "%DATA%\.env" copy "%~dp0.env.example" "%DATA%\.env" >nul
cd /d "%DATA%"
start "" "%~dp0calendar-desktop.exe"
