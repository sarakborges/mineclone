use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;

pub fn collect_json_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(directory).unwrap_or_else(|error| {
        panic!(
            "failed to read content directory {}: {error}",
            directory.display()
        )
    });

    for entry in entries {
        let path = entry
            .unwrap_or_else(|error| panic!("failed to read content entry: {error}"))
            .path();

        if path.is_dir() {
            collect_json_files(&path, files);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            files.push(path);
        }
    }
}

pub fn read_json_definition<T: DeserializeOwned>(path: &Path) -> T {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    serde_json::from_str(&source)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}
