use std::{collections::HashMap, sync::Arc};

use crate::content::{
    biome::BiomeRegistry,
    dimension::{GeneratedOceanDefinition, GeneratedSurfaceFluidDefinition},
};

use super::{
    biome::BiomeSample,
    foundation::{GenerationDomain, GenerationEntropy, GenerationPoint2, GenerationSnapshot},
};

const SURFACE_FLUID_PRESENCE_DOMAIN_PREFIX: &str =
    "generated-fluid/surface/presence/v1/";
const SURFACE_FLUID_JITTER_X_DOMAIN_PREFIX: &str =
    "generated-fluid/surface/jitter-x/v1/";
const SURFACE_FLUID_JITTER_Z_DOMAIN_PREFIX: &str =
    "generated-fluid/surface/jitter-z/v1/";

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct GeneratedFluidId(Arc<str>);

impl GeneratedFluidId {
    fn new(value: &str) -> Self {
        Self(Arc::from(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub(super) struct GeneratedFluidField {
    entropy: GenerationEntropy,
    ocean: Option<GeneratedOceanRule>,
    surface: HashMap<String, GeneratedSurfaceFluidRule>,
}

#[derive(Clone, Debug)]
struct GeneratedOceanRule {
    biome: Arc<str>,
    fluid: GeneratedFluidId,
    sea_level: i32,
}

#[derive(Clone, Debug)]
struct GeneratedSurfaceFluidRule {
    fluid: GeneratedFluidId,
    spacing: i32,
    radius: i64,
    jitter: u32,
    chance: f32,
    depth: u32,
    presence_domain: GenerationDomain,
    jitter_x_domain: GenerationDomain,
    jitter_z_domain: GenerationDomain,
}

impl GeneratedFluidField {
    pub(super) fn new(
        snapshot: &GenerationSnapshot,
        registry: &BiomeRegistry,
        generated_ocean: Option<&GeneratedOceanDefinition>,
        generated_surface_fluids: &[GeneratedSurfaceFluidDefinition],
    ) -> Self {
        let dimension_id = snapshot.dimension().id();
        let ocean = generated_ocean.map(|definition| {
            assert_surface_biome(registry, dimension_id, &definition.biome, "generatedOcean.biome");
            GeneratedOceanRule {
                biome: Arc::from(definition.biome.as_str()),
                fluid: GeneratedFluidId::new(&definition.fluid),
                sea_level: snapshot.dimension().sea_level(),
            }
        });
        let surface = generated_surface_fluids
            .iter()
            .enumerate()
            .map(|(index, definition)| {
                assert_surface_biome(
                    registry,
                    dimension_id,
                    &definition.biome,
                    &format!("generatedSurfaceFluids[{index}].biome"),
                );
                (
                    definition.biome.clone(),
                    GeneratedSurfaceFluidRule::new(&definition.biome, definition),
                )
            })
            .collect();

        Self {
            entropy: GenerationEntropy::new(snapshot),
            ocean,
            surface,
        }
    }

    /// Terrain-owned shallow cut depth for a local generated surface fluid.
    ///
    /// Placement is resolved once from world coordinates and the authoritative
    /// primary surface biome. Material/fluid composition consumes the resulting
    /// column cut instead of reimplementing this placement rule.
    pub(super) fn surface_cut_depth_at(&self, x: i32, z: i32, sample: &BiomeSample) -> u32 {
        self.surface
            .get(sample.primary().as_str())
            .filter(|rule| rule.contains(self.entropy, x, z))
            .map(|rule| rule.depth)
            .unwrap_or(0)
    }

    /// Initial generated fluid for one already-empty terrain voxel.
    ///
    /// `surface_cut_depth` must come from the terrain column result produced by
    /// this same field. Ocean fill and local surface-fluid cuts therefore share
    /// one generated-fluid ownership model without another boundary resolver.
    pub(super) fn fluid_at(
        &self,
        y: i32,
        density: f32,
        base_surface: f32,
        sample: &BiomeSample,
        surface_cut_depth: u32,
    ) -> Option<GeneratedFluidId> {
        if density >= 0.0 {
            return None;
        }

        let primary = sample.primary().as_str();
        let base_y = floor_to_world_y(base_surface);
        if surface_cut_depth > 0 {
            if let Some(rule) = self.surface.get(primary) {
                let cut_depth = i32::try_from(surface_cut_depth)
                    .expect("validated generated surface fluid depth must fit i32");
                let cut_floor = base_y.saturating_sub(cut_depth);
                if y > cut_floor && y <= base_y {
                    return Some(rule.fluid.clone());
                }
            }
        }

        let ocean = self.ocean.as_ref()?;
        if primary != ocean.biome.as_ref() || y > ocean.sea_level || y <= base_y {
            return None;
        }
        Some(ocean.fluid.clone())
    }
}

impl GeneratedSurfaceFluidRule {
    fn new(biome_id: &str, definition: &GeneratedSurfaceFluidDefinition) -> Self {
        Self {
            fluid: GeneratedFluidId::new(&definition.fluid),
            spacing: i32::try_from(definition.spacing)
                .expect("validated generated surface fluid spacing must fit i32"),
            radius: i64::from(definition.radius),
            jitter: definition.jitter,
            chance: definition.chance,
            depth: definition.depth,
            presence_domain: GenerationDomain::named(&format!(
                "{SURFACE_FLUID_PRESENCE_DOMAIN_PREFIX}{biome_id}"
            )),
            jitter_x_domain: GenerationDomain::named(&format!(
                "{SURFACE_FLUID_JITTER_X_DOMAIN_PREFIX}{biome_id}"
            )),
            jitter_z_domain: GenerationDomain::named(&format!(
                "{SURFACE_FLUID_JITTER_Z_DOMAIN_PREFIX}{biome_id}"
            )),
        }
    }

    fn contains(&self, entropy: GenerationEntropy, x: i32, z: i32) -> bool {
        let center_cell_x = x.div_euclid(self.spacing);
        let center_cell_z = z.div_euclid(self.spacing);
        let radius_squared = i128::from(self.radius) * i128::from(self.radius);

        for dz in -1..=1 {
            let Some(cell_z) = center_cell_z.checked_add(dz) else {
                continue;
            };
            for dx in -1..=1 {
                let Some(cell_x) = center_cell_x.checked_add(dx) else {
                    continue;
                };
                let cell = GenerationPoint2::new(cell_x, cell_z);
                if unit_probability(entropy.sample_2d(self.presence_domain, cell))
                    > f64::from(self.chance)
                {
                    continue;
                }

                let spacing = i64::from(self.spacing);
                let patch_x = i64::from(cell_x) * spacing
                    + spacing / 2
                    + jitter_offset(
                        entropy.sample_2d(self.jitter_x_domain, cell),
                        self.jitter,
                    );
                let patch_z = i64::from(cell_z) * spacing
                    + spacing / 2
                    + jitter_offset(
                        entropy.sample_2d(self.jitter_z_domain, cell),
                        self.jitter,
                    );
                let delta_x = i128::from(i64::from(x) - patch_x);
                let delta_z = i128::from(i64::from(z) - patch_z);
                if delta_x * delta_x + delta_z * delta_z <= radius_squared {
                    return true;
                }
            }
        }
        false
    }
}

fn assert_surface_biome(
    registry: &BiomeRegistry,
    dimension_id: &str,
    biome_id: &str,
    field: &str,
) {
    let definition = registry.get(biome_id).unwrap_or_else(|| {
        panic!("dimension {dimension_id} {field} references missing biome {biome_id}")
    });
    assert!(
        definition.belongs_to_dimension(dimension_id) && definition.surface_layout.is_some(),
        "dimension {dimension_id} {field} must reference a surface biome in the same dimension"
    );
}

fn unit_probability(value: u64) -> f64 {
    value as f64 / u64::MAX as f64
}

fn jitter_offset(value: u64, jitter: u32) -> i64 {
    if jitter == 0 {
        return 0;
    }
    let span = u64::from(jitter) * 2 + 1;
    i64::try_from(value % span).expect("generated surface fluid jitter remainder must fit i64")
        - i64::from(jitter)
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
