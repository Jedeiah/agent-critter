@echo off
REM Build and package the traffic-light plugin.
REM
REM Usage:
REM   build-plugin.bat                              # build current platform + package
REM   build-plugin.bat windows                      # package windows (uses target/release/)
REM   build-plugin.bat macos   path\to\mac-binary   # package macos with pre-built binary
REM   build-plugin.bat linux   path\to\linux-binary # package linux with pre-built binary

setlocal enabledelayedexpansion

echo === Building Traffic Light Plugin ===

set PLATFORM=%~1
set BINARY_PATH=%~2

REM If no args, detect platform and build
if "%PLATFORM%"=="" (
    set PLATFORM=windows
    set BINARY_PATH=target\release\traffic-light-daemon.exe
    echo Building for current platform...
    cargo build --release
    if errorlevel 1 (
        echo Build failed!
        exit /b 1
    )
    goto :package
)

REM If binary path provided, use it directly
if not "%BINARY_PATH%"=="" (
    if not exist "%BINARY_PATH%" (
        echo Error: Binary not found: %BINARY_PATH%
        exit /b 1
    )
    echo [PRE-BUILT] %BINARY_PATH%
    goto :package
)

REM Platform specified but no binary path — look for current build
if "%PLATFORM%"=="windows" (
    set BINARY_PATH=target\release\traffic-light-daemon.exe
) else (
    set BINARY_PATH=target\release\traffic-light-daemon
)
if not exist "!BINARY_PATH!" (
    echo Error: Binary not found at !BINARY_PATH!. Build it first or provide a path.
    exit /b 1
)

:package
set PACKAGE=dist\claude-traffic-light-plugin-%PLATFORM%.zip
echo Platform: %PLATFORM%
echo Packaging to %PACKAGE%...

REM Create dist directory
if not exist dist mkdir dist

REM Create temp directory
set TMPDIR=%TEMP%\traffic-light-build-%RANDOM%
mkdir "%TMPDIR%"

REM Copy plugin files
xcopy /E /I /Q ".claude-plugin" "%TMPDIR%\.claude-plugin\" >nul
xcopy /E /I /Q "hooks" "%TMPDIR%\hooks\" >nul
xcopy /E /I /Q "assets" "%TMPDIR%\assets\" >nul
copy /Y "README.md" "%TMPDIR%\" >nul

REM Copy binary (at root of zip, matching hooks.json path)
copy /Y "%BINARY_PATH%" "%TMPDIR%\" >nul
if errorlevel 1 (
    echo Error: Failed to copy binary!
    rmdir /S /Q "%TMPDIR%"
    exit /b 1
)

REM Create zip (use abs path since pushd changes %CD%)
set ABS_PACKAGE=%CD%\%PACKAGE%
pushd "%TMPDIR%"
powershell -Command "Compress-Archive -Path '*' -DestinationPath '%ABS_PACKAGE%' -Force"
popd

REM Cleanup
rmdir /S /Q "%TMPDIR%"

echo.
echo === Package created ===
echo File: %PACKAGE%
for %%F in ("%PACKAGE%") do set SIZE=%%~zF
echo Size: %SIZE% bytes
