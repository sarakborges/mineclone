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
    pub(crate) fn from_disk_chunk(chunk: &DiskChunk) -> Self {
        Self::new(IVec3::new(chunk.coord[0], chunk.coord[1], chunk.coord[2]))
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

pub(crate) fn read_generation_chunks(world_directory: &Path, generation: u64) -> io::Result<Vec<DiskChunk>> {
    let relative = PathBuf::from(generation_directory_name(generation));
    let generation_directory = checked_directory_slot(world_directory, relative.as_path())?;
    if !generation_directory.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("chunk generation {generation} is missing")));
    }
    let chunks_root = generation_directory.join(CHUNK_DIRECTORY);
    let metadata = fs::symlink_metadata(chunks_root.as_path())?;
    if !metadata.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "chunk generation root is not a real directory"));
    }
    let mut chunks = Vec::new();
    read_chunk_directory(chunks_root.as_path(), &mut chunks)?;
    let mut identities = HashSet::with_capacity(chunks.len());
    for chunk in &chunks {
        if !identities.insert(ChunkDiskIdentity::from_disk_chunk(chunk)) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "chunk generation contains duplicate coordinates"));
        }
    }
    Ok(chunks)
}

fn read_chunk_directory(directory: &Path, chunks: &mut Vec<DiskChunk>) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "chunk storage must not contain symbolic links"));
        }
        if file_type.is_dir() {
            read_chunk_directory(entry.path().as_path(), chunks)?;
            continue;
        }
        if !file_type.is_file() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "chunk storage contains an unsupported entry"));
        }
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "chunk storage contains a non-chunk file"));
        }
        let chunk = serde_json::from_reader(io::BufReader::new(fs::File::open(path)?)).map_err(io::Error::other)?;
        chunks.push(chunk);
    }
    Ok(())
}

fn write_generation_chunks_to_staging(staging: &Path, chunks: &[DiskChunk]) -> io::Result<()> {
    let mut identities = HashSet::with_capacity(chunks.len());
    for chunk in chunks {
        let identity = ChunkDiskIdentity::from_disk_chunk(chunk);
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
        let chunk = disk_chunk(IVec3::new(-7, 3, 12)); let identity = ChunkDiskIdentity::from_disk_chunk(&chunk);
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
        let loaded = read_generation_chunks(root.as_path(), 9).expect("published generation must load");
        assert_eq!(loaded.len(), chunks.len());
        for chunk in &chunks { let identity = ChunkDiskIdentity::from_disk_chunk(chunk); assert!(loaded.iter().any(|loaded| ChunkDiskIdentity::from_disk_chunk(loaded) == identity)); }
        remove_generation_chunks(root.as_path(), 9).expect("generation must be removable"); assert!(!root.join(generation_directory_name(9)).exists()); fs::remove_dir_all(root).expect("temp root must be removed");
    }
    #[test]
    fn duplicate_chunk_coordinates_abort_without_publishing() {
        let root = temp_directory("chunk-duplicate"); fs::create_dir_all(root.as_path()).expect("temp root must be created");
        let chunks = [disk_chunk(IVec3::new(1, 2, 3)), disk_chunk(IVec3::new(1, 2, 3))];
        let error = publish_generation_chunks(root.as_path(), 11, &chunks).expect_err("duplicate coordinates must be rejected"); assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(!root.join(generation_directory_name(11)).exists()); assert!(!root.join(staging_generation_directory_name(11)).exists()); fs::remove_dir_all(root).expect("temp root must be removed");
    }
}