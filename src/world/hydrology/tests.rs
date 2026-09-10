use super::{
    HydrologyField, HydrologySurfaceSample, WaterBody,
    constants::{HYDROLOGY_REGION_SIZE, MACRO_SAMPLE_GRID},
    drainage::{DrainageNode, drainage_position},
    lake::lake_for_local_basin,
    spatial::macro_sample_position,
};
use crate::content::{
    biome_hydrology::BiomeHydrology, builtin_ids::WATER_FLUID_ID,
    dimension_hydrology::DimensionHydrology,
};
use bevy::prelude::*;

fn settings() -> DimensionHydrology {
    DimensionHydrology {
        ocean_biome: Some("asteria:test/ocean".into()),
        coast_biome: Some("asteria:test/coast".into()),
        ..default()
    }
}

fn field() -> HydrologyField {
    HydrologyField::new(42, 64, settings())
}

fn surface(elevation: f32, continentalness: f32) -> HydrologySurfaceSample {
    HydrologySurfaceSample {
        elevation,
        continentalness,
        biome_hydrology: BiomeHydrology::default(),
    }
}

#[test]
fn water_body_strength_fades_to_zero_at_shoreline() {
    let body = WaterBody {
        center: Vec2::ZERO,
        radius: Vec2::splat(10.0),
        water_level: 64.0,
        carve_depth: 8.0,
        fluid_id: WATER_FLUID_ID.to_owned(),
    };

    assert_eq!(body.horizontal_strength(Vec2::ZERO), 1.0);
    assert_eq!(body.horizontal_strength(Vec2::new(10.0, 0.0)), 0.0);
}

#[test]
fn macro_terrain_generation_is_deterministic() {
    let field = field();
    let sample = |position: Vec2| surface(position.x + position.y, 0.5);
    let first = field.region_from_macro_terrain(IVec2::ZERO, sample);
    let second = field.region_from_macro_terrain(IVec2::ZERO, sample);

    assert_eq!(
        first.river_graph.edges().len(),
        second.river_graph.edges().len()
    );
    assert_eq!(first.water_bodies.len(), second.water_bodies.len());
}

#[test]
fn low_continentalness_produces_ocean_water_and_carving() {
    let field = field();
    let region = field.region_from_macro_terrain(IVec2::ZERO, |_| surface(70.0, 0.1));
    let center = Vec2::splat(HYDROLOGY_REGION_SIZE * 0.5);

    let water = region.water_at(center).unwrap();
    assert_eq!(water.fluid_id, WATER_FLUID_ID);
    assert_eq!(water.water_level, 64.0);
    assert!(region.density_delta(Vec3::new(center.x, 60.0, center.y)) < 0.0);
}

#[test]
fn hydrology_biome_overlay_transitions_surface_to_coast_to_ocean() {
    let field = field();
    let land = field.biome_overlay(0.5);
    let coast = field.biome_overlay(0.28);
    let ocean = field.biome_overlay(0.1);

    assert_eq!(land.surface_weight, 1.0);
    assert!(coast.coast_weight > coast.surface_weight);
    assert!(coast.coast_weight > coast.ocean_weight);
    assert_eq!(ocean.ocean_weight, 1.0);
}

#[test]
fn biome_can_disable_lake_generation() {
    let source = DrainageNode {
        position: Vec2::ZERO,
        elevation: 80.0,
        continentalness: 0.8,
        biome_hydrology: BiomeHydrology {
            can_generate_lake: false,
            lake_chance_multiplier: 10.0,
            ..default()
        },
    };
    let neighbors = vec![DrainageNode {
        position: Vec2::X,
        elevation: 84.0,
        continentalness: 0.8,
        biome_hydrology: BiomeHydrology::default(),
    }];

    assert!(
        lake_for_local_basin(IVec2::ZERO, source, &neighbors, 42, 64.0, WATER_FLUID_ID,).is_none()
    );
}

#[test]
fn adjacent_regions_sample_the_same_shared_boundary() {
    let left_boundary = macro_sample_position(IVec2::ZERO, MACRO_SAMPLE_GRID - 1, 2);
    let right_boundary = macro_sample_position(IVec2::X, 0, 2);

    assert_eq!(left_boundary, right_boundary);
}

#[test]
fn drainage_position_is_global_and_region_independent() {
    let cell = IVec2::new(3, -4);

    assert_eq!(drainage_position(cell, 99), drainage_position(cell, 99));
}
