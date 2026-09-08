use std::{
    error::Error,
    ffi::OsStr,
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

    let source_assets = project_root.join("assets");
    copy_directory(&source_assets.join("data"), &build_directory.join("data"))?;

    let build_assets = build_directory.join("assets");
    fs::create_dir_all(&build_assets)?;

    for entry in fs::read_dir(&source_assets)? {
        let entry = entry?;
        if entry.file_name() == OsStr::new("data") {
            continue;
        }

        let source = entry.path();
        let destination = build_assets.join(entry.file_name());

        if source.is_dir() {
            copy_directory(&source, &destination)?;
        } else {
            fs::copy(source, destination)?;
        }
    }

    Ok(build_directory)
}

fn distribution_executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "Asteria.exe"
    } else {
        "Asteria"
    }
}
