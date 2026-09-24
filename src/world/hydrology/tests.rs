use super::{
    HydrologyField, HydrologySurfaceSample, WaterBody,
    constants::HYDROLOGY_REGION_SIZE,
    drainage::{DrainageNode, drainage_position},
    lake::{LakeBasinContext, lake_for_local_basin},
};
use crate::content::{
    biome_hydrology::BiomeHydrologyRules, builtin_ids::WATER_FLUID_ID,
    dimension_hydrology::DimensionHydrology,
};
use bevy::prelude::*;

fn field() -> HydrologyField {
    HydrologyField::new(42, 64, DimensionHydrology::default())
}

fn surface(elevation: f32, ocean_weight: f32) -> HydrologySurfaceSample {
    HydrologySurfaceSample {
        elevation,
        ocean_weight,
        biome_hydrology: BiomeHydrologyRules::default(),
    }
}

#[test]
fn water_body_strength_fades_to_zero_at_shoreline() {
    let body = WaterBody {
        center: Vec2::ZERO,
        radius: Vec2::splat(10.0),
        rotation: 0.0,
        shape_seed: 42,
        water_level: 64.0,
        carve_depth: 8.0,
        fluid_id: WATER_FLUID_ID.to_owned(),
    };

    assert_eq!(body.horizontal_strength(Vec2::ZERO), 1.0);
    assert!(body.horizontal_strength(Vec2::new(10.0, 0.0)) < 0.5);
    assert_eq!(body.horizontal_strength(Vec2::new(15.0, 0.0)), 0.0);
}

#[test]
fn macro_terrain_generation_is_deterministic() {
    let field = field();
    let sample = |position: Vec2| surface(position.x + position.y, 0.0);
    let first = field.region_from_macro_terrain(IVec2::ZERO, true, true, sample);
    let second = field.region_from_macro_terrain(IVec2::ZERO, true, true, sample);

    assert_eq!(
        first.river_graph.edge_count(),
        second.river_graph.edge_count()
    );
    assert_eq!(first.water_bodies.len(), second.water_bodies.len());
}

#[test]
fn ocean_biome_is_not_recreated_as_hydrology_water() {
    let field = field();
    let region = field.region_from_macro_terrain(
        IVec2::ZERO,
        true,
        true,
        |_| surface(50.0, 1.0),
    );
    let center = Vec2::splat(HYDROLOGY_REGION_SIZE * 0.5);

    assert!(region.water_at(center).is_none());
    assert_eq!(region.density_delta(Vec3::new(center.x, 60.0, center.y)), 0.0);
}

#[test]
fn biome_can_disable_lake_generation() {
    let source = DrainageNode {
        position: Vec2::ZERO,
        elevation: 80.0,
        ocean_weight: 0.0,
        biome_hydrology: BiomeHydrologyRules {
            can_generate_lake: false,
            lake_chance_multiplier: 10.0,
            ..default()
        },
    };
    let neighbors = vec![DrainageNode {
        position: Vec2::X,
        elevation: 84.0,
        ocean_weight: 0.0,
        biome_hydrology: BiomeHydrologyRules::default(),
    }];

    assert!(
        lake_for_local_basin(
            IVec2::ZERO,
            source,
            &neighbors,
            &LakeBasinContext {
                seed: 42,
                sea_level: 64.0,
                water_fluid: WATER_FLUID_ID,
                lake_weight: 1.0,
            },
        )
        .is_none()
    );
}

#[test]
fn drainage_position_is_global_and_region_independent() {
    let cell = IVec2::new(3, -4);

    assert_eq!(drainage_position(cell, 99), drainage_position(cell, 99));
}
