@echo off
title PhoCast - USB Bridge Setup
echo ========================================================
echo        PhoCast - USB High-Speed Connection
echo ========================================================
echo.

set "ADB_BIN=%~dp0tools\adb\adb.exe"
if not exist "%ADB_BIN%" (
    set "ADB_BIN=adb"
)

echo [1/3] Checking connected Android devices...
"%ADB_BIN%" devices

echo.
echo [2/3] Setting up high-speed USB port forwarding (5000 -^> 5000)...
"%ADB_BIN%" forward tcp:5000 tcp:5000

if %errorlevel% equ 0 (
    echo.
    echo ========================================================
    echo [SUCCESS] USB Bridge Connected!
    echo.
    echo In OBS Studio:
    echo - Source: 'PhoCast Camera'
    echo - IP Address: 127.0.0.1
    echo - Port: 5000
    echo.
    echo You now have ultra-low latency direct USB streaming!
    echo ========================================================
) else (
    echo.
    echo [ERROR] Could not connect to phone.
    echo.
    echo Checklist:
    echo 1. USB cable is plugged into PC and Phone.
    echo 2. 'Developer Options' -> 'USB Debugging' is enabled on your phone.
    echo 3. When prompted on your phone screen, tap 'Allow USB debugging'.
)

echo.
pause
