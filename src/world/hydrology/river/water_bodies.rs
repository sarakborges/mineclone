use bevy::prelude::*;

use super::path::{WATERFALL_MINIMUM_DROP, WaterfallLanding, river_height};
use super::super::{
    drainage::DrainageNode,
    math::{cell_hash, hash_unit, lerp},
    types::WaterBody,
};

const PLUNGE_POOL_CHANCE: f32 = 0.84;

pub(super) fn mountain_spring_body(
    cell: IVec2,
    source: DrainageNode,
    seed: u64,
    sea_level: f32,
    water_fluid: &str,
) -> WaterBody {
    let hash = cell_hash(cell, seed ^ 0x1f83_d9ab_fb41_bd6b);
    let base_radius = lerp(4.0, 7.5, hash_unit(hash.rotate_left(11)));
    let aspect = lerp(0.82, 1.18, hash_unit(hash.rotate_left(27)));

    WaterBody {
        center: source.position,
        radius: Vec2::new(base_radius * aspect, base_radius * (2.0 - aspect)),
        rotation: hash_unit(hash.rotate_left(41)) * std::f32::consts::TAU,
        shape_seed: hash.rotate_left(7),
        water_level: river_height(source, sea_level),
        carve_depth: lerp(2.5, 4.5, hash_unit(hash.rotate_left(53))),
        fluid_id: water_fluid.to_owned(),
    }
}

pub(super) fn plunge_pool_for_waterfall(
    source_cell: IVec2,
    waterfall: WaterfallLanding,
    seed: u64,
    water_fluid: &str,
) -> Option<WaterBody> {
    let hash = cell_hash(source_cell, seed ^ 0x428a_2f98_d728_ae22);
    if hash_unit(hash.rotate_left(15)) >= PLUNGE_POOL_CHANCE {
        return None;
    }

    let drop_strength =
        ((waterfall.drop - WATERFALL_MINIMUM_DROP) / 24.0).clamp(0.0, 1.0);
    let base_radius = lerp(
        6.0,
        13.0,
        (drop_strength * 0.7 + hash_unit(hash.rotate_left(31)) * 0.3).clamp(0.0, 1.0),
    );
    let aspect = lerp(0.78, 1.22, hash_unit(hash.rotate_left(47)));

    Some(WaterBody {
        center: Vec2::new(waterfall.position.x, waterfall.position.z),
        radius: Vec2::new(base_radius * aspect, base_radius * (2.0 - aspect)),
        rotation: hash_unit(hash.rotate_left(5)) * std::f32::consts::TAU,
        shape_seed: hash.rotate_left(39),
        water_level: waterfall.position.y + 0.35,
        carve_depth: lerp(4.5, 9.5, drop_strength),
        fluid_id: water_fluid.to_owned(),
    })
}
