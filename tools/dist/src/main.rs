mod build_game;
mod copy_tree;
mod package;

use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let project_root = project_root();
    let executable = build_game::build_release(&project_root)?;
    let build_directory = package::create_distribution(&project_root, &executable)?;

    println!("Asteria build created at {}", build_directory.display());
    Ok(())
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|tools| tools.parent())
        .expect("distribution tool should live under tools/dist")
        .to_path_buf()
}
