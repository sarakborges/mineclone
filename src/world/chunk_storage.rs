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
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => Ok(path),
        Ok(_) => Err(io::Error::new(io::ErrorKind::InvalidData, format!("chunk storage directory slot is not a real directory: {}", path.display()))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(path),
        Err(error) => Err(error),
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ChunkDiskIdentity { chunk_position: IVec3 }
impl ChunkDiskIdentity {
    pub(crate) fn new(chunk_position: IVec3) -> Self { Self { chunk_position } }
    pub(crate) fn from_disk_chunk(chunk: &DiskChunk) -> Self { Self::new(chunk.chunk_position()) }
    pub(crate) fn chunk_position(self) -> IVec3 { self.chunk_position }
    pub(crate) fn relative_path(self) -> PathBuf {
        let position = self.chunk_position;
        PathBuf::from(CHUNK_DIRECTORY).join(position.x.to_string()).join(position.y.to_string()).join(format!("{}.{}", position.z, CHUNK_FILE_EXTENSION))
    }
    pub(crate) fn generation_relative_path(self, generation: u64) -> PathBuf { PathBuf::from(generation_directory_name(generation)).join(self.relative_path()) }
}

pub(crate) fn publish_generation_chunks(world_directory: &Path, generation: u64, chunks: &[DiskChunk]) -> io::Result<()> {
    let published_relative = PathBuf::from(generation_directory_name(generation));
    let staging_relative = PathBuf::from(staging_generation_directory_name(generation));
    let published = checked_directory_slot(world_directory, &published_relative)?;
    let staging = checked_directory_slot(world_directory, &staging_relative)?;
    if published.exists() || staging.exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("chunk generation {generation} already has a storage slot")));
    }
    fs::create_dir(&staging)?;
    if let Err(error) = write_generation_chunks_to_staging(&staging, chunks) {
        if fs::symlink_metadata(&staging).is_ok_and(|metadata| metadata.file_type().is_dir() && !metadata.file_type().is_symlink()) {
            let _ = fs::remove_dir_all(&staging);
        }
        return Err(error);
    }
    fs::rename(staging, published)?;
    sync_directory(world_directory)
}

fn write_generation_chunks_to_staging(staging: &Path, chunks: &[DiskChunk]) -> io::Result<()> {
    let mut identities = HashSet::with_capacity(chunks.len());
    for chunk in chunks {
        let identity = ChunkDiskIdentity::from_disk_chunk(chunk);
        if !identities.insert(identity) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("duplicate persisted chunk coordinate: {:?}", identity.chunk_position())));
        }
        let path = staging.join(identity.relative_path());
        let parent = path.parent().ok_or_else(|| io::Error::other("chunk storage path has no parent"))?;
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
    let path = checked_directory_slot(world_directory, &relative)?;
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => { fs::remove_dir_all(path)?; sync_directory(world_directory) }
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
        for entry in fs::read_dir(&current)? { let entry = entry?; if entry.file_type()?.is_dir() { directories.push(entry.path()); } }
    }
    for directory in directories.into_iter().rev() { sync_directory(&directory)?; }
    Ok(())
}
fn sync_directory(directory: &Path) -> io::Result<()> { fs::File::open(directory)?.sync_all() }

#[cfg(test)]
mod tests {
    use super::*;
    fn temp_directory(label: &str) -> PathBuf {
        let unique = format!("asteria-{label}-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("clock must be after epoch").as_nanos());
        std::env::temp_dir().join(unique)
    }
    #[test]
    fn chunk_identity_is_stable_across_disk_conversion() {
        let chunk = DiskChunk::new(IVec3::new(-7, 3, 12), Vec::new()); let identity = ChunkDiskIdentity::from_disk_chunk(&chunk);
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
    fn chunk_identity_scopes_path_to_save_generation() {
        let identity = ChunkDiskIdentity::new(IVec3::new(-7, 3, 12));
        assert_eq!(identity.generation_relative_path(42), PathBuf::from("generation-42").join("chunks").join("-7").join("3").join("12.chunk.json"));
    }
    #[test]
    fn directory_slot_rejects_regular_files() {
        let root = temp_directory("chunk-slot-file"); fs::create_dir_all(&root).expect("temp root must be created"); let slot = PathBuf::from(generation_directory_name(5)); fs::write(root.join(&slot), b"not a directory").expect("fixture file must be written");
        let error = checked_directory_slot(&root, &slot).expect_err("file slot must be rejected"); assert_eq!(error.kind(), io::ErrorKind::InvalidData); fs::remove_dir_all(root).expect("temp root must be removed");
    }
    #[test]
    fn publishes_generation_through_private_staging_directory() {
        let root = temp_directory("chunk-publish"); fs::create_dir_all(&root).expect("temp root must be created");
        let chunks = [DiskChunk::new(IVec3::new(-1, 0, 2), Vec::new()), DiskChunk::new(IVec3::new(3, 4, -5), Vec::new())];
        publish_generation_chunks(&root, 9, &chunks).expect("generation must publish"); assert!(!root.join(staging_generation_directory_name(9)).exists());
        for chunk in &chunks { let identity = ChunkDiskIdentity::from_disk_chunk(chunk); let path = root.join(identity.generation_relative_path(9)); assert!(path.is_file()); let decoded: DiskChunk = serde_json::from_slice(&fs::read(path).expect("published chunk must be readable")).expect("published chunk must decode"); assert_eq!(decoded.chunk_position(), chunk.chunk_position()); }
        remove_generation_chunks(&root, 9).expect("generation must be removable"); assert!(!root.join(generation_directory_name(9)).exists()); fs::remove_dir_all(root).expect("temp root must be removed");
    }
    #[test]
    fn duplicate_chunk_coordinates_abort_without_publishing() {
        let root = temp_directory("chunk-duplicate"); fs::create_dir_all(&root).expect("temp root must be created");
        let chunks = [DiskChunk::new(IVec3::new(1, 2, 3), Vec::new()), DiskChunk::new(IVec3::new(1, 2, 3), Vec::new())];
        let error = publish_generation_chunks(&root, 11, &chunks).expect_err("duplicate coordinates must be rejected"); assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(!root.join(generation_directory_name(11)).exists()); assert!(!root.join(staging_generation_directory_name(11)).exists()); fs::remove_dir_all(root).expect("temp root must be removed");
    }
}
