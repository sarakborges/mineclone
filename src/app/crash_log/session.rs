use std::{
    env,
    fs::{File, OpenOptions, create_dir_all},
    io::Write,
    path::PathBuf,
    sync::OnceLock,
    time::SystemTime,
};

use super::timestamp::format_timestamp;

const LOG_DIRECTORY: &str = "logs";
const DATA_DIRECTORY: &str = "data";
const ASSETS_DIRECTORY: &str = "assets";

static SESSION_LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub(super) fn initialize_session_log() {
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
        let _ = writeln!(
            file,
            "Platform: {} / {}",
            env::consts::OS,
            env::consts::ARCH
        );
        let _ = writeln!(file);
        let _ = file.flush();
    }

    let _ = SESSION_LOG_PATH.set(path);
}

pub(super) fn append_session_line(line: &str) -> bool {
    with_session_file(|file| {
        let timestamp = format_timestamp(SystemTime::now());
        let _ = writeln!(file, "{line}");
        let _ = writeln!(file, "Timestamp (UTC): {}", timestamp.display);
        let _ = writeln!(file);
    })
}

pub(super) fn with_session_file(write: impl FnOnce(&mut File)) -> bool {
    let Some(path) = SESSION_LOG_PATH.get() else {
        return false;
    };
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return false;
    };

    write(&mut file);
    file.flush().is_ok()
}

fn runtime_root() -> PathBuf {
    if let Ok(executable) = env::current_exe()
        && let Some(directory) = executable.parent()
        && directory.join(DATA_DIRECTORY).is_dir()
        && directory.join(ASSETS_DIRECTORY).is_dir()
    {
        return directory.to_path_buf();
    }

    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
