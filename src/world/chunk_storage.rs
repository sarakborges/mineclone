use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use bevy::prelude::IVec3;
use serde::{Deserialize, Serialize};

use crate::{
    voxel::chunk_disk::DiskChunk,
    world::storage_durability::{sync_directory, sync_directory_tree},
};

const LEGACY_CHUNK_DIRECTORY: &str = "chunks";
const REGION_DIRECTORY: &str = "regions";
const CHUNK_FILE_EXTENSION: &str = "chunk.json";
const REGION_FILE_EXTENSION: &str = "region.json";
const REGION_SIZE_CHUNKS: i32 = 8;
const MAX_REGION_FILE_BYTES: u64 = 256 * 1024 * 1024;

pub(crate) fn generation_directory_name(generation: u64) -> String {
    format!("generation-{generation}")
}

pub(crate) fn staging_generation_directory_name(generation: u64) -> String {
    format!(".generation-{generation}.tmp")
}

fn checked_directory_slot_exists(world_directory: &Path, relative: &Path) -> io::Result<bool> {
    let path = checked_directory_slot(world_directory, relative)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.is_dir()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

pub(crate) fn generation_storage_slot_exists(
    world_directory: &Path,
    generation: u64,
) -> io::Result<bool> {
    Ok(
        checked_directory_slot_exists(
            world_directory,
            Path::new(&generation_directory_name(generation)),
        )? || checked_directory_slot_exists(
            world_directory,
            Path::new(&staging_generation_directory_name(generation)),
        )?,
    )
}

pub(crate) fn generation_chunks_published(
    world_directory: &Path,
    generation: u64,
) -> io::Result<bool> {
    checked_directory_slot_exists(
        world_directory,
        Path::new(&generation_directory_name(generation)),
    )
}

fn checked_directory_slot(world_directory: &Path, relative: &Path) -> io::Result<PathBuf> {
    let path = world_directory.join(relative);
    match fs::symlink_metadata(path.as_path()) {
        Ok(metadata) if metadata.is_dir() => Ok(path),
        Ok(_) => {
            let display = path.display();
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("chunk storage directory slot is not a real directory: {display}"),
            ))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(path),
        Err(error) => Err(error),
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ChunkDiskIdentity {
    chunk_position: IVec3,
}

impl ChunkDiskIdentity {
    pub(crate) fn new(chunk_position: IVec3) -> Self {
        Self { chunk_position }
    }

    pub(crate) fn from_disk_chunk(chunk: &DiskChunk) -> io::Result<Self> {
        Ok(Self::new(chunk.coord()?))
    }

    pub(crate) fn chunk_position(self) -> IVec3 {
        self.chunk_position
    }

    #[cfg(test)]
    fn legacy_relative_path(self) -> PathBuf {
        let position = self.chunk_position;
        let z = position.z;
        PathBuf::from(LEGACY_CHUNK_DIRECTORY)
            .join(position.x.to_string())
            .join(position.y.to_string())
            .join(format!("{z}.{CHUNK_FILE_EXTENSION}"))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ChunkRegionIdentity {
    region_position: IVec3,
}

impl ChunkRegionIdentity {
    fn from_chunk_position(chunk_position: IVec3) -> Self {
        Self {
            region_position: IVec3::new(
                chunk_position.x.div_euclid(REGION_SIZE_CHUNKS),
                chunk_position.y.div_euclid(REGION_SIZE_CHUNKS),
                chunk_position.z.div_euclid(REGION_SIZE_CHUNKS),
            ),
        }
    }

    fn from_region_position(region_position: IVec3) -> io::Result<Self> {
        if region_position.y < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "persisted region Y cannot be negative",
            ));
        }
        Ok(Self { region_position })
    }

    fn relative_path(self) -> PathBuf {
        let position = self.region_position;
        PathBuf::from(REGION_DIRECTORY).join(format!(
            "{}_{}_{}.{}",
            position.x, position.y, position.z, REGION_FILE_EXTENSION
        ))
    }

    fn contains(self, chunk_position: IVec3) -> bool {
        Self::from_chunk_position(chunk_position) == self
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredChunkRegion {
    region: [i32; 3],
    chunks: Vec<DiskChunk>,
}

#[derive(Serialize)]
struct DiskChunkRegionRef<'a> {
    region: [i32; 3],
    chunks: Vec<&'a DiskChunk>,
}

pub(crate) fn publish_generation_chunks(
    world_directory: &Path,
    generation: u64,
    chunks: &[DiskChunk],
) -> io::Result<()> {
    let published_relative = PathBuf::from(generation_directory_name(generation));
    let staging_relative = PathBuf::from(staging_generation_directory_name(generation));
    let published = checked_directory_slot(world_directory, published_relative.as_path())?;
    let staging = checked_directory_slot(world_directory, staging_relative.as_path())?;
    if published.exists() || staging.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("chunk generation {generation} already has a storage slot"),
        ));
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

    let regions_root = generation_directory.join(REGION_DIRECTORY);
    let chunks_root = generation_directory.join(LEGACY_CHUNK_DIRECTORY);
    let has_regions = optional_real_directory(&regions_root, "chunk region storage root")?;
    let has_legacy = optional_real_directory(&chunks_root, "legacy chunk storage root")?;

    match (has_regions, has_legacy) {
        (true, true) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "chunk generation mixes region and legacy storage layouts",
        )),
        (true, false) => load_region_chunks(&regions_root),
        (false, true) => load_legacy_chunks(&chunks_root),
        (false, false) => Ok(Vec::new()),
    }
}

fn optional_real_directory(path: &Path, label: &str) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(true),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} must be a real directory"),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn load_region_chunks(regions_root: &Path) -> io::Result<Vec<DiskChunk>> {
    let mut chunks = Vec::new();
    let mut identities = HashSet::new();

    for entry in fs::read_dir(regions_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            return Err(noncanonical_region_path(entry.path()));
        }

        let expected = parse_region_file_name(&entry.file_name())?;
        let metadata = entry.metadata()?;
        if !metadata.is_file() || metadata.len() > MAX_REGION_FILE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "chunk region file is invalid or exceeds supported size: {}",
                    entry.path().display()
                ),
            ));
        }

        let file = fs::File::open(entry.path())?;
        let stored: StoredChunkRegion =
            serde_json::from_reader(io::BufReader::new(file)).map_err(io::Error::other)?;
        let actual = ChunkRegionIdentity::from_region_position(IVec3::new(
            stored.region[0],
            stored.region[1],
            stored.region[2],
        ))?;
        if actual != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "persisted region identity does not match canonical path: expected {:?}, found {:?}",
                    expected.region_position, actual.region_position
                ),
            ));
        }

        for chunk in stored.chunks {
            let identity = ChunkDiskIdentity::from_disk_chunk(&chunk)?;
            let position = identity.chunk_position();
            if !expected.contains(position) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "persisted chunk {position:?} is outside region {:?}",
                        expected.region_position
                    ),
                ));
            }
            if !identities.insert(identity) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("duplicate persisted chunk coordinate: {position:?}"),
                ));
            }
            chunks.push((position, chunk));
        }
    }

    chunks.sort_unstable_by_key(|(coord, _)| (coord.y, coord.z, coord.x));
    Ok(chunks.into_iter().map(|(_, chunk)| chunk).collect())
}

fn load_legacy_chunks(chunks_root: &Path) -> io::Result<Vec<DiskChunk>> {
    let mut chunks = Vec::new();

    for x_entry in fs::read_dir(chunks_root)? {
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

fn parse_region_file_name(value: &std::ffi::OsStr) -> io::Result<ChunkRegionIdentity> {
    let text = value
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "region filename is not UTF-8"))?;
    let suffix = format!(".{REGION_FILE_EXTENSION}");
    let coordinates = text.strip_suffix(&suffix).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("noncanonical region filename: {text}"),
        )
    })?;
    let mut parts = coordinates.split('_');
    let x = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "region X is missing"))?;
    let y = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "region Y is missing"))?;
    let z = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "region Z is missing"))?;
    if parts.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("noncanonical region filename: {text}"),
        ));
    }

    ChunkRegionIdentity::from_region_position(IVec3::new(
        parse_canonical_i32(std::ffi::OsStr::new(x))?,
        parse_canonical_i32(std::ffi::OsStr::new(y))?,
        parse_canonical_i32(std::ffi::OsStr::new(z))?,
    ))
}

fn noncanonical_chunk_path(path: PathBuf) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("noncanonical chunk storage path: {}", path.display()),
    )
}

fn noncanonical_region_path(path: PathBuf) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("noncanonical chunk region path: {}", path.display()),
    )
}

fn write_generation_chunks_to_staging(staging: &Path, chunks: &[DiskChunk]) -> io::Result<()> {
    let mut identities = HashSet::with_capacity(chunks.len());
    let mut regions = HashMap::<ChunkRegionIdentity, Vec<(IVec3, &DiskChunk)>>::new();

    for chunk in chunks {
        let identity = ChunkDiskIdentity::from_disk_chunk(chunk)?;
        if !identities.insert(identity) {
            let position = identity.chunk_position();
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("duplicate persisted chunk coordinate: {position:?}"),
            ));
        }

        let position = identity.chunk_position();
        regions
            .entry(ChunkRegionIdentity::from_chunk_position(position))
            .or_default()
            .push((position, chunk));
    }

    let mut regions = regions.into_iter().collect::<Vec<_>>();
    regions.sort_unstable_by_key(|(identity, _)| {
        let position = identity.region_position;
        (position.y, position.z, position.x)
    });

    for (identity, mut region_chunks) in regions {
        region_chunks.sort_unstable_by_key(|(coord, _)| (coord.y, coord.z, coord.x));
        let path = staging.join(identity.relative_path());
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("chunk region storage path has no parent"))?;
        fs::create_dir_all(parent)?;

        let region = identity.region_position;
        let payload = DiskChunkRegionRef {
            region: [region.x, region.y, region.z],
            chunks: region_chunks.into_iter().map(|(_, chunk)| chunk).collect(),
        };

        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        {
            let mut buffered = io::BufWriter::new(&mut file);
            serde_json::to_writer(&mut buffered, &payload)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            buffered.flush()?;
        }
        file.sync_all()?;
    }

    sync_directory_tree(staging)
}

pub(crate) fn remove_generation_chunks(
    world_directory: &Path,
    generation: u64,
) -> io::Result<()> {
    let relative = PathBuf::from(generation_directory_name(generation));
    let path = checked_directory_slot(world_directory, relative.as_path())?;
    match fs::symlink_metadata(path.as_path()) {
        Ok(metadata) if metadata.is_dir() => {
            fs::remove_dir_all(path)?;
            sync_directory(world_directory)
        }
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "chunk generation storage must be a real directory",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_directory(label: &str) -> PathBuf {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock must be after epoch")
            .as_nanos();
        let unique = format!("asteria-{label}-{pid}-{nanos}");
        std::env::temp_dir().join(unique)
    }

    fn disk_chunk(position: IVec3) -> DiskChunk {
        serde_json::from_value(serde_json::json!({
            "coord": [position.x, position.y, position.z]
        }))
        .expect("minimal disk chunk fixture must decode")
    }

    #[test]
    fn chunk_identity_is_stable_across_disk_conversion() {
        let chunk = disk_chunk(IVec3::new(-7, 3, 12));
        let identity = ChunkDiskIdentity::from_disk_chunk(&chunk)
            .expect("disk chunk identity must be valid");
        assert_eq!(identity.chunk_position(), IVec3::new(-7, 3, 12));
        assert_eq!(
            identity,
            ChunkDiskIdentity::new(IVec3::new(-7, 3, 12))
        );
    }

    #[test]
    fn legacy_chunk_identity_maps_to_canonical_relative_path() {
        let identity = ChunkDiskIdentity::new(IVec3::new(-7, 3, 12));
        assert_eq!(
            identity.legacy_relative_path(),
            PathBuf::from("chunks")
                .join("-7")
                .join("3")
                .join("12.chunk.json")
        );
    }

    #[test]
    fn region_identity_uses_euclidean_chunk_groups() {
        assert_eq!(
            ChunkRegionIdentity::from_chunk_position(IVec3::new(7, 7, 7)).region_position,
            IVec3::ZERO
        );
        assert_eq!(
            ChunkRegionIdentity::from_chunk_position(IVec3::new(8, 8, -1)).region_position,
            IVec3::new(1, 1, -1)
        );
    }

    #[test]
    fn generation_directory_identity_is_canonical() {
        assert_eq!(generation_directory_name(42), "generation-42");
    }

    #[test]
    fn staging_generation_directory_identity_is_distinct() {
        assert_eq!(
            staging_generation_directory_name(42),
            ".generation-42.tmp"
        );
        assert_ne!(
            staging_generation_directory_name(42),
            generation_directory_name(42)
        );
    }

    #[test]
    fn directory_slot_rejects_regular_files() {
        let root = temp_directory("chunk-slot-file");
        fs::create_dir_all(root.as_path()).expect("temp root must be created");
        let slot = PathBuf::from(generation_directory_name(5));
        fs::write(root.join(slot.as_path()), b"not a directory")
            .expect("fixture file must be written");
        let error = checked_directory_slot(root.as_path(), slot.as_path())
            .expect_err("file slot must be rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        fs::remove_dir_all(root).expect("temp root must be removed");
    }

    #[test]
    fn publishes_generation_in_region_files() {
        let root = temp_directory("chunk-publish");
        fs::create_dir_all(root.as_path()).expect("temp root must be created");
        let chunks = [
            disk_chunk(IVec3::new(-1, 0, 2)),
            disk_chunk(IVec3::new(3, 4, -5)),
            disk_chunk(IVec3::new(4, 4, -6)),
        ];

        publish_generation_chunks(root.as_path(), 9, &chunks).expect("generation must publish");
        assert!(!root.join(staging_generation_directory_name(9)).exists());

        let generation = root.join(generation_directory_name(9));
        assert!(!generation.join(LEGACY_CHUNK_DIRECTORY).exists());
        let regions = generation.join(REGION_DIRECTORY);
        assert!(regions.is_dir());
        assert_eq!(
            fs::read_dir(&regions)
                .expect("regions must be readable")
                .count(),
            2
        );

        let loaded =
            load_generation_chunks(root.as_path(), 9).expect("published generation must load");
        let loaded_positions = loaded
            .iter()
            .map(|chunk| {
                ChunkDiskIdentity::from_disk_chunk(chunk)
                    .expect("loaded identity must be valid")
                    .chunk_position()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            loaded_positions,
            vec![
                IVec3::new(-1, 0, 2),
                IVec3::new(4, 4, -6),
                IVec3::new(3, 4, -5),
            ]
        );

        remove_generation_chunks(root.as_path(), 9).expect("generation must be removable");
        assert!(!root.join(generation_directory_name(9)).exists());
        fs::remove_dir_all(root).expect("temp root must be removed");
    }

    #[test]
    fn empty_generation_round_trips_without_storage_directory() {
        let root = temp_directory("chunk-empty");
        fs::create_dir_all(root.as_path()).expect("temp root must be created");
        publish_generation_chunks(root.as_path(), 12, &[])
            .expect("empty generation must publish");
        assert!(
            load_generation_chunks(root.as_path(), 12)
                .expect("empty generation must load")
                .is_empty()
        );
        fs::remove_dir_all(root).expect("temp root must be removed");
    }

    #[test]
    fn load_reads_legacy_per_chunk_layout() {
        let root = temp_directory("chunk-legacy");
        fs::create_dir_all(root.as_path()).expect("temp root must be created");
        let generation = root.join(generation_directory_name(14));
        let chunk = disk_chunk(IVec3::new(-7, 3, 12));
        let path = generation.join(
            ChunkDiskIdentity::new(IVec3::new(-7, 3, 12)).legacy_relative_path(),
        );
        fs::create_dir_all(path.parent().expect("legacy chunk must have a parent"))
            .expect("legacy parent must be created");
        fs::write(
            &path,
            serde_json::to_vec(&chunk).expect("legacy chunk must serialize"),
        )
        .expect("legacy chunk must be written");

        let loaded =
            load_generation_chunks(root.as_path(), 14).expect("legacy generation must load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(
            ChunkDiskIdentity::from_disk_chunk(&loaded[0])
                .expect("legacy identity must remain valid")
                .chunk_position(),
            IVec3::new(-7, 3, 12)
        );

        fs::remove_dir_all(root).expect("temp root must be removed");
    }

    #[test]
    fn load_rejects_chunk_outside_region_payload() {
        let root = temp_directory("chunk-mismatch");
        fs::create_dir_all(root.as_path()).expect("temp root must be created");
        publish_generation_chunks(root.as_path(), 13, &[disk_chunk(IVec3::new(1, 2, 3))])
            .expect("fixture generation must publish");

        let region = ChunkRegionIdentity::from_chunk_position(IVec3::new(1, 2, 3));
        let canonical = root
            .join(generation_directory_name(13))
            .join(region.relative_path());
        fs::write(
            canonical,
            serde_json::to_vec(&serde_json::json!({
                "region": [0, 0, 0],
                "chunks": [{"coord": [9, 2, 3]}]
            }))
            .expect("mismatched fixture must serialize"),
        )
        .expect("mismatched fixture must overwrite");

        let error = load_generation_chunks(root.as_path(), 13)
            .expect_err("out-of-region payload must be rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        fs::remove_dir_all(root).expect("temp root must be removed");
    }

    #[test]
    fn load_rejects_mixed_region_and_legacy_layouts() {
        let root = temp_directory("chunk-mixed");
        fs::create_dir_all(root.as_path()).expect("temp root must be created");
        publish_generation_chunks(root.as_path(), 15, &[disk_chunk(IVec3::ZERO)])
            .expect("fixture generation must publish");

        let legacy = root
            .join(generation_directory_name(15))
            .join(ChunkDiskIdentity::new(IVec3::ZERO).legacy_relative_path());
        fs::create_dir_all(legacy.parent().expect("legacy chunk must have a parent"))
            .expect("legacy parent must be created");
        fs::write(
            legacy,
            serde_json::to_vec(&disk_chunk(IVec3::ZERO))
                .expect("legacy chunk must serialize"),
        )
        .expect("legacy chunk must be written");

        let error = load_generation_chunks(root.as_path(), 15)
            .expect_err("mixed layouts must be rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        fs::remove_dir_all(root).expect("temp root must be removed");
    }

    #[test]
    fn duplicate_chunk_coordinates_abort_without_publishing() {
        let root = temp_directory("chunk-duplicate");
        fs::create_dir_all(root.as_path()).expect("temp root must be created");
        let chunks = [
            disk_chunk(IVec3::new(1, 2, 3)),
            disk_chunk(IVec3::new(1, 2, 3)),
        ];
        let error = publish_generation_chunks(root.as_path(), 11, &chunks)
            .expect_err("duplicate coordinates must be rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(!root.join(generation_directory_name(11)).exists());
        assert!(!root.join(staging_generation_directory_name(11)).exists());
        fs::remove_dir_all(root).expect("temp root must be removed");
    }
}
