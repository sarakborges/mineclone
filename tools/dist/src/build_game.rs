use std::{
    error::Error,
    io,
    path::{Path, PathBuf},
    process::Command,
};

pub fn build_release(project_root: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let target_directory = project_root.join("target");
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--bin")
        .arg("asteria")
        .arg("--target-dir")
        .arg(&target_directory)
        .current_dir(project_root)
        .status()?;

    if !status.success() {
        return Err(io::Error::other("release build failed").into());
    }

    let executable = target_directory
        .join("release")
        .join(executable_name());

    if !executable.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("release executable not found at {}", executable.display()),
        )
        .into());
    }

    Ok(executable)
}

fn executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "asteria.exe"
    } else {
        "asteria"
    }
}
