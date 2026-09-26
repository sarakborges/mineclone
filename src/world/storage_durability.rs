use std::{io, path::Path};

#[cfg(unix)]
use std::fs;

/// Make a directory entry durable when the platform exposes a portable
/// directory-sync capability through Rust's standard library.
///
/// Regular files are always synced before publication. On Windows, opening a
/// directory with `File::open` is not a portable fsync mechanism and returns
/// ERROR_ACCESS_DENIED on normal world directories, so directory syncing is a
/// no-op rather than turning a successful atomic rename into a failed save.
#[cfg(unix)]
pub(crate) fn sync_directory(directory: &Path) -> io::Result<()> {
    fs::File::open(directory)?.sync_all()
}

#[cfg(not(unix))]
pub(crate) fn sync_directory(_directory: &Path) -> io::Result<()> {
    Ok(())
}

/// Sync every directory created below a staging root before that root is
/// atomically published. Platforms without portable directory syncing skip the
/// traversal entirely; their files have already been individually synced.
#[cfg(unix)]
pub(crate) fn sync_directory_tree(directory: &Path) -> io::Result<()> {
    let mut directories = vec![directory.to_path_buf()];
    let mut index = 0;
    while index < directories.len() {
        let current = directories[index].clone();
        index += 1;
        for entry in fs::read_dir(current.as_path())? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                directories.push(entry.path());
            }
        }
    }
    for directory in directories.into_iter().rev() {
        sync_directory(directory.as_path())?;
    }
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn sync_directory_tree(_directory: &Path) -> io::Result<()> {
    Ok(())
}
