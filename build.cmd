@echo off
setlocal

cd /d "%~dp0"

cargo run --manifest-path tools\dist\Cargo.toml --target-dir target\dist-tool --
if errorlevel 1 exit /b %errorlevel%

if not exist "build\Asteria.exe" (
    echo ERROR: build\Asteria.exe was not created.
    exit /b 1
)

if not exist "build\data\" (
    echo ERROR: build\data was not created.
    exit /b 1
)

if not exist "build\assets\" (
    echo ERROR: build\assets was not created.
    exit /b 1
)

echo.
echo Asteria build ready at: %CD%\build
