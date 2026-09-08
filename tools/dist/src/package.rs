use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use crate::copy_tree::copy_directory;

pub fn create_distribution(
    project_root: &Path,
    executable: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    let build_directory = project_root.join("build");

    if build_directory.exists() {
        fs::remove_dir_all(&build_directory)?;
    }

    fs::create_dir_all(&build_directory)?;
    fs::copy(executable, build_directory.join(distribution_executable_name()))?;
    copy_directory(&project_root.join("data"), &build_directory.join("data"))?;
    copy_directory(&project_root.join("assets"), &build_directory.join("assets"))?;

    Ok(build_directory)
}

fn distribution_executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "Asteria.exe"
    } else {
        "Asteria"
    }
}
