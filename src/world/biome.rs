use bevy::prelude::*;

use crate::player::camera::GameplayCamera;

use super::biome_field::{BiomeField, BiomeInfluence};

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
    pub surface_id: String,
    pub surface_influences: Vec<CurrentBiomeInfluence>,
    pub volume_id: Option<String>,
    pub volume_influences: Vec<CurrentBiomeInfluence>,
    pub volume_strength: f32,
}

impl Default for CurrentBiome {
    fn default() -> Self {
        let default_influence = CurrentBiomeInfluence {
            id: DEFAULT_BIOME_ID.to_owned(),
            weight: 1.0,
        };

        Self {
            id: DEFAULT_BIOME_ID.to_owned(),
            influences: vec![default_influence.clone()],
            surface_id: DEFAULT_BIOME_ID.to_owned(),
            surface_influences: vec![default_influence],
            volume_id: None,
            volume_influences: Vec::new(),
            volume_strength: 0.0,
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

    let sample = biome_field.sample_resolved(player.translation);

    current_biome.id = sample.primary_id.to_owned();
    replace_influences(&mut current_biome.influences, &sample.influences);

    current_biome.surface_id = sample.surface.primary_id.to_owned();
    replace_influences(
        &mut current_biome.surface_influences,
        &sample.surface.influences,
    );

    if let Some(volume) = sample.volume {
        current_biome.volume_id = Some(volume.primary_id.to_owned());
        current_biome.volume_strength = volume.strength;
        replace_influences(
            &mut current_biome.volume_influences,
            &volume.influences,
        );
    } else {
        current_biome.volume_id = None;
        current_biome.volume_strength = 0.0;
        current_biome.volume_influences.clear();
    }
}

fn replace_influences(
    target: &mut Vec<CurrentBiomeInfluence>,
    source: &[BiomeInfluence<'_>],
) {
    target.clear();
    target.extend(source.iter().map(|influence| CurrentBiomeInfluence {
        id: influence.id.to_owned(),
        weight: influence.weight,
    }));
}
