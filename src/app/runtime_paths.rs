use std::{
    env,
    path::{Path, PathBuf},
};

const DATA_DIRECTORY: &str = "data";
const ASSETS_DIRECTORY: &str = "assets";
const DEVELOPMENT_DATA_DIRECTORY: &str = "assets/data";

pub fn prepare_runtime_directory() {
    let Ok(executable) = env::current_exe() else {
        return;
    };
    let Some(directory) = executable.parent() else {
        return;
    };

    if directory.join(DATA_DIRECTORY).is_dir() && directory.join(ASSETS_DIRECTORY).is_dir() {
        env::set_current_dir(directory).unwrap_or_else(|error| {
            panic!(
                "failed to use packaged runtime directory {}: {error}",
                directory.display()
            )
        });
    }
}

pub fn data_root() -> PathBuf {
    if Path::new(DATA_DIRECTORY).is_dir() {
        PathBuf::from(DATA_DIRECTORY)
    } else {
        PathBuf::from(DEVELOPMENT_DATA_DIRECTORY)
    }
}
