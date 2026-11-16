@echo off
title PhoCast - Driver Cleanup Tool
echo Requesting administrator privileges...

:: Check for Administrator Permissions
net session >nul 2>&1
if %errorlevel% neq 0 (
    powershell -Command "Start-Process '%~f0' -Verb RunAs"
    exit /b
)

echo [1/3] Unregistering DirectShow DLLs...
regsvr32 /u /s "C:\Program Files (x86)\PhoCast\driver\UnityCaptureFilter64.dll"
regsvr32 /u /s "C:\Program Files (x86)\PhoCast\driver\UnityCaptureFilter32.dll"
if exist "%~dp0driver\UnityCaptureFilter64.dll" regsvr32 /u /s "%~dp0driver\UnityCaptureFilter64.dll"
if exist "%~dp0driver\UnityCaptureFilter32.dll" regsvr32 /u /s "%~dp0driver\UnityCaptureFilter32.dll"

echo [2/3] Deleting DirectShow Filter Registry Entries...
reg delete "HKLM\Software\Classes\CLSID\{860BB310-5D01-11d0-BD3B-00A0C911CE86}\Instance\{5C2CD55C-92AD-4999-8666-912BD3E70010}" /f >nul 2>&1
reg delete "HKLM\Software\Classes\CLSID\{5C2CD55C-92AD-4999-8666-912BD3E70010}" /f >nul 2>&1
reg delete "HKLM\Software\Classes\WOW6432Node\CLSID\{860BB310-5D01-11d0-BD3B-00A0C911CE86}\Instance\{5C2CD55C-92AD-4999-8666-912BD3E70010}" /f >nul 2>&1
reg delete "HKLM\Software\Classes\WOW6432Node\CLSID\{5C2CD55C-92AD-4999-8666-912BD3E70010}" /f >nul 2>&1

echo [3/3] Deleting Driver Files...
if exist "C:\Program Files (x86)\PhoCast\driver" rmdir /s /q "C:\Program Files (x86)\PhoCast\driver"

echo.
echo ========================================================
echo [SUCCESS] PhoCast DirectShow Camera Driver completely removed!
echo OBS Studio and NVIDIA Broadcast will no longer detect or crash from it.
echo ========================================================
echo.
timeout /t 3
