mod session;
mod timestamp;
#[cfg(target_os = "windows")]
mod windows;

use std::{
    any::Any,
    backtrace::Backtrace,
    env,
    io::Write,
    panic::{self, PanicHookInfo},
    sync::atomic::{AtomicBool, Ordering},
    time::SystemTime,
};

use self::{
    session::{append_runtime_line, append_session_line, initialize_session_log, with_session_file},
    timestamp::format_timestamp,
};

static PANIC_LOG_WRITTEN: AtomicBool = AtomicBool::new(false);

pub fn install_crash_logger() {
    PANIC_LOG_WRITTEN.store(false, Ordering::Release);
    initialize_session_log();

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

pub(crate) fn append_runtime_diagnostic(line: &str) -> bool {
    append_runtime_line(line)
}

pub(crate) fn log_gameplay_event(message: impl Into<String>) {
    log_gameplay_line("EVENT", message.into());
}

pub(crate) fn log_gameplay_warn(message: impl Into<String>) {
    log_gameplay_line("WARN", message.into());
}

pub(crate) fn log_gameplay_error(message: impl Into<String>) {
    log_gameplay_line("ERROR", message.into());
}

pub(crate) fn log_system_event(message: impl Into<String>) {
    log_system_line("EVENT", message.into());
}

pub(crate) fn log_system_warn(message: impl Into<String>) {
    log_system_line("WARN", message.into());
}

pub(crate) fn log_system_error(message: impl Into<String>) {
    log_system_line("ERROR", message.into());
}

pub(crate) fn log_diagnostic(message: impl Into<String>) {
    let message = message.into();
    bevy::log::info!(target: "asteria::diagnostic", "{message}");
    let _ = append_runtime_diagnostic(&format!("[DIAGNOSTIC] {message}"));
}

fn log_gameplay_line(level: &str, message: String) {
    match level {
        "WARN" => bevy::log::warn!(target: "asteria::gameplay", "{message}"),
        "ERROR" => bevy::log::error!(target: "asteria::gameplay", "{message}"),
        _ => bevy::log::info!(target: "asteria::gameplay", "{message}"),
    }
    let _ = append_runtime_diagnostic(&format!("[{level}] {message}"));
}

fn log_system_line(level: &str, message: String) {
    match level {
        "WARN" => bevy::log::warn!(target: "asteria::system", "{message}"),
        "ERROR" => bevy::log::error!(target: "asteria::system", "{message}"),
        _ => bevy::log::info!(target: "asteria::system", "{message}"),
    }
    let _ = append_runtime_diagnostic(&format!("[SYSTEM {level}] {message}"));
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
    with_session_file(|file| {
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
    })
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
