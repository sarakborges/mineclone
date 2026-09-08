@echo off
setlocal

cargo dist
if errorlevel 1 exit /b %errorlevel%

echo.
echo Asteria build ready at: %CD%\build
