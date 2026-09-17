use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use super::world_names::{WORLDS_DIRECTORY, available_world_name, validate_world_name};

const INITIAL_MANIFEST: &str = "manifest-00000000000000000000.json";
const MANIFEST_TEMP: &str = "manifest-00000000000000000000.json.tmp";
const SAVE_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Deserialize, Serialize)]
struct WorldManifest {
    format_version: u32,
    id: String,
    seed: u64,
    dimension_id: String,
    ticks_per_second: u32,
    last_saved_unix_ms: u64,
}

/// Reserve the world ID with an exclusive directory creation, then publish
/// initial seed/rules metadata. A directory alone is not a valid saved world.
/// Full voxel/player persistence is a separate integration step.
pub(crate) fn create_new_world(
    requested_name: &str,
    seed: u64,
    dimension_id: &str,
    ticks_per_second: u32,
) -> io::Result<String> {
    if ticks_per_second == 0 || dimension_id.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "World seed metadata must include a dimension and a positive tick rate",
        ));
    }

    let root = Path::new(WORLDS_DIRECTORY);
    fs::create_dir_all(root)?;
    let mut candidate = available_world_name(requested_name)?;
    loop {
        validate_world_name(&candidate)?;
        let directory = root.join(&candidate);
        match fs::create_dir(&directory) {
            Ok(()) => {
                if let Err(error) = write_initial_manifest(
                    &directory,
                    &candidate,
                    seed,
                    dimension_id,
                    ticks_per_second,
                ) {
                    // Only remove the empty directory we just reserved; never
                    // delete another world's contents or hide an I/O failure.
                    let _ = fs::remove_file(directory.join(MANIFEST_TEMP));
                    let _ = fs::remove_dir(&directory);
                    return Err(error);
                }
                return Ok(candidate);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                candidate = available_world_name(&format!("Copy of {candidate}"))?;
            }
            Err(error) => return Err(error),
        }
    }
}

fn write_initial_manifest(
    directory: &Path,
    id: &str,
    seed: u64,
    dimension_id: &str,
    ticks_per_second: u32,
) -> io::Result<()> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?;
    let last_saved_unix_ms = u64::try_from(elapsed.as_millis()).map_err(io::Error::other)?;
    let manifest = WorldManifest {
        format_version: SAVE_FORMAT_VERSION,
        id: id.to_owned(),
        seed,
        dimension_id: dimension_id.to_owned(),
        ticks_per_second,
        last_saved_unix_ms,
    };
    let mut content = serde_json::to_vec_pretty(&manifest).map_err(io::Error::other)?;
    content.push(b'\n');

    let temporary = directory.join(MANIFEST_TEMP);
    let final_path = directory.join(INITIAL_MANIFEST);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(&content)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temporary, &final_path)?;

    // A world is not considered created until its published manifest can be
    // parsed and its identity matches the exclusively reserved folder.
    let persisted: WorldManifest = serde_json::from_slice(&fs::read(final_path)?)
        .map_err(io::Error::other)?;
    if persisted.id != id || persisted.format_version != SAVE_FORMAT_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "New world manifest identity or format changed during publication",
        ));
    }
    Ok(())
}
