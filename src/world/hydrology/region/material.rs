use bevy::prelude::*;

use super::HydrologyRegion;
use crate::world::hydrology::{
    constants::{
        BED_MATERIAL_DEPTH, COAST_MAXIMUM_SURFACE_HEIGHT, OCEAN_EXTRA_DEPTH,
        OCEAN_MINIMUM_DEPTH, SHORE_STRENGTH,
    },
    math::{hydrology_dominates_surface, lerp, smoothstep},
};

impl HydrologyRegion {
    pub fn solid_block_at(&self, position: Vec3) -> Option<&str> {
        let horizontal = Vec2::new(position.x, position.z);

        self.water_body_material_at(position, horizontal)
            .or_else(|| self.river_material_at(position, horizontal))
            .or_else(|| self.ocean_material_at(position, horizontal))
    }

    fn water_body_material_at(&self, position: Vec3, horizontal: Vec2) -> Option<&str> {
        let (body, strength) = self
            .water_bodies
            .iter()
            .filter_map(|body| {
                let strength = body.horizontal_strength(horizontal);
                (strength > 0.0).then_some((body, strength))
            })
            .max_by(|(_, left), (_, right)| left.total_cmp(right))?;
        let bottom = body.water_level - body.carve_depth * strength;

        if !is_near_bed(position.y, bottom) {
            return None;
        }

        self.lake_bed_material(strength)
    }

    fn river_material_at(&self, position: Vec3, horizontal: Vec2) -> Option<&str> {
        let river = self.river_graph.sample_horizontal(horizontal)?;
        let profile = smoothstep(river.strength);
        let bed = river.height - self.river_carve_depth * profile;

        if !is_near_bed(position.y, bed) {
            return None;
        }

        if river.strength <= SHORE_STRENGTH {
            self.settings
                .shore_block
                .as_deref()
                .or(self.settings.river_bed_block.as_deref())
        } else {
            self.settings
                .river_bed_block
                .as_deref()
                .or(self.settings.shore_block.as_deref())
        }
    }

    fn ocean_material_at(&self, position: Vec3, horizontal: Vec2) -> Option<&str> {
        let strength = self.ocean_strength_at(horizontal);
        if strength <= 0.0 || !hydrology_dominates_surface(strength) {
            return None;
        }

        let sample = self.macro_sample_at(horizontal)?;
        let target_floor = self.sea_level - OCEAN_MINIMUM_DEPTH - OCEAN_EXTRA_DEPTH * strength;
        let floor = lerp(sample.elevation, target_floor, strength);

        if floor > self.sea_level + COAST_MAXIMUM_SURFACE_HEIGHT
            || !is_near_bed(position.y, floor)
        {
            return None;
        }

        if strength <= SHORE_STRENGTH {
            self.settings
                .shore_block
                .as_deref()
                .or(self.settings.ocean_bed_block.as_deref())
        } else {
            self.settings
                .ocean_bed_block
                .as_deref()
                .or(self.settings.shore_block.as_deref())
        }
    }

    fn lake_bed_material(&self, strength: f32) -> Option<&str> {
        let bed = self.settings.lake_bed_block.as_deref();

        if strength <= SHORE_STRENGTH {
            self.settings.shore_block.as_deref().or(bed)
        } else {
            bed.or(self.settings.shore_block.as_deref())
        }
    }
}

fn is_near_bed(y: f32, bed: f32) -> bool {
    y >= bed - BED_MATERIAL_DEPTH && y <= bed + BED_MATERIAL_DEPTH
}
