use super::{CurrentBiome, CurrentBiomeInfluence};
use crate::world::{
    biome_field::{BiomeFieldSample, BiomeInfluence, VolumeBiomeFieldSample},
    hydrology::HydrologyBiomeOverlay,
};

pub(super) struct ResolvedSurfaceIdentity {
    pub hydrology_id: Option<String>,
    pub hydrology_influences: Vec<CurrentBiomeInfluence>,
    pub influences: Vec<CurrentBiomeInfluence>,
}

pub(super) fn resolve_surface_identity(
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

    let hydrology_id = primary_influence(&hydrology_influences).map(str::to_owned);

    ResolvedSurfaceIdentity {
        hydrology_id,
        hydrology_influences,
        influences,
    }
}

pub(super) fn resolve_final_identity(
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

pub(super) fn replace_influences(
    target: &mut Vec<CurrentBiomeInfluence>,
    source: &[BiomeInfluence<'_>],
) {
    target.clear();
    target.extend(owned_influences(source));
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

fn normalize_influences(influences: &mut [CurrentBiomeInfluence]) {
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
