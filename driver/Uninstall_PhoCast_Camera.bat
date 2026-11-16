@echo off
title Uninstall PhoCast Virtual Camera Driver

:: Check for Administrator Permissions
>nul 2>&1 "%SYSTEMROOT%\system32\cacls.exe" "%SYSTEMROOT%\system32\config\system"
if '%errorlevel%' NEQ '0' (
    echo Requesting administrator privileges...
    powershell -Command "Start-Process cmd -ArgumentList '/c \"%~s0\"' -Verb runAs"
    exit /b
)

cd /d "%~dp0"
echo ========================================================
echo       Uninstalling PhoCast Camera Driver
echo ========================================================
echo.
regsvr32 /u /s "UnityCaptureFilter64.dll"
regsvr32 /u /s "UnityCaptureFilter32.dll"

echo [SUCCESS] 'PhoCast Camera' has been unregistered.
echo.
pause
