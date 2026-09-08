use bevy::prelude::*;

use crate::player::camera::GameplayCamera;

use super::biome_field::BiomeField;

pub const DEFAULT_BIOME_ID: &str = "mineclone:overworld/plains";

#[derive(Clone)]
pub struct CurrentBiomeInfluence {
    pub id: String,
    pub weight: f32,
}

#[derive(Resource)]
pub struct CurrentBiome {
    pub id: String,
    pub influences: Vec<CurrentBiomeInfluence>,
}

impl Default for CurrentBiome {
    fn default() -> Self {
        Self {
            id: DEFAULT_BIOME_ID.to_owned(),
            influences: vec![CurrentBiomeInfluence {
                id: DEFAULT_BIOME_ID.to_owned(),
                weight: 1.0,
            }],
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

    current_biome.influences.clear();
    current_biome
        .influences
        .extend(sample.influences.into_iter().map(|influence| CurrentBiomeInfluence {
            id: influence.id.to_owned(),
            weight: influence.weight,
        }));
}
