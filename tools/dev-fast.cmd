@echo off
setlocal

for /f "delims=" %%I in ('rustc --print sysroot') do set "RUST_SYSROOT=%%I"
set "RUST_LLD=%RUST_SYSROOT%\lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe"

if not exist "%RUST_LLD%" (
    echo Asteria fast dev launcher: bundled rust-lld.exe was not found.
    echo Falling back is intentionally disabled so the normal development flow stays unchanged.
    exit /b 1
)

set "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=%RUST_LLD%"
cargo run %*
