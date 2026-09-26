use bevy::prelude::*;

use super::PLAYER_SKIN_TEXTURE_PATH;

pub(crate) fn apply_player_skin_material(
    material: &mut StandardMaterial,
    asset_server: &AssetServer,
) {
    material.base_color = Color::WHITE;
    material.base_color_texture = Some(asset_server.load(PLAYER_SKIN_TEXTURE_PATH));
    material.unlit = true;
    material.metallic = 0.0;
    material.perceptual_roughness = 1.0;
}
