@echo off
rem Starts the tray process (one instance, keeps syncing) and opens the calendar window.
start "" "%~dp0calendar-tray.exe"
start "" "%~dp0calendar-desktop.exe"
