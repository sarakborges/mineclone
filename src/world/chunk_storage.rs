use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use bevy::prelude::IVec3;

use crate::voxel::chunk_disk::DiskChunk;

const CHUNK_DIRECTORY: &str = "chunks";
const CHUNK_FILE_EXTENSION: &str = "chunk.json";

pub(crate) fn generation_directory_name(generation: u64) -> String { format!("generation-{generation}") }
pub(crate) fn staging_generation_directory_name(generation: u64) -> String { format!(".generation-{generation}.tmp") }

fn checked_directory_slot(world_directory: &Path, relative: &Path) -> io::Result<PathBuf> {
    let path = world_directory.join(relative);
    match fs::symlink_metadata(path.as_path()) {
        Ok(metadata) if metadata.is_dir() => Ok(path),
        Ok(_) => {
            let display = path.display();
            Err(io::Error::new(io::ErrorKind::InvalidData, format!("chunk storage directory slot is not a real directory: {display}")))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(path),
        Err(error) => Err(error),
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ChunkDiskIdentity { chunk_position: IVec3 }
impl ChunkDiskIdentity {
    pub(crate) fn new(chunk_position: IVec3) -> Self { Self { chunk_position } }
    pub(crate) fn from_disk_chunk(chunk: &DiskChunk) -> io::Result<Self> {
        Ok(Self::new(chunk.coord()?))
    }
    pub(crate) fn chunk_position(self) -> IVec3 { self.chunk_position }
    pub(crate) fn relative_path(self) -> PathBuf {
        let position = self.chunk_position;
        let z = position.z;
        PathBuf::from(CHUNK_DIRECTORY).join(position.x.to_string()).join(position.y.to_string()).join(format!("{z}.{CHUNK_FILE_EXTENSION}"))
    }
}

pub(crate) fn publish_generation_chunks(world_directory: &Path, generation: u64, chunks: &[DiskChunk]) -> io::Result<()> {
    let published_relative = PathBuf::from(generation_directory_name(generation));
    let staging_relative = PathBuf::from(staging_generation_directory_name(generation));
    let published = checked_directory_slot(world_directory, published_relative.as_path())?;
    let staging = checked_directory_slot(world_directory, staging_relative.as_path())?;
    if published.exists() || staging.exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("chunk generation {generation} already has a storage slot")));
    }
    fs::create_dir(staging.as_path())?;
    if let Err(error) = write_generation_chunks_to_staging(staging.as_path(), chunks) {
        if fs::symlink_metadata(staging.as_path()).is_ok_and(|metadata| metadata.is_dir()) {
            let _ = fs::remove_dir_all(staging.as_path());
        }
        return Err(error);
    }
    fs::rename(staging, published)?;
    sync_directory(world_directory)
}


pub(crate) fn load_generation_chunks(
    world_directory: &Path,
    generation: u64,
) -> io::Result<Vec<DiskChunk>> {
    let generation_directory = checked_directory_slot(
        world_directory,
        Path::new(&generation_directory_name(generation)),
    )?;
    let metadata = fs::symlink_metadata(&generation_directory)?;
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "chunk generation storage must be a real directory",
        ));
    }

    let chunks_root = generation_directory.join(CHUNK_DIRECTORY);
    let chunks_metadata = match fs::symlink_metadata(&chunks_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    if !chunks_metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "chunk storage root must be a real directory",
        ));
    }

    let mut chunks = Vec::new();
    for x_entry in fs::read_dir(&chunks_root)? {
        let x_entry = x_entry?;
        if !x_entry.file_type()?.is_dir() {
            return Err(noncanonical_chunk_path(x_entry.path()));
        }
        let x = parse_canonical_i32(&x_entry.file_name())?;

        for y_entry in fs::read_dir(x_entry.path())? {
            let y_entry = y_entry?;
            if !y_entry.file_type()?.is_dir() {
                return Err(noncanonical_chunk_path(y_entry.path()));
            }
            let y = parse_canonical_i32(&y_entry.file_name())?;
            if y < 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "persisted chunk Y cannot be negative",
                ));
            }

            for z_entry in fs::read_dir(y_entry.path())? {
                let z_entry = z_entry?;
                if !z_entry.file_type()?.is_file() {
                    return Err(noncanonical_chunk_path(z_entry.path()));
                }
                let z = parse_chunk_file_z(&z_entry.file_name())?;
                let expected = IVec3::new(x, y, z);
                let file = fs::File::open(z_entry.path())?;
                if !file.metadata()?.is_file() {
                    return Err(noncanonical_chunk_path(z_entry.path()));
                }
                let chunk: DiskChunk =
                    serde_json::from_reader(io::BufReader::new(file)).map_err(io::Error::other)?;
                let actual = ChunkDiskIdentity::from_disk_chunk(&chunk)?.chunk_position();
                if actual != expected {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "persisted chunk identity does not match canonical path: expected {expected:?}, found {actual:?}"
                        ),
                    ));
                }
                chunks.push((expected, chunk));
            }
        }
    }

    chunks.sort_unstable_by_key(|(coord, _)| (coord.y, coord.z, coord.x));
    Ok(chunks.into_iter().map(|(_, chunk)| chunk).collect())
}

fn parse_canonical_i32(value: &std::ffi::OsStr) -> io::Result<i32> {
    let text = value
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "chunk path is not UTF-8"))?;
    let parsed = text.parse::<i32>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid chunk coordinate component: {text}"),
        )
    })?;
    if parsed.to_string() != text {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("noncanonical chunk coordinate component: {text}"),
        ));
    }
    Ok(parsed)
}

fn parse_chunk_file_z(value: &std::ffi::OsStr) -> io::Result<i32> {
    let text = value
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "chunk filename is not UTF-8"))?;
    let suffix = format!(".{CHUNK_FILE_EXTENSION}");
    let z = text.strip_suffix(&suffix).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("noncanonical chunk filename: {text}"),
        )
    })?;
    parse_canonical_i32(std::ffi::OsStr::new(z))
}

fn noncanonical_chunk_path(path: PathBuf) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("noncanonical chunk storage path: {}", path.display()),
    )
}

fn write_generation_chunks_to_staging(staging: &Path, chunks: &[DiskChunk]) -> io::Result<()> {
    let mut identities = HashSet::with_capacity(chunks.len());
    for chunk in chunks {
        let identity = ChunkDiskIdentity::from_disk_chunk(chunk)?;
        if !identities.insert(identity) {
            let position = identity.chunk_position();
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("duplicate persisted chunk coordinate: {position:?}")));
        }
        let path = staging.join(identity.relative_path());
        let parent = path.parent().ok_or(io::Error::other("chunk storage path has no parent"))?;
        fs::create_dir_all(parent)?;
        let payload = serde_json::to_vec(chunk).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(&payload)?;
        file.sync_all()?;
    }
    sync_directory_tree(staging)
}

pub(crate) fn remove_generation_chunks(world_directory: &Path, generation: u64) -> io::Result<()> {
    let relative = PathBuf::from(generation_directory_name(generation));
    let path = checked_directory_slot(world_directory, relative.as_path())?;
    match fs::symlink_metadata(path.as_path()) {
        Ok(metadata) if metadata.is_dir() => { fs::remove_dir_all(path)?; sync_directory(world_directory) }
        Ok(_) => Err(io::Error::new(io::ErrorKind::InvalidData, "chunk generation storage must be a real directory")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn sync_directory_tree(directory: &Path) -> io::Result<()> {
    let mut directories = vec![directory.to_path_buf()];
    let mut index = 0;
    while index < directories.len() {
        let current = directories[index].clone(); index += 1;
        for entry in fs::read_dir(current.as_path())? { let entry = entry?; if entry.file_type()?.is_dir() { directories.push(entry.path()); } }
    }
    for directory in directories.into_iter().rev() { sync_directory(directory.as_path())?; }
    Ok(())
}
fn sync_directory(directory: &Path) -> io::Result<()> { fs::File::open(directory)?.sync_all() }

#[cfg(test)]
mod tests {
    use super::*;
    fn temp_directory(label: &str) -> PathBuf {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("clock must be after epoch").as_nanos();
        let unique = format!("asteria-{label}-{pid}-{nanos}");
        std::env::temp_dir().join(unique)
    }
    fn disk_chunk(position: IVec3) -> DiskChunk {
        serde_json::from_value(serde_json::json!({"coord": [position.x, position.y, position.z]}))
            .expect("minimal disk chunk fixture must decode")
    }
    #[test]
    fn chunk_identity_is_stable_across_disk_conversion() {
        let chunk = disk_chunk(IVec3::new(-7, 3, 12)); let identity = ChunkDiskIdentity::from_disk_chunk(&chunk).expect("disk chunk identity must be valid");
        assert_eq!(identity.chunk_position(), IVec3::new(-7, 3, 12)); assert_eq!(identity, ChunkDiskIdentity::new(IVec3::new(-7, 3, 12)));
    }
    #[test]
    fn chunk_identity_maps_to_canonical_relative_path() {
        let identity = ChunkDiskIdentity::new(IVec3::new(-7, 3, 12));
        assert_eq!(identity.relative_path(), PathBuf::from("chunks").join("-7").join("3").join("12.chunk.json"));
    }
    #[test]
    fn generation_directory_identity_is_canonical() { assert_eq!(generation_directory_name(42), "generation-42"); }
    #[test]
    fn staging_generation_directory_identity_is_distinct() { assert_eq!(staging_generation_directory_name(42), ".generation-42.tmp"); assert_ne!(staging_generation_directory_name(42), generation_directory_name(42)); }
    #[test]
    fn directory_slot_rejects_regular_files() {
        let root = temp_directory("chunk-slot-file"); fs::create_dir_all(root.as_path()).expect("temp root must be created"); let slot = PathBuf::from(generation_directory_name(5)); fs::write(root.join(slot.as_path()), b"not a directory").expect("fixture file must be written");
        let error = checked_directory_slot(root.as_path(), slot.as_path()).expect_err("file slot must be rejected"); assert_eq!(error.kind(), io::ErrorKind::InvalidData); fs::remove_dir_all(root).expect("temp root must be removed");
    }
    #[test]
    fn publishes_generation_through_private_staging_directory() {
        let root = temp_directory("chunk-publish"); fs::create_dir_all(root.as_path()).expect("temp root must be created");
        let chunks = [disk_chunk(IVec3::new(-1, 0, 2)), disk_chunk(IVec3::new(3, 4, -5))];
        publish_generation_chunks(root.as_path(), 9, &chunks).expect("generation must publish"); assert!(!root.join(staging_generation_directory_name(9)).exists());
        for chunk in &chunks { let identity = ChunkDiskIdentity::from_disk_chunk(chunk).expect("fixture identity must be valid"); let path = root.join(generation_directory_name(9)).join(identity.relative_path()); assert!(path.is_file()); let decoded: DiskChunk = serde_json::from_slice(&fs::read(path).expect("published chunk must be readable")).expect("published chunk must decode"); assert_eq!(ChunkDiskIdentity::from_disk_chunk(&decoded).expect("published identity must be valid"), identity); }
        let loaded = load_generation_chunks(root.as_path(), 9).expect("published generation must load");
        let loaded_positions = loaded.iter().map(|chunk| ChunkDiskIdentity::from_disk_chunk(chunk).expect("loaded identity must be valid").chunk_position()).collect::<Vec<_>>();
        assert_eq!(loaded_positions, vec![IVec3::new(-1, 0, 2), IVec3::new(3, 4, -5)]);
        remove_generation_chunks(root.as_path(), 9).expect("generation must be removable"); assert!(!root.join(generation_directory_name(9)).exists()); fs::remove_dir_all(root).expect("temp root must be removed");
    }
    #[test]
    fn empty_generation_round_trips_without_chunk_directory() {
        let root = temp_directory("chunk-empty");
        fs::create_dir_all(root.as_path()).expect("temp root must be created");
        publish_generation_chunks(root.as_path(), 12, &[]).expect("empty generation must publish");
        assert!(load_generation_chunks(root.as_path(), 12).expect("empty generation must load").is_empty());
        fs::remove_dir_all(root).expect("temp root must be removed");
    }

    #[test]
    fn load_rejects_chunk_whose_payload_identity_disagrees_with_path() {
        let root = temp_directory("chunk-mismatch");
        fs::create_dir_all(root.as_path()).expect("temp root must be created");
        publish_generation_chunks(root.as_path(), 13, &[disk_chunk(IVec3::new(1, 2, 3))])
            .expect("fixture generation must publish");
        let canonical = root
            .join(generation_directory_name(13))
            .join(ChunkDiskIdentity::new(IVec3::new(1, 2, 3)).relative_path());
        fs::write(
            canonical,
            serde_json::to_vec(&disk_chunk(IVec3::new(9, 2, 3)))
                .expect("mismatched fixture must serialize"),
        )
        .expect("mismatched fixture must overwrite");
        let error = load_generation_chunks(root.as_path(), 13)
            .expect_err("mismatched path identity must be rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        fs::remove_dir_all(root).expect("temp root must be removed");
    }

    #[test]
    fn duplicate_chunk_coordinates_abort_without_publishing() {
        let root = temp_directory("chunk-duplicate"); fs::create_dir_all(root.as_path()).expect("temp root must be created");
        let chunks = [disk_chunk(IVec3::new(1, 2, 3)), disk_chunk(IVec3::new(1, 2, 3))];
        let error = publish_generation_chunks(root.as_path(), 11, &chunks).expect_err("duplicate coordinates must be rejected"); assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(!root.join(generation_directory_name(11)).exists()); assert!(!root.join(staging_generation_directory_name(11)).exists()); fs::remove_dir_all(root).expect("temp root must be removed");
    }
}