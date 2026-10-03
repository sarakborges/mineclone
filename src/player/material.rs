use std::path::Path;

use bevy::prelude::*;

const PLAYER_SKIN_TEXTURE_PATHS: [&str; 4] = [
    "textures/player/512.png",
    "textures/player/256.png",
    "textures/player/128.png",
    "textures/player/64.png",
];

pub(crate) fn apply_player_skin_material(
    material: &mut StandardMaterial,
    asset_server: &AssetServer,
) {
    material.base_color = Color::WHITE;
    material.base_color_texture = Some(asset_server.load(player_skin_texture_path()));
    material.unlit = true;
    material.metallic = 0.0;
    material.perceptual_roughness = 1.0;
}

pub(super) fn player_skin_texture_path() -> &'static str {
    select_player_skin_texture_path(|path| Path::new("assets").join(path).is_file()).unwrap_or_else(
        || {
            panic!(
                "player skin texture not found; expected one of: {}",
                PLAYER_SKIN_TEXTURE_PATHS.join(", ")
            )
        },
    )
}

fn select_player_skin_texture_path(mut exists: impl FnMut(&str) -> bool) -> Option<&'static str> {
    PLAYER_SKIN_TEXTURE_PATHS
        .iter()
        .copied()
        .find(|path| exists(path))
}

#[cfg(test)]
mod tests {
    use super::{PLAYER_SKIN_TEXTURE_PATHS, select_player_skin_texture_path};

    #[test]
    fn player_skin_prefers_highest_available_resolution() {
        let available = ["textures/player/256.png", "textures/player/64.png"];

        assert_eq!(
            select_player_skin_texture_path(|path| available.contains(&path)),
            Some("textures/player/256.png")
        );
    }

    #[test]
    fn player_skin_supports_each_authored_resolution() {
        for expected in PLAYER_SKIN_TEXTURE_PATHS {
            assert_eq!(
                select_player_skin_texture_path(|path| path == expected),
                Some(expected)
            );
        }
    }

    #[test]
    fn player_skin_reports_missing_texture() {
        assert_eq!(select_player_skin_texture_path(|_| false), None);
    }
}
