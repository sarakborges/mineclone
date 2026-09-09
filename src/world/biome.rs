use bevy::prelude::*;

use crate::player::camera::GameplayCamera;

use super::{
    biome_field::{BiomeField, BiomeFieldSample, BiomeInfluence, VolumeBiomeFieldSample},
    hydrology::HydrologyBiomeOverlay,
    world_feature_fields::WorldFeatureFields,
};

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
) {
    let Some(biome_field) = biome_field else {
        return;
    };

    let position = player.translation;
    let horizontal = Vec2::new(position.x, position.z);
    let surface = biome_field.sample_surface(horizontal);
    let volume = biome_field.sample_volume(position);
    let hydrology = feature_fields.as_ref().map(|fields| {
        let continentalness = biome_field.climate_at(horizontal).continentalness;
        fields.hydrology().biome_overlay(continentalness)
    });
    let resolved_surface = resolve_surface_identity(&surface, hydrology);

    current_biome.surface_id = surface.primary_id.to_owned();
    replace_influences(&mut current_biome.surface_influences, &surface.influences);
    current_biome.hydrology_id = resolved_surface.hydrology_id;
    current_biome.hydrology_influences = resolved_surface.hydrology_influences;

    if let Some(volume) = volume {
        current_biome.volume_id = Some(volume.primary_id.to_owned());
        current_biome.volume_strength = volume.strength;
        replace_influences(
            &mut current_biome.volume_influences,
            &volume.influences,
        );
        resolve_final_identity(
            &mut current_biome,
            resolved_surface.influences,
            Some(&volume),
        );
    } else {
        current_biome.volume_id = None;
        current_biome.volume_strength = 0.0;
        current_biome.volume_influences.clear();
        resolve_final_identity(&mut current_biome, resolved_surface.influences, None);
    }
}

struct ResolvedSurfaceIdentity {
    hydrology_id: Option<String>,
    hydrology_influences: Vec<CurrentBiomeInfluence>,
    influences: Vec<CurrentBiomeInfluence>,
}

fn resolve_surface_identity(
    surface: &BiomeFieldSample<'_>,
    hydrology: Option<HydrologyBiomeOverlay<'_>>,
) -> ResolvedSurfaceIdentity {
    let Some(hydrology) = hydrology else {
        return ResolvedSurfaceIdentity {
            hydrology_id: None,
            hydrology_influences: Vec::new(),
            influences: owned_influences(&surface.influences),
        };
    };

    let mut influences = Vec::new();
    let mut hydrology_influences = Vec::new();

    for influence in &surface.influences {
        push_influence(
            &mut influences,
            influence.id,
            influence.weight * hydrology.surface_weight,
        );
    }

    if let Some(coast_id) = hydrology.coast_biome {
        push_influence(&mut influences, coast_id, hydrology.coast_weight);
        push_influence(
            &mut hydrology_influences,
            coast_id,
            hydrology.coast_weight,
        );
    }

    if let Some(ocean_id) = hydrology.ocean_biome {
        push_influence(&mut influences, ocean_id, hydrology.ocean_weight);
        push_influence(
            &mut hydrology_influences,
            ocean_id,
            hydrology.ocean_weight,
        );
    }

    normalize_influences(&mut influences);
    normalize_influences(&mut hydrology_influences);

    let hydrology_id = primary_influence(&hydrology_influences).map(|id| id.to_owned());

    ResolvedSurfaceIdentity {
        hydrology_id,
        hydrology_influences,
        influences,
    }
}

fn resolve_final_identity(
    current_biome: &mut CurrentBiome,
    surface: Vec<CurrentBiomeInfluence>,
    volume: Option<&VolumeBiomeFieldSample<'_>>,
) {
    let volume_strength = volume.map_or(0.0, |sample| sample.strength.clamp(0.0, 1.0));
    let surface_strength = 1.0 - volume_strength;
    let mut influences = Vec::new();

    for influence in surface {
        push_influence(
            &mut influences,
            &influence.id,
            influence.weight * surface_strength,
        );
    }

    if let Some(volume) = volume {
        for influence in &volume.influences {
            push_influence(
                &mut influences,
                influence.id,
                influence.weight * volume_strength,
            );
        }
    }

    normalize_influences(&mut influences);

    if let Some(primary) = primary_influence(&influences) {
        current_biome.id = primary.to_owned();
    }
    current_biome.influences = influences;
}

fn owned_influences(source: &[BiomeInfluence<'_>]) -> Vec<CurrentBiomeInfluence> {
    source
        .iter()
        .map(|influence| CurrentBiomeInfluence {
            id: influence.id.to_owned(),
            weight: influence.weight,
        })
        .collect()
}

fn replace_influences(
    target: &mut Vec<CurrentBiomeInfluence>,
    source: &[BiomeInfluence<'_>],
) {
    target.clear();
    target.extend(owned_influences(source));
}

fn push_influence(target: &mut Vec<CurrentBiomeInfluence>, id: &str, weight: f32) {
    if weight <= 0.0 {
        return;
    }

    if let Some(existing) = target.iter_mut().find(|influence| influence.id == id) {
        existing.weight += weight;
        return;
    }

    target.push(CurrentBiomeInfluence {
        id: id.to_owned(),
        weight,
    });
}

fn normalize_influences(influences: &mut Vec<CurrentBiomeInfluence>) {
    let total: f32 = influences.iter().map(|influence| influence.weight).sum();

    if total <= f32::EPSILON {
        return;
    }

    for influence in influences {
        influence.weight /= total;
    }
}

fn primary_influence(influences: &[CurrentBiomeInfluence]) -> Option<&str> {
    influences
        .iter()
        .max_by(|left, right| left.weight.total_cmp(&right.weight))
        .map(|influence| influence.id.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hydrology_overlay_keeps_surface_and_hydrology_layers_separate() {
        let surface = BiomeFieldSample {
            primary_id: "surface",
            influences: vec![BiomeInfluence {
                id: "surface",
                weight: 1.0,
            }],
        };
        let resolved = resolve_surface_identity(
            &surface,
            Some(HydrologyBiomeOverlay {
                surface_weight: 0.25,
                coast_biome: Some("coast"),
                coast_weight: 0.75,
                ocean_biome: Some("ocean"),
                ocean_weight: 0.0,
            }),
        );

        assert_eq!(resolved.hydrology_id.as_deref(), Some("coast"));
        assert_eq!(resolved.hydrology_influences.len(), 1);
        assert_eq!(resolved.influences.len(), 2);
    }
}
