@echo off
title PhoCast Client (Wi-Fi Mode)
echo ========================================================
echo           PhoCast PC Client - Wi-Fi Mode
echo ========================================================
echo.
set /p PHONE_IP="Enter your phone's IP address displayed in PhoCast (e.g. 192.168.1.50): "
echo Connecting to %PHONE_IP%:5000...
python phocast_client.py --ip %PHONE_IP%
pause
