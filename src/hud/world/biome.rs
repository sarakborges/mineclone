use bevy::prelude::*;

use crate::{
    content::biome::BiomeRegistry,
    localization::ActiveLanguage,
    world::biome::CurrentBiome,
};

#[derive(Component)]
pub(super) struct BiomeHudText;

pub(super) fn update_biome_hud(
    biome: Res<CurrentBiome>,
    biomes: Res<BiomeRegistry>,
    language: Res<ActiveLanguage>,
    mut biome_text: Single<&mut Text, With<BiomeHudText>>,
) {
    let biome_name = biomes
        .get(&biome.id)
        .map(|definition| definition.name.text(language.get()))
        .unwrap_or(biome.id.as_str());

    if biome_text.0 != biome_name {
        biome_text.0 = biome_name.to_string();
    }
}
