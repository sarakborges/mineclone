use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io,
    path::Path,
    sync::{
        Arc, Condvar, Mutex, MutexGuard, OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use bevy::prelude::Resource;

const SESSION_LOCK_FILE: &str = "session.lock";

static WORLD_LOCKS: OnceLock<Mutex<HashMap<String, Arc<WorldGate>>>> = OnceLock::new();

#[derive(Resource)]
pub(crate) struct WorldDirectoryLock {
    _file: fs::File,
}

pub(super) fn acquire_world_directory_lock(
    directory: &Path,
) -> io::Result<WorldDirectoryLock> {
    let path = directory.join(SESSION_LOCK_FILE);
    if let Ok(metadata) = fs::symlink_metadata(&path)
        && (!metadata.file_type().is_file() || metadata.file_type().is_symlink())
    {
        return Err(invalid_data("world session lock must be a regular file"));
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)?;
    if !file.metadata()?.is_file() {
        return Err(invalid_data(
            "opened world session lock is not a regular file",
        ));
    }

    match file.try_lock() {
        Ok(()) => Ok(WorldDirectoryLock { _file: file }),
        Err(fs::TryLockError::WouldBlock) => Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "world is already open in another Asteria process",
        )),
        Err(fs::TryLockError::Error(error)) => Err(error),
    }
}

pub(super) fn remove_world_directory_lock_file(directory: &Path) -> io::Result<()> {
    match fs::remove_file(directory.join(SESSION_LOCK_FILE)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[derive(Default)]
pub(super) struct WorldGate {
    write: Mutex<()>,
    readers: AtomicUsize,
    reader_signal: Mutex<()>,
    readers_finished: Condvar,
    prune_running: AtomicBool,
}

impl WorldGate {
    pub(super) fn lock_write(&self) -> io::Result<MutexGuard<'_, ()>> {
        self.write
            .lock()
            .map_err(|_| io::Error::other("world save lock poisoned"))
    }

    pub(super) fn pin_read(self: &Arc<Self>) -> ReadLease {
        self.readers.fetch_add(1, Ordering::AcqRel);
        ReadLease(Arc::clone(self))
    }

    pub(super) fn try_begin_prune(self: &Arc<Self>) -> Option<PruneLease> {
        if self.prune_running.swap(true, Ordering::AcqRel) {
            return None;
        }
        Some(PruneLease(Arc::clone(self)))
    }

    pub(super) fn lock_after_readers(&self) -> io::Result<MutexGuard<'_, ()>> {
        loop {
            let write = self.lock_write()?;
            if self.readers.load(Ordering::Acquire) == 0 {
                return Ok(write);
            }
            drop(write);

            let mut signal = self
                .reader_signal
                .lock()
                .map_err(|_| io::Error::other("reader wait lock poisoned"))?;
            while self.readers.load(Ordering::Acquire) != 0 {
                signal = self
                    .readers_finished
                    .wait(signal)
                    .map_err(|_| io::Error::other("reader wait lock poisoned"))?;
            }
        }
    }
}

pub(super) fn world_lock(id: &str) -> io::Result<Arc<WorldGate>> {
    let locks = WORLD_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut locks = locks
        .lock()
        .map_err(|_| io::Error::other("world lock catalog poisoned"))?;
    Ok(Arc::clone(
        locks
            .entry(id.to_owned())
            .or_insert_with(|| Arc::new(WorldGate::default())),
    ))
}

pub(super) struct ReadLease(Arc<WorldGate>);

impl Drop for ReadLease {
    fn drop(&mut self) {
        let _signal = self
            .0
            .reader_signal
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if self.0.readers.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.0.readers_finished.notify_all();
        }
    }
}

pub(super) struct PruneLease(Arc<WorldGate>);

impl Drop for PruneLease {
    fn drop(&mut self) {
        self.0.prune_running.store(false, Ordering::Release);
    }
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
