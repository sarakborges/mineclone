use std::{collections::HashMap, sync::Arc};

use crate::content::biome::{BiomeRegistry, SurfaceLayerDefinition};

use super::{
    biome::{BiomeLayout, BiomeSample},
    foundation::GenerationSnapshot,
    terrain::{TerrainField, TerrainVolumeSample},
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct GeneratedBlockId(Arc<str>);

impl GeneratedBlockId {
    fn new(value: &str) -> Self {
        Self(Arc::from(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MaterialVolumeSample {
    origin_x: i32,
    origin_y: i32,
    origin_z: i32,
    width: u32,
    height: u32,
    depth: u32,
    blocks: Vec<Option<GeneratedBlockId>>,
}

impl MaterialVolumeSample {
    pub(crate) const fn origin(&self) -> (i32, i32, i32) {
        (self.origin_x, self.origin_y, self.origin_z)
    }

    pub(crate) const fn width(&self) -> u32 {
        self.width
    }

    pub(crate) const fn height(&self) -> u32 {
        self.height
    }

    pub(crate) const fn depth(&self) -> u32 {
        self.depth
    }

    pub(crate) fn solid_block_at(
        &self,
        x_index: u32,
        y_index: u32,
        z_index: u32,
    ) -> Option<&GeneratedBlockId> {
        if x_index >= self.width || y_index >= self.height || z_index >= self.depth {
            return None;
        }
        let width = self.width as usize;
        let height = self.height as usize;
        self.blocks
            .get((z_index as usize * height + y_index as usize) * width + x_index as usize)
            .and_then(Option::as_ref)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct MaterialQueries<'a> {
    materials: &'a MaterialField,
}

impl MaterialQueries<'_> {
    /// Generated solid block material at one world-space voxel.
    ///
    /// `None` means the authoritative terrain field is empty at the position.
    /// Generated fluids are composed by the same Phase 5 owner in a later slice
    /// and are intentionally not represented by this solid-only query yet.
    pub(crate) fn solid_block_at(&self, x: i32, y: i32, z: i32) -> Option<GeneratedBlockId> {
        self.materials.solid_block_at(x, y, z)
    }

    /// Dense unit-step solid-material sampling for later chunk synthesis.
    pub(crate) fn sample_solid_volume(
        &self,
        origin_x: i32,
        origin_y: i32,
        origin_z: i32,
        width: u32,
        height: u32,
        depth: u32,
    ) -> MaterialVolumeSample {
        self.materials
            .sample_solid_volume(origin_x, origin_y, origin_z, width, height, depth)
    }
}

#[derive(Clone, Debug)]
pub(super) struct MaterialField {
    biomes: Arc<BiomeLayout>,
    terrain: Arc<TerrainField>,
    rules: HashMap<String, MaterialRule>,
}

#[derive(Clone, Copy)]
struct MaterialVolumeVoxel {
    x_index: u32,
    y_index: u32,
    z_index: u32,
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Clone, Debug)]
struct MaterialRule {
    layers: Arc<[MaterialLayer]>,
    finite_depth: u32,
}

#[derive(Clone, Debug)]
struct MaterialLayer {
    block: GeneratedBlockId,
    end_depth_exclusive: Option<u32>,
}

impl MaterialRule {
    fn new(definitions: &[SurfaceLayerDefinition]) -> Self {
        let mut finite_depth = 0_u32;
        let layers = definitions
            .iter()
            .map(|definition| {
                let end_depth_exclusive = definition.depth.map(|depth| {
                    finite_depth = finite_depth
                        .checked_add(depth)
                        .expect("validated surface layer depth must fit u32");
                    finite_depth
                });
                MaterialLayer {
                    block: GeneratedBlockId::new(&definition.block),
                    end_depth_exclusive,
                }
            })
            .collect::<Vec<_>>()
            .into();
        Self {
            layers,
            finite_depth,
        }
    }

    fn block_at_depth(&self, depth: u32) -> &GeneratedBlockId {
        self.layers
            .iter()
            .find(|layer| {
                layer
                    .end_depth_exclusive
                    .is_none_or(|end_depth| depth < end_depth)
            })
            .map(|layer| &layer.block)
            .expect("validated surface material profile must end with a core layer")
    }
}

impl MaterialField {
    pub(super) fn new(
        snapshot: &GenerationSnapshot,
        registry: &BiomeRegistry,
        biomes: Arc<BiomeLayout>,
        terrain: Arc<TerrainField>,
    ) -> Self {
        let dimension_id = snapshot.dimension().id();
        let rules = registry
            .surface_for_dimension(dimension_id)
            .map(|definition| {
                let layers = definition.surface_layers_profile().unwrap_or_else(|| {
                    panic!(
                        "surface biome {} has no authored surfaceLayers material profile",
                        definition.id
                    )
                });
                (definition.id.clone(), MaterialRule::new(layers))
            })
            .collect::<HashMap<_, _>>();
        assert!(
            !rules.is_empty(),
            "dimension {dimension_id} has no authored surface materials"
        );

        Self {
            biomes,
            terrain,
            rules,
        }
    }

    pub(super) fn queries(&self) -> MaterialQueries<'_> {
        MaterialQueries { materials: self }
    }

    fn solid_block_at(&self, x: i32, y: i32, z: i32) -> Option<GeneratedBlockId> {
        let terrain = self.terrain.queries();
        if terrain.density_at(x, y, z) < 0.0 {
            return None;
        }

        let biome_sample = self.biomes.queries().surface_biome_at(x, z);
        let rule = self.rule_for(&biome_sample);
        let base_y = floor_to_world_y(terrain.base_surface_at(x, z));
        let material_depth = if y <= base_y {
            depth_below(base_y, y)
        } else {
            self.additive_depth_scalar(x, y, z, rule.finite_depth)
        };
        Some(rule.block_at_depth(material_depth).clone())
    }

    fn sample_solid_volume(
        &self,
        origin_x: i32,
        origin_y: i32,
        origin_z: i32,
        width: u32,
        height: u32,
        depth: u32,
    ) -> MaterialVolumeSample {
        let terrain_queries = self.terrain.queries();
        let terrain_volume = terrain_queries.sample_density_volume(
            origin_x, origin_y, origin_z, width, height, depth,
        );
        let terrain_columns = terrain_queries.sample_surface_area(origin_x, origin_z, width, depth);
        let biome_samples = self
            .biomes
            .queries()
            .sample_surface_area(origin_x, origin_z, width, depth);
        let sample_count = checked_sample_count([width, height, depth]);
        let mut blocks = Vec::with_capacity(sample_count);

        for z_index in 0..depth {
            let z = unit_axis(origin_z, z_index);
            for y_index in 0..height {
                let y = unit_axis(origin_y, y_index);
                for x_index in 0..width {
                    let x = unit_axis(origin_x, x_index);
                    let density = terrain_volume
                        .density_at(x_index, y_index, z_index)
                        .expect("matching terrain density volume must contain every material voxel");
                    if density < 0.0 {
                        blocks.push(None);
                        continue;
                    }

                    let biome_sample = biome_samples
                        .sample_at(x_index, z_index)
                        .expect("matching biome sample area must contain every material column");
                    let rule = self.rule_for(biome_sample);
                    let base_surface = terrain_columns
                        .sample_at(x_index, z_index)
                        .expect("matching terrain sample area must contain every material column")
                        .base_surface();
                    let base_y = floor_to_world_y(base_surface);
                    let material_depth = if y <= base_y {
                        depth_below(base_y, y)
                    } else {
                        self.additive_depth_from_volume(
                            &terrain_volume,
                            MaterialVolumeVoxel {
                                x_index,
                                y_index,
                                z_index,
                                x,
                                y,
                                z,
                            },
                            rule.finite_depth,
                        )
                    };
                    blocks.push(Some(rule.block_at_depth(material_depth).clone()));
                }
            }
        }

        MaterialVolumeSample {
            origin_x,
            origin_y,
            origin_z,
            width,
            height,
            depth,
            blocks,
        }
    }

    fn rule_for(&self, sample: &BiomeSample) -> &MaterialRule {
        self.rules
            .get(sample.primary().as_str())
            .unwrap_or_else(|| {
                panic!(
                    "surface biome {} has no material rule",
                    sample.primary().as_str()
                )
            })
    }

    fn additive_depth_scalar(&self, x: i32, y: i32, z: i32, finite_depth: u32) -> u32 {
        for offset in 1..=finite_depth {
            let Some(sample_y) = checked_offset_y(y, offset) else {
                return offset - 1;
            };
            if self.terrain.queries().density_at(x, sample_y, z) < 0.0 {
                return offset - 1;
            }
        }
        finite_depth
    }

    fn additive_depth_from_volume(
        &self,
        volume: &TerrainVolumeSample,
        voxel: MaterialVolumeVoxel,
        finite_depth: u32,
    ) -> u32 {
        let MaterialVolumeVoxel {
            x_index,
            y_index,
            z_index,
            x,
            y,
            z,
        } = voxel;
        for offset in 1..=finite_depth {
            let Some(sample_y) = checked_offset_y(y, offset) else {
                return offset - 1;
            };
            let sample_density = y_index
                .checked_add(offset)
                .filter(|sample_y_index| *sample_y_index < volume.height())
                .and_then(|sample_y_index| volume.density_at(x_index, sample_y_index, z_index))
                .unwrap_or_else(|| self.terrain.queries().density_at(x, sample_y, z));
            if sample_density < 0.0 {
                return offset - 1;
            }
        }
        finite_depth
    }
}

fn floor_to_world_y(value: f32) -> i32 {
    if value <= i32::MIN as f32 {
        i32::MIN
    } else if value >= i32::MAX as f32 {
        i32::MAX
    } else {
        value.floor() as i32
    }
}

fn depth_below(surface_y: i32, y: i32) -> u32 {
    u32::try_from(i64::from(surface_y) - i64::from(y)).unwrap_or(u32::MAX)
}

fn checked_offset_y(y: i32, offset: u32) -> Option<i32> {
    i32::try_from(i64::from(y) + i64::from(offset)).ok()
}

fn unit_axis(origin: i32, index: u32) -> i32 {
    i32::try_from(i64::from(origin) + i64::from(index))
        .expect("validated material volume axis must fit i32")
}

fn checked_sample_count(extents: [u32; 3]) -> usize {
    extents
        .into_iter()
        .try_fold(1_u64, |count, extent| count.checked_mul(u64::from(extent)))
        .and_then(|count| usize::try_from(count).ok())
        .expect("material sample volume is too large")
}
