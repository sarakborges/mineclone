use super::CurrentBiomeInfluence;
use crate::world::{
    biome_field::{BiomeFieldSample, BiomeInfluence},
    hydrology::HydrologyBiomeOverlay,
};

#[derive(Clone, Copy)]
pub(super) struct VolumeBiomeIdentity<'a> {
    pub id: &'a str,
    pub strength: f32,
}

pub(super) fn resolve_surface_identity(
    surface: &BiomeFieldSample<'_>,
    hydrology: Option<HydrologyBiomeOverlay<'_>>,
    influences: &mut Vec<CurrentBiomeInfluence>,
    hydrology_influences: &mut Vec<CurrentBiomeInfluence>,
    hydrology_id: &mut Option<String>,
) -> usize {
    let Some(hydrology) = hydrology else {
        hydrology_influences.clear();
        replace_optional_string(hydrology_id, None);
        return copy_influences(influences, &surface.influences);
    };

    let mut influence_count = 0;
    let mut hydrology_count = 0;

    for influence in &surface.influences {
        push_influence(
            influences,
            &mut influence_count,
            influence.id,
            influence.weight * hydrology.surface_weight,
        );
    }

    if let Some(coast_id) = hydrology.coast_biome {
        push_influence(
            influences,
            &mut influence_count,
            coast_id,
            hydrology.coast_weight,
        );
        push_influence(
            hydrology_influences,
            &mut hydrology_count,
            coast_id,
            hydrology.coast_weight,
        );
    }

    if let Some(ocean_id) = hydrology.ocean_biome {
        push_influence(
            influences,
            &mut influence_count,
            ocean_id,
            hydrology.ocean_weight,
        );
        push_influence(
            hydrology_influences,
            &mut hydrology_count,
            ocean_id,
            hydrology.ocean_weight,
        );
    }

    normalize_influences(&mut influences[..influence_count]);
    normalize_influences(&mut hydrology_influences[..hydrology_count]);
    hydrology_influences.truncate(hydrology_count);

    let resolved_hydrology_id = primary_influence(hydrology_influences);
    replace_optional_string(hydrology_id, resolved_hydrology_id);
    influence_count
}

pub(super) fn apply_volume_identity(
    influences: &mut Vec<CurrentBiomeInfluence>,
    mut influence_count: usize,
    volume: Option<VolumeBiomeIdentity<'_>>,
    fallback_id: &str,
    id: &mut String,
) {
    let volume_strength = volume.map_or(0.0, |identity| identity.strength.clamp(0.0, 1.0));
    let surface_strength = 1.0 - volume_strength;

    for influence in &mut influences[..influence_count] {
        influence.weight *= surface_strength;
    }

    if let Some(volume) = volume {
        push_influence(
            influences,
            &mut influence_count,
            volume.id,
            volume_strength,
        );
    }

    normalize_influences(&mut influences[..influence_count]);
    let resolved_id = primary_influence(&influences[..influence_count]).unwrap_or(fallback_id);
    replace_string(id, resolved_id);
    influences.truncate(influence_count);
}

pub(super) fn replace_influences(
    target: &mut Vec<CurrentBiomeInfluence>,
    source: &[BiomeInfluence<'_>],
) {
    let count = copy_influences(target, source);
    target.truncate(count);
}

pub(super) fn replace_single_influence(
    target: &mut Vec<CurrentBiomeInfluence>,
    id: &str,
    weight: f32,
) {
    write_influence(target, 0, id, weight);
    target.truncate(1);
}

pub(super) fn replace_string(target: &mut String, value: &str) {
    if target == value {
        return;
    }

    target.clear();
    target.push_str(value);
}

pub(super) fn replace_optional_string(target: &mut Option<String>, value: Option<&str>) {
    match value {
        Some(value) => {
            if let Some(existing) = target.as_mut() {
                replace_string(existing, value);
            } else {
                *target = Some(value.to_owned());
            }
        }
        None => {
            if target.is_some() {
                *target = None;
            }
        }
    }
}

fn copy_influences(
    target: &mut Vec<CurrentBiomeInfluence>,
    source: &[BiomeInfluence<'_>],
) -> usize {
    for (index, influence) in source.iter().enumerate() {
        write_influence(target, index, influence.id, influence.weight);
    }
    source.len()
}

fn push_influence(
    target: &mut Vec<CurrentBiomeInfluence>,
    target_count: &mut usize,
    id: &str,
    weight: f32,
) {
    if weight <= 0.0 {
        return;
    }

    if let Some(existing) = target[..*target_count]
        .iter_mut()
        .find(|influence| influence.id == id)
    {
        existing.weight += weight;
        return;
    }

    write_influence(target, *target_count, id, weight);
    *target_count += 1;
}

fn write_influence(
    target: &mut Vec<CurrentBiomeInfluence>,
    index: usize,
    id: &str,
    weight: f32,
) {
    if let Some(existing) = target.get_mut(index) {
        replace_string(&mut existing.id, id);
        existing.weight = weight;
        return;
    }

    debug_assert_eq!(index, target.len());
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
            influences: [BiomeInfluence {
                id: "surface",
                weight: 1.0,
                surface_index: 0,
            }]
            .into_iter()
            .collect(),
        };
        let mut influences = Vec::new();
        let mut hydrology_influences = Vec::new();
        let mut hydrology_id = None;
        let count = resolve_surface_identity(
            &surface,
            Some(HydrologyBiomeOverlay {
                surface_weight: 0.25,
                coast_biome: Some("coast"),
                coast_weight: 0.75,
                ocean_biome: Some("ocean"),
                ocean_weight: 0.0,
            }),
            &mut influences,
            &mut hydrology_influences,
            &mut hydrology_id,
        );
        influences.truncate(count);

        assert_eq!(hydrology_id.as_deref(), Some("coast"));
        assert_eq!(hydrology_influences.len(), 1);
        assert_eq!(influences.len(), 2);
    }

    #[test]
    fn final_identity_preserves_fallback_when_no_influences_exist() {
        let mut influences = Vec::new();
        let mut id = String::new();

        apply_volume_identity(&mut influences, 0, None, "fallback", &mut id);

        assert_eq!(id, "fallback");
        assert!(influences.is_empty());
    }

    #[test]
    fn replacing_influences_reuses_existing_string_slots() {
        let mut target = vec![CurrentBiomeInfluence {
            id: String::from("old-biome-name-with-capacity"),
            weight: 0.25,
        }];
        let capacity = target[0].id.capacity();
        let source = [BiomeInfluence {
            id: "new",
            weight: 1.0,
            surface_index: 0,
        }];

        replace_influences(&mut target, &source);

        assert_eq!(target[0].id, "new");
        assert_eq!(target[0].weight, 1.0);
        assert!(target[0].id.capacity() >= capacity);
    }
}
