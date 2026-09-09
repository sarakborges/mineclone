use std::{
    any::Any,
    backtrace::Backtrace,
    env,
    fs::{create_dir_all, OpenOptions},
    io::Write,
    panic::{self, PanicHookInfo},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

const LOG_DIRECTORY: &str = "logs";
const DATA_DIRECTORY: &str = "data";
const ASSETS_DIRECTORY: &str = "assets";

static PANIC_LOG_WRITTEN: AtomicBool = AtomicBool::new(false);
static SESSION_LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn install_crash_logger() {
    PANIC_LOG_WRITTEN.store(false, Ordering::Release);
    let path = initialize_session_log();
    let _ = SESSION_LOG_PATH.set(path);

    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if write_panic_log(info) {
            PANIC_LOG_WRITTEN.store(true, Ordering::Release);
        }
        previous_hook(info);
    }));

    #[cfg(target_os = "windows")]
    windows::install_exception_handler();
}

pub fn write_caught_panic(payload: &(dyn Any + Send)) {
    if PANIC_LOG_WRITTEN.load(Ordering::Acquire) {
        return;
    }

    let message = panic_payload_message(payload);
    if write_report(&message, "<unavailable; panic caught after unwind>") {
        PANIC_LOG_WRITTEN.store(true, Ordering::Release);
    }
}

pub fn mark_clean_shutdown() {
    let _ = append_session_line("CLEAN SHUTDOWN");
}

fn initialize_session_log() -> PathBuf {
    let timestamp = format_timestamp(SystemTime::now());
    let root = runtime_root();
    let log_directory = root.join(LOG_DIRECTORY);
    let preferred_path = log_directory.join(format!("{}.txt", timestamp.file_name));

    let path = if create_dir_all(&log_directory).is_ok() {
        preferred_path
    } else {
        root.join(format!("asteria-crash-{}.txt", timestamp.file_name))
    };

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&path)
    {
        let _ = writeln!(file, "Asteria session log");
        let _ = writeln!(file, "===================");
        let _ = writeln!(file, "START");
        let _ = writeln!(file, "Started (UTC): {}", timestamp.display);
        let _ = writeln!(file, "Process ID: {}", std::process::id());
        let _ = writeln!(file, "Platform: {} / {}", env::consts::OS, env::consts::ARCH);
        let _ = writeln!(file);
        let _ = file.flush();
    }

    path
}

fn append_session_line(line: &str) -> bool {
    let Some(path) = SESSION_LOG_PATH.get() else {
        return false;
    };
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return false;
    };

    let timestamp = format_timestamp(SystemTime::now());
    let _ = writeln!(file, "{line}");
    let _ = writeln!(file, "Timestamp (UTC): {}", timestamp.display);
    let _ = writeln!(file);
    file.flush().is_ok()
}

fn write_panic_log(info: &PanicHookInfo<'_>) -> bool {
    let message = panic_message(info);
    let location = info
        .location()
        .map(|location| {
            format!(
                "{}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            )
        })
        .unwrap_or_else(|| "<unknown>".to_owned());

    write_report(&message, &location)
}

fn write_report(message: &str, location: &str) -> bool {
    let Some(path) = SESSION_LOG_PATH.get() else {
        return false;
    };
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return false;
    };

    let timestamp = format_timestamp(SystemTime::now());
    let thread = std::thread::current();
    let thread_name = thread.name().unwrap_or("<unnamed>");
    let current_directory = env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "<unavailable>".to_owned());
    let executable = env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "<unavailable>".to_owned());
    let backtrace = Backtrace::force_capture();

    let _ = writeln!(file, "RUST PANIC");
    let _ = writeln!(file, "==========");
    let _ = writeln!(file, "Timestamp (UTC): {}", timestamp.display);
    let _ = writeln!(file, "Thread: {thread_name}");
    let _ = writeln!(file, "Executable: {executable}");
    let _ = writeln!(file, "Working directory: {current_directory}");
    let _ = writeln!(file, "Panic: {message}");
    let _ = writeln!(file, "Location: {location}");
    let _ = writeln!(file);
    let _ = writeln!(file, "Backtrace:");
    let _ = writeln!(file, "{backtrace}");
    let _ = writeln!(file);

    file.flush().is_ok()
}

#[cfg(target_os = "windows")]
fn write_native_exception(code: u32, address: usize) -> bool {
    let Some(path) = SESSION_LOG_PATH.get() else {
        return false;
    };
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return false;
    };

    let timestamp = format_timestamp(SystemTime::now());
    let _ = writeln!(file, "WINDOWS NATIVE EXCEPTION");
    let _ = writeln!(file, "========================");
    let _ = writeln!(file, "Timestamp (UTC): {}", timestamp.display);
    let _ = writeln!(file, "Exception code: 0x{code:08X} ({})", windows_exception_name(code));
    let _ = writeln!(file, "Exception address: 0x{address:016X}");
    let _ = writeln!(file);

    file.flush().is_ok()
}

#[cfg(target_os = "windows")]
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

fn panic_message(info: &PanicHookInfo<'_>) -> String {
    panic_payload_message(info.payload())
}

fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_owned();
    }

    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }

    "<non-string panic payload>".to_owned()
}

fn runtime_root() -> PathBuf {
    if let Ok(executable) = env::current_exe() {
        if let Some(directory) = executable.parent() {
            if directory.join(DATA_DIRECTORY).is_dir()
                && directory.join(ASSETS_DIRECTORY).is_dir()
            {
                return directory.to_path_buf();
            }
        }
    }

    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

struct Timestamp {
    file_name: String,
    display: String,
}

fn format_timestamp(time: SystemTime) -> Timestamp {
    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let total_seconds = duration.as_secs() as i64;
    let nanoseconds = duration.subsec_nanos();
    let days = total_seconds.div_euclid(86_400);
    let seconds_of_day = total_seconds.rem_euclid(86_400);
    let (year, month, day) = civil_date_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = seconds_of_day % 3_600 / 60;
    let second = seconds_of_day % 60;

    let file_name = format!(
        "{year:04}-{month:02}-{day:02}_{hour:02}-{minute:02}-{second:02}-{nanoseconds:09}"
    );
    let display = format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{nanoseconds:09}Z"
    );

    Timestamp { file_name, display }
}

fn civil_date_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096)
            / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };

    (year, month, day)
}

#[cfg(target_os = "windows")]
mod windows {
    use std::ffi::c_void;

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

        let _ = super::write_native_exception(code, address);
        EXCEPTION_EXECUTE_HANDLER
    }
}
