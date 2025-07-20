@echo off
:: SSH AI Terminal - Stop Script
:: Stops all running instances

echo ============================================
echo     SSH AI Terminal - Stop Service
echo ============================================
echo.

:: Find and kill ssh-ai-terminal.exe processes
echo [INFO] Looking for SSH AI Terminal processes...
echo.

set FOUND=0
for /f "tokens=2" %%a in ('tasklist ^| findstr /i "ssh-ai-terminal.exe"') do (
    set FOUND=1
    echo [INFO] Found process with PID: %%a
    taskkill /f /pid %%a >nul 2>&1
    if !errorlevel! equ 0 (
        echo [SUCCESS] Stopped process %%a
    ) else (
        echo [ERROR] Failed to stop process %%a
    )
)

if %FOUND% equ 0 (
    echo [INFO] No SSH AI Terminal processes found
) else (
    echo.
    echo [SUCCESS] All processes stopped
)

:: Also check port 8005
echo.
echo [INFO] Checking port 8005...
for /f "tokens=5" %%a in ('netstat -ano ^| findstr ":8005" ^| findstr "LISTENING"') do (
    echo [INFO] Found process on port 8005 (PID: %%a)
    taskkill /f /pid %%a >nul 2>&1
    if !errorlevel! equ 0 (
        echo [SUCCESS] Stopped process on port 8005
    )
)

echo.
echo [INFO] Cleanup complete
echo.
pause