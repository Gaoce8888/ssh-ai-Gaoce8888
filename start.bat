@echo off
setlocal EnableDelayedExpansion

:: SSH AI Terminal Startup Script
:: ===============================

cls
echo ============================================
echo        SSH AI Terminal - Startup
echo ============================================
echo.

:: Check if running as administrator
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo [WARNING] Not running as administrator
    echo Some features may not work properly
    echo.
)

:: Set working directory to script location
cd /d "%~dp0"
echo [INFO] Working directory: %CD%
echo.

:: Check if executable exists
if not exist "target\release\ssh-ai-terminal.exe" (
    echo [ERROR] Executable not found!
    echo Please build the project first using:
    echo   cargo build --release
    echo.
    pause
    exit /b 1
)

:: Check if port 8005 is available
netstat -an | findstr ":8005" | findstr "LISTENING" >nul 2>&1
if %errorlevel% equ 0 (
    echo [WARNING] Port 8005 is already in use!
    echo.
    echo Options:
    echo 1. Kill the process using port 8005
    echo 2. Exit and handle manually
    echo.
    choice /c 12 /n /m "Select option (1-2): "
    if !errorlevel! equ 1 (
        echo.
        echo Killing process on port 8005...
        for /f "tokens=5" %%a in ('netstat -ano ^| findstr ":8005" ^| findstr "LISTENING"') do (
            taskkill /f /pid %%a >nul 2>&1
            if !errorlevel! equ 0 (
                echo [SUCCESS] Process killed
            ) else (
                echo [ERROR] Failed to kill process
            )
        )
        timeout /t 2 /nobreak >nul
    ) else (
        echo.
        echo Exiting...
        pause
        exit /b 1
    )
)

:: Check configuration file
if not exist "config.json" (
    echo [WARNING] config.json not found!
    if exist "config-template.json" (
        echo Creating config.json from template...
        copy "config-template.json" "config.json" >nul
        echo [SUCCESS] config.json created
    ) else (
        echo [ERROR] No configuration template found!
        pause
        exit /b 1
    )
)
echo.

:: Create necessary directories
echo [INFO] Checking directories...
if not exist "logs" mkdir "logs"
if not exist "data" mkdir "data"
if not exist "static" (
    echo [ERROR] Static files directory not found!
    pause
    exit /b 1
)
echo [SUCCESS] All directories ready
echo.

:: Display configuration
echo [INFO] Server Configuration:
for /f "tokens=2 delims=:," %%a in ('findstr /c:"\"port\"" config.json') do (
    set PORT=%%a
    set PORT=!PORT: =!
    echo   - Port: !PORT!
)
for /f "tokens=2 delims=:," %%a in ('findstr /c:"\"address\"" config.json') do (
    set ADDR=%%a
    set ADDR=!ADDR: =!
    set ADDR=!ADDR:"=!
    echo   - Address: !ADDR!
)
echo.

:: Start the server
echo [INFO] Starting SSH AI Terminal...
echo ============================================
echo.

:: Run the executable
target\release\ssh-ai-terminal.exe

:: Check exit code
if %errorlevel% neq 0 (
    echo.
    echo [ERROR] Server exited with error code: %errorlevel%
    pause
    exit /b %errorlevel%
)

echo.
echo [INFO] Server stopped
pause
endlocal