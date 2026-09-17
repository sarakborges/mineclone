use std::{collections::HashSet, fs, io, path::Path};

pub(crate) const DEFAULT_WORLD_NAME: &str = "New World";
pub(crate) const WORLDS_DIRECTORY: &str = "worlds";
const MAX_NAME_UTF16_UNITS: usize = 200;
const COPY_PREFIX: &str = "Copy of ";

/// Resolve a user-visible name to a unique, safe directory ID without writing
/// anything to disk. The actual save writer must still use create_dir (rather
/// than overwriting a directory) to protect against changes after this check.
pub(crate) fn available_world_name(requested: &str) -> io::Result<String> {
    let requested = requested.trim();
    validate_world_name(requested)?;

    let mut occupied = HashSet::new();
    match fs::read_dir(WORLDS_DIRECTORY) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry?;
                if let Some(name) = entry.file_name().to_str() {
                    occupied.insert(name.to_lowercase());
                }
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    unique_name(requested, &occupied)
}

pub(crate) fn validate_world_name(name: &str) -> io::Result<()> {
    let invalid = name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with([' ', '.'])
        || name.encode_utf16().count() > MAX_NAME_UTF16_UNITS
        || name.chars().any(|character| {
            character.is_control() || "<>:\"/\\|?*".contains(character)
        });
    let stem = name.split('.').next().unwrap_or(name).trim_end();
    let reserved = matches!(
        stem.to_ascii_uppercase().as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "COM2" | "COM3" | "COM4"
            | "COM5" | "COM6" | "COM7" | "COM8" | "COM9" | "LPT1" | "LPT2"
            | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9"
    );
    if invalid || reserved || Path::new(name).components().count() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Choose a nonempty name without path separators, reserved device names or trailing dots/spaces (maximum 200 characters).",
        ));
    }
    Ok(())
}

fn unique_name(requested: &str, occupied: &HashSet<String>) -> io::Result<String> {
    let mut candidate = requested.to_owned();
    while occupied.contains(&candidate.to_lowercase()) {
        candidate = format!("{COPY_PREFIX}{candidate}");
        validate_world_name(&candidate)?;
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_repeated_collisions() {
        let occupied = ["forest", "copy of forest"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert_eq!(
            unique_name("Forest", &occupied).unwrap(),
            "Copy of Copy of Forest"
        );
    }

    #[test]
    fn rejects_windows_device_names_and_path_traversal() {
        for name in ["CON", "nul.txt", "../world", "foo/bar", "foo\\bar", "test.", " "] {
            assert!(validate_world_name(name).is_err(), "{name}");
        }
    }
}
