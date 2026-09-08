use bevy::prelude::*;

use crate::{
    content::biome::BiomeRegistry,
    world::biome::CurrentBiome,
};

#[derive(Component)]
pub(super) struct BiomeHudText;

pub(super) fn update_biome_hud(
    biome: Res<CurrentBiome>,
    biomes: Res<BiomeRegistry>,
    mut biome_text: Single<&mut Text, With<BiomeHudText>>,
) {
    let biome_name = biomes
        .get(&biome.id)
        .map(|definition| definition.name.as_str())
        .unwrap_or(biome.id.as_str());

    biome_text.0 = biome_name.to_string();
}
