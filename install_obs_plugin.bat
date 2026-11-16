@echo off
title PhoCast - Install OBS Studio Plugin
echo Requesting administrator privileges...

:: Check for Administrator Permissions
net session >nul 2>&1
if %errorlevel% neq 0 (
    powershell -Command "Start-Process '%~f0' -Verb RunAs"
    exit /b
)

echo ========================================================
echo       Installing PhoCast Native OBS Studio Plugin
echo ========================================================
echo.

set "SOURCE_DLL=%~dp0obs-plugin\target\release\phocast_obs.dll"
set "OBS_DIR=C:\Program Files\obs-studio\obs-plugins\64bit"

if not exist "%SOURCE_DLL%" (
    echo [ERROR] %SOURCE_DLL% not found!
    echo Please compile the plugin first: cd obs-plugin ^&^& cargo build --release
    echo.
    pause
    exit /b 1
)

if not exist "%OBS_DIR%" (
    echo [ERROR] OBS Studio 64-bit plugins directory not found at:
    echo %OBS_DIR%
    echo Please install OBS Studio 64-bit or check your installation path.
    echo.
    pause
    exit /b 1
)

echo Copying phocast_obs.dll to %OBS_DIR%...
copy /y "%SOURCE_DLL%" "%OBS_DIR%\phocast_obs.dll"

if %errorlevel% equ 0 (
    echo.
    echo ========================================================
    echo [SUCCESS] PhoCast OBS Plugin successfully installed!
    echo.
    echo Next Steps:
    echo 1. Open or Restart OBS Studio.
    echo 2. In Sources pane, click '+' (Add Source).
    echo 3. Select 'PhoCast Camera'.
    echo 4. Enjoy 1080p 60FPS crystal clear stream with 0 driver crashes!
    echo ========================================================
) else (
    echo [ERROR] Failed to copy plugin. Please ensure OBS Studio is closed and try again.
)

echo.
pause
