use bevy::prelude::*;

use crate::player::camera::GameplayCamera;

use super::biome_field::BiomeField;

pub const DEFAULT_BIOME_ID: &str = "mineclone:overworld/plains";

#[derive(Resource)]
pub struct CurrentBiome {
    pub id: String,
    pub secondary_id: String,
    pub secondary_weight: f32,
}

impl Default for CurrentBiome {
    fn default() -> Self {
        Self {
            id: DEFAULT_BIOME_ID.to_owned(),
            secondary_id: DEFAULT_BIOME_ID.to_owned(),
            secondary_weight: 0.0,
        }
    }
}

pub fn track_current_biome(
    player: Single<&Transform, With<GameplayCamera>>,
    biome_field: Option<Res<BiomeField>>,
    mut current_biome: ResMut<CurrentBiome>,
) {
    let Some(biome_field) = biome_field else {
        return;
    };

    let position = Vec2::new(player.translation.x, player.translation.z);
    let sample = biome_field.sample(position);

    if current_biome.id != sample.primary_id {
        current_biome.id = sample.primary_id.to_owned();
    }
    if current_biome.secondary_id != sample.secondary_id {
        current_biome.secondary_id = sample.secondary_id.to_owned();
    }
    current_biome.secondary_weight = sample.secondary_weight;
}
