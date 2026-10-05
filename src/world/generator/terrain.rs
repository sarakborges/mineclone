use std::{collections::HashMap, sync::Arc};

use crate::content::biome::{BiomeDefinition, BiomeRegistry, SurfaceTerrainDefinition};

use super::{
    biome::{BiomeLayout, BiomeSample},
    foundation::{GenerationDomain, GenerationEntropy, GenerationPoint2, GenerationSnapshot},
};

const MACRO_NOISE_DOMAIN_PREFIX: &str = "terrain/base-surface/macro/v1/";
const DETAIL_NOISE_DOMAIN_PREFIX: &str = "terrain/base-surface/detail/v1/";

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TerrainColumnSample {
    base_surface: f32,
    surface_y: i32,
}

impl TerrainColumnSample {
    pub(crate) const fn base_surface(self) -> f32 {
        self.base_surface
    }

    pub(crate) const fn surface_y(self) -> i32 {
        self.surface_y
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TerrainAreaSample {
    origin_x: i32,
    origin_z: i32,
    width: u32,
    depth: u32,
    step: u32,
    samples: Vec<TerrainColumnSample>,
}

impl TerrainAreaSample {
    pub(crate) const fn origin(&self) -> (i32, i32) {
        (self.origin_x, self.origin_z)
    }

    pub(crate) const fn width(&self) -> u32 {
        self.width
    }

    pub(crate) const fn depth(&self) -> u32 {
        self.depth
    }

    pub(crate) const fn step(&self) -> u32 {
        self.step
    }

    pub(crate) fn sample_at(&self, x_index: u32, z_index: u32) -> Option<TerrainColumnSample> {
        if x_index >= self.width || z_index >= self.depth {
            return None;
        }
        self.samples
            .get(z_index as usize * self.width as usize + x_index as usize)
            .copied()
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TerrainQueries<'a> {
    terrain: &'a TerrainField,
}

impl TerrainQueries<'_> {
    /// Continuous authoritative 2D base-surface crossing for one X/Z column.
    pub(crate) fn base_surface_at(&self, x: i32, z: i32) -> f32 {
        self.terrain.base_surface_at(x, z)
    }

    /// Highest generated base-solid voxel for the current terrain field.
    ///
    /// Later 3D contributions extend this operation without changing the base
    /// surface fact exposed by `base_surface_at`.
    pub(crate) fn surface_at(&self, x: i32, z: i32) -> i32 {
        self.terrain.column_at(x, z).surface_y()
    }

    /// Final terrain density at a world-space voxel coordinate.
    /// Positive/zero values are solid; negative values are empty.
    pub(crate) fn density_at(&self, x: i32, y: i32, z: i32) -> f32 {
        self.terrain.density_at(x, y, z)
    }

    pub(crate) fn sample_surface_area(
        &self,
        origin_x: i32,
        origin_z: i32,
        width: u32,
        depth: u32,
    ) -> TerrainAreaSample {
        self.terrain
            .sample_surface_grid(origin_x, origin_z, width, depth, 1)
    }

    pub(crate) fn sample_surface_grid(
        &self,
        origin_x: i32,
        origin_z: i32,
        width: u32,
        depth: u32,
        step: u32,
    ) -> TerrainAreaSample {
        self.terrain
            .sample_surface_grid(origin_x, origin_z, width, depth, step)
    }
}

#[derive(Clone, Debug)]
pub(super) struct TerrainField {
    sea_level: f32,
    entropy: GenerationEntropy,
    biomes: Arc<BiomeLayout>,
    rules: HashMap<String, TerrainRule>,
}

#[derive(Clone, Debug)]
struct TerrainRule {
    profile: SurfaceTerrainDefinition,
    macro_domain: GenerationDomain,
    detail_domain: GenerationDomain,
}

impl TerrainRule {
    fn from_definition(definition: &BiomeDefinition) -> Self {
        Self {
            profile: definition.surface_terrain_profile(),
            macro_domain: GenerationDomain::named(&format!(
                "{MACRO_NOISE_DOMAIN_PREFIX}{}",
                definition.id
            )),
            detail_domain: GenerationDomain::named(&format!(
                "{DETAIL_NOISE_DOMAIN_PREFIX}{}",
                definition.id
            )),
        }
    }
}

impl TerrainField {
    pub(super) fn new(
        snapshot: &GenerationSnapshot,
        registry: &BiomeRegistry,
        biomes: Arc<BiomeLayout>,
    ) -> Self {
        let dimension_id = snapshot.dimension().id();
        let rules = registry
            .surface_for_dimension(dimension_id)
            .map(|definition| {
                (
                    definition.id.clone(),
                    TerrainRule::from_definition(definition),
                )
            })
            .collect::<HashMap<_, _>>();
        assert!(
            !rules.is_empty(),
            "dimension {dimension_id} has no authored surface terrain"
        );

        Self {
            sea_level: snapshot.dimension().sea_level() as f32,
            entropy: GenerationEntropy::new(snapshot),
            biomes,
            rules,
        }
    }

    pub(super) fn queries(&self) -> TerrainQueries<'_> {
        TerrainQueries { terrain: self }
    }

    fn column_at(&self, x: i32, z: i32) -> TerrainColumnSample {
        let base_surface = self.base_surface_at(x, z);
        TerrainColumnSample {
            base_surface,
            surface_y: floor_to_world_y(base_surface),
        }
    }

    fn base_surface_at(&self, x: i32, z: i32) -> f32 {
        let sample = self.biomes.queries().surface_biome_at(x, z);
        self.base_surface_from_biome_sample(x, z, &sample)
    }

    fn density_at(&self, x: i32, y: i32, z: i32) -> f32 {
        self.base_surface_at(x, z) - y as f32
    }

    fn sample_surface_grid(
        &self,
        origin_x: i32,
        origin_z: i32,
        width: u32,
        depth: u32,
        step: u32,
    ) -> TerrainAreaSample {
        assert!(width > 0 && depth > 0, "terrain sample area must be non-empty");
        assert!(step > 0, "terrain sample step must be positive");
        validate_grid_extent(origin_x, width, step, "X");
        validate_grid_extent(origin_z, depth, step, "Z");
        let sample_count = u64::from(width)
            .checked_mul(u64::from(depth))
            .and_then(|count| usize::try_from(count).ok())
            .expect("terrain sample area is too large");
        let biome_samples = self
            .biomes
            .queries()
            .sample_surface_grid(origin_x, origin_z, width, depth, step);
        let mut samples = Vec::with_capacity(sample_count);

        for z_index in 0..depth {
            let z = grid_axis(origin_z, z_index, step);
            for x_index in 0..width {
                let x = grid_axis(origin_x, x_index, step);
                let biome_sample = biome_samples
                    .sample_at(x_index, z_index)
                    .expect("matching biome sample grid must contain every terrain sample");
                let base_surface = self.base_surface_from_biome_sample(x, z, biome_sample);
                samples.push(TerrainColumnSample {
                    base_surface,
                    surface_y: floor_to_world_y(base_surface),
                });
            }
        }

        TerrainAreaSample {
            origin_x,
            origin_z,
            width,
            depth,
            step,
            samples,
        }
    }

    fn base_surface_from_biome_sample(&self, x: i32, z: i32, sample: &BiomeSample) -> f32 {
        let mut surface = 0.0_f32;
        let mut total_weight = 0.0_f32;

        for influence in sample.influences() {
            let rule = self.rules.get(influence.biome().as_str()).unwrap_or_else(|| {
                panic!(
                    "surface biome {} has no terrain rule",
                    influence.biome().as_str()
                )
            });
            let profile = rule.profile;
            let macro_noise = value_noise_2d(
                self.entropy,
                rule.macro_domain,
                x,
                z,
                profile.macro_scale,
            );
            let detail_noise = value_noise_2d(
                self.entropy,
                rule.detail_domain,
                x,
                z,
                profile.detail_scale,
            );
            let biome_surface = self.sea_level
                + profile.base_height_offset
                + macro_noise * profile.macro_amplitude
                + detail_noise * profile.detail_amplitude;
            surface += biome_surface * influence.weight();
            total_weight += influence.weight();
        }

        debug_assert!((total_weight - 1.0).abs() <= 0.001);
        surface
    }
}

fn value_noise_2d(
    entropy: GenerationEntropy,
    domain: GenerationDomain,
    x: i32,
    z: i32,
    scale: u32,
) -> f32 {
    let scale_i64 = i64::from(scale);
    let x_i64 = i64::from(x);
    let z_i64 = i64::from(z);
    let cell_x = x_i64.div_euclid(scale_i64);
    let cell_z = z_i64.div_euclid(scale_i64);
    let fraction_x = x_i64.rem_euclid(scale_i64) as f32 / scale as f32;
    let fraction_z = z_i64.rem_euclid(scale_i64) as f32 / scale as f32;
    let smooth_x = smoothstep(fraction_x);
    let smooth_z = smoothstep(fraction_z);

    let x0 = i32::try_from(cell_x).expect("terrain noise X lattice must fit i32");
    let z0 = i32::try_from(cell_z).expect("terrain noise Z lattice must fit i32");
    let x1 = x0
        .checked_add(1)
        .expect("terrain noise X lattice neighbor must fit i32");
    let z1 = z0
        .checked_add(1)
        .expect("terrain noise Z lattice neighbor must fit i32");

    let n00 = entropy_value(entropy.sample_2d(domain, GenerationPoint2::new(x0, z0)));
    let n10 = entropy_value(entropy.sample_2d(domain, GenerationPoint2::new(x1, z0)));
    let n01 = entropy_value(entropy.sample_2d(domain, GenerationPoint2::new(x0, z1)));
    let n11 = entropy_value(entropy.sample_2d(domain, GenerationPoint2::new(x1, z1)));
    let nx0 = lerp(n00, n10, smooth_x);
    let nx1 = lerp(n01, n11, smooth_x);
    lerp(nx0, nx1, smooth_z)
}

fn entropy_value(value: u64) -> f32 {
    const MASK: u64 = (1_u64 << 24) - 1;
    let unit = (value >> 40) & MASK;
    unit as f32 / MASK as f32 * 2.0 - 1.0
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
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

fn grid_axis(origin: i32, index: u32, step: u32) -> i32 {
    let offset = i64::from(index) * i64::from(step);
    i32::try_from(i64::from(origin) + offset).expect("validated terrain grid axis must fit i32")
}

fn validate_grid_extent(origin: i32, count: u32, step: u32, axis: &str) {
    let last_index = u64::from(count.saturating_sub(1));
    let offset = last_index
        .checked_mul(u64::from(step))
        .and_then(|value| i64::try_from(value).ok())
        .expect("terrain sample extent is too large");
    let end = i64::from(origin)
        .checked_add(offset)
        .expect("terrain sample extent overflowed");
    assert!(
        i32::try_from(end).is_ok(),
        "terrain sample {axis} extent exceeds world coordinates"
    );
}
