@echo off
title Install PhoCast Virtual Camera Driver

:: Check for Administrator Permissions
>nul 2>&1 "%SYSTEMROOT%\system32\cacls.exe" "%SYSTEMROOT%\system32\config\system"
if '%errorlevel%' NEQ '0' (
    echo Requesting administrator privileges...
    powershell -Command "Start-Process cmd -ArgumentList '/c \"%~s0\"' -Verb runAs"
    exit /b
)

cd /d "%~dp0"
echo ========================================================
echo       Installing PhoCast Camera (DirectShow Driver)
echo ========================================================
echo.
regsvr32 /s "UnityCaptureFilter64.dll" "/i:UnityCaptureName=PhoCast Camera"
regsvr32 /s "UnityCaptureFilter32.dll" "/i:UnityCaptureName=PhoCast Camera"

echo [SUCCESS] 'PhoCast Camera' has been registered in Windows!
echo You can now select 'PhoCast Camera' under 'Video Capture Device'
echo in OBS Studio, Zoom, Discord, Google Meet, Teams, etc.
echo.
pause
