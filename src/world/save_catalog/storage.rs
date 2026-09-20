use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{invalid_data, snapshot::WorldManifest};

const MAX_SNAPSHOT_BYTES: u64 = 512 * 1024 * 1024;

pub(super) fn open_snapshot_file(
    directory: &Path,
    manifest: &WorldManifest,
) -> io::Result<fs::File> {
    let filename = manifest
        .snapshot_file
        .as_ref()
        .ok_or_else(|| invalid_data("no complete snapshot"))?;
    let expected = snapshot_name(manifest.generation);
    if filename != &expected {
        return Err(invalid_data(
            "manifest references a noncanonical snapshot path",
        ));
    }

    let path = directory.join(filename);
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_SNAPSHOT_BYTES {
        return Err(invalid_data(
            "snapshot is not a regular file or exceeds supported size",
        ));
    }

    let file = fs::File::open(path)?;
    let opened_metadata = file.metadata()?;
    if !opened_metadata.is_file() || opened_metadata.len() > MAX_SNAPSHOT_BYTES {
        return Err(invalid_data(
            "opened snapshot is not a regular file or exceeds supported size",
        ));
    }
    Ok(file)
}

pub(super) fn highest_generation(directory: &Path) -> io::Result<u64> {
    Ok(manifest_paths(directory)?
        .into_iter()
        .map(|(generation, _)| generation)
        .max()
        .unwrap_or(0))
}

pub(super) fn manifest_paths(directory: &Path) -> io::Result<Vec<(u64, PathBuf)>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if let Some(generation) = parse_generation(&name, "manifest-") {
            result.push((generation, entry.path()));
        }
    }
    Ok(result)
}

pub(super) fn snapshot_generation(name: &str) -> Option<u64> {
    parse_generation(name, "snapshot-")
}

fn parse_generation(name: &str, prefix: &str) -> Option<u64> {
    let digits = name.strip_prefix(prefix)?.strip_suffix(".json")?;
    (digits.len() == 20 && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
}

pub(super) fn manifest_name(generation: u64) -> String {
    format!("manifest-{generation:020}.json")
}

pub(super) fn snapshot_name(generation: u64) -> String {
    format!("snapshot-{generation:020}.json")
}

struct SnapshotSizeLimit<W> {
    writer: W,
    remaining: u64,
}

impl<W: Write> Write for SnapshotSizeLimit<W> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if data.len() as u64 > self.remaining {
            return Err(invalid_data("snapshot exceeds the maximum supported size"));
        }
        let written = self.writer.write(data)?;
        self.remaining -= written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

pub(super) fn publish_json<T: Serialize>(
    directory: &Path,
    filename: &str,
    value: &T,
) -> io::Result<()> {
    let temporary = directory.join(format!("{filename}.tmp"));
    let final_path = directory.join(filename);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;

    let result = (|| {
        {
            let mut buffered = io::BufWriter::new(&mut file);
            if filename.starts_with("snapshot-") {
                let mut bounded = SnapshotSizeLimit {
                    writer: &mut buffered,
                    remaining: MAX_SNAPSHOT_BYTES,
                };
                serde_json::to_writer(&mut bounded, value).map_err(io::Error::other)?;
                bounded.write_all(b"\n")?;
            } else {
                serde_json::to_writer(&mut buffered, value).map_err(io::Error::other)?;
                buffered.write_all(b"\n")?;
            }
            buffered.flush()?;
        }
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &final_path)?;
        if let Err(error) = sync_directory(directory) {
            return Err(rollback_published_file(directory, &final_path, error));
        }
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn rollback_published_file(
    directory: &Path,
    final_path: &Path,
    publish_error: io::Error,
) -> io::Error {
    match fs::remove_file(final_path).and_then(|_| sync_directory(directory)) {
        Ok(()) => publish_error,
        Err(rollback_error) => io::Error::new(
            publish_error.kind(),
            format!(
                "{publish_error}; rollback of published file failed: {rollback_error}"
            ),
        ),
    }
}

fn sync_directory(directory: &Path) -> io::Result<()> {
    fs::File::open(directory)?.sync_all()
}

pub(super) fn read_json_file<T: for<'de> Deserialize<'de>>(file: fs::File) -> io::Result<T> {
    serde_json::from_reader(io::BufReader::new(file)).map_err(io::Error::other)
}

pub(super) fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> io::Result<T> {
    read_json_file(fs::File::open(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_names_round_trip_through_storage_parser() {
        let generation = 42;
        assert_eq!(
            parse_generation(&manifest_name(generation), "manifest-"),
            Some(generation)
        );
        assert_eq!(
            snapshot_generation(&snapshot_name(generation)),
            Some(generation)
        );
    }

    #[test]
    fn generation_parser_rejects_noncanonical_width() {
        assert_eq!(parse_generation("manifest-42.json", "manifest-"), None);
        assert_eq!(snapshot_generation("snapshot-42.json"), None);
    }
}
