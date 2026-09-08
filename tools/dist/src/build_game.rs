use std::{
    env,
    error::Error,
    io,
    path::{Path, PathBuf},
    process::Command,
};

pub fn build_release(project_root: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let status = Command::new("cargo")
        .args(["build", "--release", "-p", "mineclone", "--bin", "mineclone"])
        .current_dir(project_root)
        .status()?;

    if !status.success() {
        return Err(io::Error::other("release build failed").into());
    }

    let executable = target_directory(project_root)
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

fn target_directory(project_root: &Path) -> PathBuf {
    let Some(target_dir) = env::var_os("CARGO_TARGET_DIR") else {
        return project_root.join("target");
    };

    let target_dir = PathBuf::from(target_dir);
    if target_dir.is_absolute() {
        target_dir
    } else {
        project_root.join(target_dir)
    }
}

fn executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "mineclone.exe"
    } else {
        "mineclone"
    }
}
