@echo off
setlocal EnableExtensions

cd /d "%~dp0"
if errorlevel 1 goto :fail

echo [1/4] Checking Cargo...
where cargo >nul 2>nul
if errorlevel 1 (
    echo ERROR: Cargo was not found in PATH.
    echo Install Rust with rustup or reopen the terminal after installing it.
    goto :fail
)

echo [2/4] Building Asteria release...
cargo build --release --bin asteria
if errorlevel 1 goto :fail

if not exist "target\release\asteria.exe" (
    echo ERROR: target\release\asteria.exe was not created.
    goto :fail
)

echo [3/4] Packaging build directory...
if exist "build" rmdir /s /q "build"
if exist "build" (
    echo ERROR: Could not remove the previous build directory.
    echo Close Asteria.exe or any program using files inside build and try again.
    goto :fail
)

mkdir "build"
if errorlevel 1 goto :fail

copy /y "target\release\asteria.exe" "build\Asteria.exe" >nul
if errorlevel 1 goto :fail

xcopy "data" "build\data\" /E /I /H /K /R /Y >nul
if errorlevel 1 goto :fail

xcopy "assets" "build\assets\" /E /I /H /K /R /Y >nul
if errorlevel 1 goto :fail

echo [4/4] Verifying distribution...
if not exist "build\Asteria.exe" (
    echo ERROR: build\Asteria.exe was not created.
    goto :fail
)
if not exist "build\data\" (
    echo ERROR: build\data was not created.
    goto :fail
)
if not exist "build\assets\" (
    echo ERROR: build\assets was not created.
    goto :fail
)

echo.
echo Asteria build ready at: %CD%\build
exit /b 0

:fail
set "BUILD_ERROR=%errorlevel%"
if "%BUILD_ERROR%"=="0" set "BUILD_ERROR=1"
echo.
echo BUILD FAILED with exit code %BUILD_ERROR%.
echo.
pause
exit /b %BUILD_ERROR%
