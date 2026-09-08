use std::{
    env,
    path::{Path, PathBuf},
};

const DATA_DIRECTORY: &str = "data";
const ASSETS_DIRECTORY: &str = "assets";

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
    let path = Path::new(DATA_DIRECTORY);
    assert!(path.is_dir(), "data directory not found at {}", path.display());
    path.to_path_buf()
}
