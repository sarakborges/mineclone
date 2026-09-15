mod identity;

use bevy::prelude::*;

use crate::{
    content::builtin_ids::PLAINS_BIOME_ID, player::camera::GameplayCamera,
    voxel::coordinates::chunk_coord_from_position,
};

use self::identity::{
    VolumeBiomeIdentity, replace_influences, resolve_final_identity, resolve_surface_identity,
};
use super::{
    biome_field::BiomeField,
    generation_region::{generation_region_coord, generation_region_world_bounds},
    world_feature_fields::WorldFeatureFields,
};

pub const DEFAULT_BIOME_ID: &str = PLAINS_BIOME_ID;

#[derive(Clone, PartialEq)]
pub struct CurrentBiomeInfluence {
    pub id: String,
    pub weight: f32,
}

#[derive(Clone, Resource, PartialEq)]
pub struct CurrentBiome {
    pub id: String,
    pub influences: Vec<CurrentBiomeInfluence>,
    pub surface_id: String,
    pub surface_influences: Vec<CurrentBiomeInfluence>,
    pub hydrology_id: Option<String>,
    pub hydrology_influences: Vec<CurrentBiomeInfluence>,
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
            hydrology_id: None,
            hydrology_influences: Vec::new(),
            volume_id: None,
            volume_influences: Vec::new(),
            volume_strength: 0.0,
        }
    }
}

pub fn track_current_biome(
    player: Single<&Transform, With<GameplayCamera>>,
    biome_field: Option<Res<BiomeField>>,
    feature_fields: Option<Res<WorldFeatureFields>>,
    mut current_biome: ResMut<CurrentBiome>,
    mut last_position: Local<Option<Vec3>>,
) {
    let Some(biome_field) = biome_field else {
        return;
    };

    let position = player.translation;
    let source_changed = biome_field.is_changed()
        || feature_fields
            .as_ref()
            .is_some_and(|fields| fields.is_changed());
    if !source_changed && last_position.is_some_and(|previous| previous == position) {
        return;
    }
    *last_position = Some(position);

    let horizontal = Vec2::new(position.x, position.z);
    let surface = biome_field.sample_surface(horizontal);
    let volume = feature_fields.as_ref().and_then(|fields| {
        let chunk_coord = chunk_coord_from_position(position);
        let region_coord = generation_region_coord(chunk_coord);
        let volume_region = fields.volume_biome_region(region_coord, || {
            let (minimum, maximum) = generation_region_world_bounds(region_coord);
            biome_field.volume_region_in_bounds(minimum, maximum)
        });
        let selection =
            biome_field.volume_selection_in_region(position, volume_region.as_ref())?;

        Some(VolumeBiomeIdentity {
            id: biome_field.volume_biome_id(selection),
            strength: selection.strength,
        })
    });
    let hydrology = feature_fields.as_ref().map(|fields| {
        let continentalness = biome_field.climate_at(horizontal).continentalness;
        fields.hydrology_biome_overlay(continentalness)
    });
    let resolved_surface = resolve_surface_identity(&surface, hydrology);
    let mut next_biome = (*current_biome).clone();

    next_biome.surface_id = surface.primary_id.to_owned();
    replace_influences(&mut next_biome.surface_influences, &surface.influences);
    next_biome.hydrology_id = resolved_surface.hydrology_id;
    next_biome.hydrology_influences = resolved_surface.hydrology_influences;

    if let Some(volume) = volume {
        next_biome.volume_id = Some(volume.id.to_owned());
        next_biome.volume_strength = volume.strength;
        next_biome.volume_influences.clear();
        next_biome.volume_influences.push(CurrentBiomeInfluence {
            id: volume.id.to_owned(),
            weight: 1.0,
        });
        resolve_final_identity(
            &mut next_biome,
            resolved_surface.influences,
            Some(volume),
        );
    } else {
        next_biome.volume_id = None;
        next_biome.volume_strength = 0.0;
        next_biome.volume_influences.clear();
        resolve_final_identity(&mut next_biome, resolved_surface.influences, None);
    }

    if *current_biome != next_biome {
        *current_biome = next_biome;
    }
}
