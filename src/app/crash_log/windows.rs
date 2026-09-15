use std::{ffi::c_void, io::Write, time::SystemTime};

use super::{session::with_session_file, timestamp::format_timestamp};

const EXCEPTION_EXECUTE_HANDLER: i32 = 1;

#[repr(C)]
struct ExceptionRecord {
    exception_code: u32,
    exception_flags: u32,
    exception_record: *mut ExceptionRecord,
    exception_address: *mut c_void,
    number_parameters: u32,
    exception_information: [usize; 15],
}

#[repr(C)]
struct ExceptionPointers {
    exception_record: *mut ExceptionRecord,
    context_record: *mut c_void,
}

type ExceptionFilter = Option<unsafe extern "system" fn(*mut ExceptionPointers) -> i32>;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetUnhandledExceptionFilter(filter: ExceptionFilter) -> ExceptionFilter;
}

pub(super) fn install_exception_handler() {
    unsafe {
        SetUnhandledExceptionFilter(Some(handle_exception));
    }
}

unsafe extern "system" fn handle_exception(info: *mut ExceptionPointers) -> i32 {
    let mut code = 0_u32;
    let mut address = 0_usize;

    if !info.is_null() {
        let record = unsafe { (*info).exception_record };
        if !record.is_null() {
            code = unsafe { (*record).exception_code };
            address = unsafe { (*record).exception_address as usize };
        }
    }

    let _ = write_native_exception(code, address);
    EXCEPTION_EXECUTE_HANDLER
}

fn write_native_exception(code: u32, address: usize) -> bool {
    with_session_file(|file| {
        let timestamp = format_timestamp(SystemTime::now());
        let _ = writeln!(file, "WINDOWS NATIVE EXCEPTION");
        let _ = writeln!(file, "========================");
        let _ = writeln!(file, "Timestamp (UTC): {}", timestamp.display);
        let _ = writeln!(
            file,
            "Exception code: 0x{code:08X} ({})",
            windows_exception_name(code)
        );
        let _ = writeln!(file, "Exception address: 0x{address:016X}");
        let _ = writeln!(file);
    })
}

fn windows_exception_name(code: u32) -> &'static str {
    match code {
        0xC0000005 => "access violation",
        0xC000001D => "illegal instruction",
        0xC000008C => "array bounds exceeded",
        0xC000008D => "floating-point denormal operand",
        0xC000008E => "floating-point divide by zero",
        0xC000008F => "floating-point inexact result",
        0xC0000090 => "floating-point invalid operation",
        0xC0000091 => "floating-point overflow",
        0xC0000092 => "floating-point stack check",
        0xC0000093 => "floating-point underflow",
        0xC0000094 => "integer divide by zero",
        0xC0000095 => "integer overflow",
        0xC0000096 => "privileged instruction",
        0xC00000FD => "stack overflow",
        0xC0000409 => "stack buffer overrun / fast fail",
        _ => "unknown native exception",
    }
}
