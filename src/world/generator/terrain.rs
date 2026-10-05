use std::{collections::HashMap, sync::Arc};

use crate::content::biome::{
    BiomeDefinition, BiomeRegistry, FloatingFormationDefinition, SurfaceTerrainDefinition,
};

use super::{
    biome::{BiomeLayout, BiomeSample},
    foundation::{
        GenerationDomain, GenerationEntropy, GenerationPoint2, GenerationPoint3,
        GenerationSnapshot,
    },
    generated_fluid::GeneratedFluidField,
};

const MACRO_NOISE_DOMAIN_PREFIX: &str = "terrain/base-surface/macro/v1/";
const DETAIL_NOISE_DOMAIN_PREFIX: &str = "terrain/base-surface/detail/v1/";
const CAVE_PRIMARY_DOMAIN: &str = "terrain/density/caves/primary/v1";
const CAVE_SECONDARY_DOMAIN: &str = "terrain/density/caves/secondary/v1";
const FLOATING_MASK_DOMAIN_PREFIX: &str = "terrain/density/floating/mask/v1/";
const FLOATING_DETAIL_DOMAIN_PREFIX: &str = "terrain/density/floating/detail/v1/";

const CAVE_HORIZONTAL_SCALE: u32 = 56;
const CAVE_VERTICAL_SCALE: u32 = 36;
const CAVE_MIN_DEPTH: f32 = 10.0;
const CAVE_MAX_DEPTH: f32 = 120.0;
const CAVE_BOUNDARY_FADE: f32 = 8.0;
const CAVE_NOISE_HALF_WIDTH: f32 = 0.18;
const CAVE_DENSITY_SCALE: f32 = 24.0;
const SURFACE_FLUID_VOID_DENSITY: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TerrainColumnSample {
    base_surface: f32,
    surface_y: i32,
    surface_cut_depth: u32,
}

impl TerrainColumnSample {
    pub(crate) const fn base_surface(self) -> f32 {
        self.base_surface
    }

    pub(crate) const fn surface_y(self) -> i32 {
        self.surface_y
    }

    pub(crate) const fn surface_cut_depth(self) -> u32 {
        self.surface_cut_depth
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TerrainVolumeSample {
    origin_x: i32,
    origin_y: i32,
    origin_z: i32,
    width: u32,
    height: u32,
    depth: u32,
    densities: Vec<f32>,
}

impl TerrainVolumeSample {
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

    pub(crate) fn density_at(
        &self,
        x_index: u32,
        y_index: u32,
        z_index: u32,
    ) -> Option<f32> {
        if x_index >= self.width || y_index >= self.height || z_index >= self.depth {
            return None;
        }
        let width = self.width as usize;
        let height = self.height as usize;
        self.densities
            .get((z_index as usize * height + y_index as usize) * width + x_index as usize)
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

    /// Authoritative bounded column summary for one X/Z coordinate.
    pub(crate) fn column_at(&self, x: i32, z: i32) -> TerrainColumnSample {
        self.terrain.column_at(x, z)
    }

    /// Highest generated solid voxel for the current final terrain field.
    ///
    /// Additive contributions expose explicit finite vertical candidate bounds,
    /// so this query never scans the whole world height.
    pub(crate) fn surface_at(&self, x: i32, z: i32) -> i32 {
        self.column_at(x, z).surface_y()
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

    /// Dense unit-step XYZ sampling for future chunk materialization and
    /// diagnostics. Biome ownership and base surface are sampled once per X/Z
    /// column and reused across every Y in the request.
    pub(crate) fn sample_density_volume(
        &self,
        origin_x: i32,
        origin_y: i32,
        origin_z: i32,
        width: u32,
        height: u32,
        depth: u32,
    ) -> TerrainVolumeSample {
        self.terrain
            .sample_density_volume(origin_x, origin_y, origin_z, width, height, depth)
    }
}

#[derive(Clone, Debug)]
pub(super) struct TerrainField {
    sea_level: f32,
    entropy: GenerationEntropy,
    biomes: Arc<BiomeLayout>,
    generated_fluids: Arc<GeneratedFluidField>,
    rules: HashMap<String, TerrainRule>,
    caves: CaveField,
}

#[derive(Clone, Debug)]
struct TerrainRule {
    profile: SurfaceTerrainDefinition,
    macro_domain: GenerationDomain,
    detail_domain: GenerationDomain,
    floating: Option<FloatingFormationRule>,
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
            floating: definition
                .terrain_3d_profile()
                .and_then(|terrain| terrain.floating_formation)
                .map(|floating| FloatingFormationRule::new(&definition.id, floating)),
        }
    }
}

#[derive(Clone, Debug)]
struct FloatingFormationRule {
    definition: FloatingFormationDefinition,
    mask_domain: GenerationDomain,
    detail_domain: GenerationDomain,
}

impl FloatingFormationRule {
    fn new(biome_id: &str, definition: FloatingFormationDefinition) -> Self {
        Self {
            definition,
            mask_domain: GenerationDomain::named(&format!(
                "{FLOATING_MASK_DOMAIN_PREFIX}{biome_id}"
            )),
            detail_domain: GenerationDomain::named(&format!(
                "{FLOATING_DETAIL_DOMAIN_PREFIX}{biome_id}"
            )),
        }
    }

    const fn vertical_bounds(&self) -> (i32, i32) {
        (self.definition.min_y, self.definition.max_y)
    }

    fn density(
        &self,
        entropy: GenerationEntropy,
        x: i32,
        y: i32,
        z: i32,
        influence_weight: f32,
    ) -> f32 {
        let definition = self.definition;
        if !(definition.min_y..=definition.max_y).contains(&y) || influence_weight <= 0.0 {
            return f32::NEG_INFINITY;
        }

        let min_y = definition.min_y as f32;
        let max_y = definition.max_y as f32;
        let center_y = (min_y + max_y) * 0.5;
        let half_height = (max_y - min_y) * 0.5;
        let vertical_distance = ((y as f32 - center_y) / half_height).abs();

        let mask_noise = value_noise_2d(
            entropy,
            self.mask_domain,
            x,
            z,
            definition.horizontal_scale,
        );
        let mask_unit = mask_noise * 0.5 + 0.5;
        let threshold = 1.0 - definition.coverage;
        let horizontal_support =
            ((mask_unit - threshold) / definition.coverage).clamp(0.0, 1.0);
        let detail_noise = value_noise_3d(
            entropy,
            self.detail_domain,
            x,
            y,
            z,
            definition.detail_scale,
            definition.detail_scale,
        );
        let raw_shape =
            horizontal_support - vertical_distance + detail_noise * definition.roughness;
        let blend_penalty = 1.0 - influence_weight.clamp(0.0, 1.0);
        (raw_shape - blend_penalty) * definition.density_scale
    }
}

#[derive(Clone, Debug)]
struct CaveField {
    primary_domain: GenerationDomain,
    secondary_domain: GenerationDomain,
}

impl CaveField {
    fn new() -> Self {
        Self {
            primary_domain: GenerationDomain::named(CAVE_PRIMARY_DOMAIN),
            secondary_domain: GenerationDomain::named(CAVE_SECONDARY_DOMAIN),
        }
    }

    /// Returns positive signed void density only inside the bounded cave field.
    ///
    /// The upper bound stays below the base surface so this subtractive
    /// contribution cannot punch through the authoritative top crossing.
    fn void_density(
        &self,
        entropy: GenerationEntropy,
        x: i32,
        y: i32,
        z: i32,
        base_surface: f32,
    ) -> f32 {
        let depth = base_surface - y as f32;
        if !(CAVE_MIN_DEPTH..=CAVE_MAX_DEPTH).contains(&depth) {
            return 0.0;
        }

        let primary = value_noise_3d(
            entropy,
            self.primary_domain,
            x,
            y,
            z,
            CAVE_HORIZONTAL_SCALE,
            CAVE_VERTICAL_SCALE,
        )
        .abs();
        let secondary = value_noise_3d(
            entropy,
            self.secondary_domain,
            x,
            y,
            z,
            CAVE_HORIZONTAL_SCALE,
            CAVE_VERTICAL_SCALE,
        )
        .abs();
        let noise_clearance = CAVE_NOISE_HALF_WIDTH - primary.max(secondary);
        if noise_clearance <= 0.0 {
            return 0.0;
        }

        let upper_clearance = (depth - CAVE_MIN_DEPTH) / CAVE_BOUNDARY_FADE;
        let lower_clearance = (CAVE_MAX_DEPTH - depth) / CAVE_BOUNDARY_FADE;
        let depth_envelope = upper_clearance.min(lower_clearance).clamp(0.0, 1.0);
        if depth_envelope <= 0.0 {
            return 0.0;
        }

        noise_clearance / CAVE_NOISE_HALF_WIDTH * depth_envelope * CAVE_DENSITY_SCALE
    }
}

impl TerrainField {
    pub(super) fn new(
        snapshot: &GenerationSnapshot,
        registry: &BiomeRegistry,
        biomes: Arc<BiomeLayout>,
        generated_fluids: Arc<GeneratedFluidField>,
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
            generated_fluids,
            rules,
            caves: CaveField::new(),
        }
    }

    pub(super) fn queries(&self) -> TerrainQueries<'_> {
        TerrainQueries { terrain: self }
    }

    fn column_at(&self, x: i32, z: i32) -> TerrainColumnSample {
        let sample = self.biomes.queries().surface_biome_at(x, z);
        let base_surface = self.base_surface_from_biome_sample(x, z, &sample);
        let surface_cut_depth = self.generated_fluids.surface_cut_depth_at(x, z, &sample);
        TerrainColumnSample {
            base_surface,
            surface_y: self.effective_surface_y(
                x,
                z,
                base_surface,
                surface_cut_depth,
                &sample,
            ),
            surface_cut_depth,
        }
    }

    fn base_surface_at(&self, x: i32, z: i32) -> f32 {
        let sample = self.biomes.queries().surface_biome_at(x, z);
        self.base_surface_from_biome_sample(x, z, &sample)
    }

    fn density_at(&self, x: i32, y: i32, z: i32) -> f32 {
        let sample = self.biomes.queries().surface_biome_at(x, z);
        let base_surface = self.base_surface_from_biome_sample(x, z, &sample);
        let surface_cut_depth = self.generated_fluids.surface_cut_depth_at(x, z, &sample);
        self.density_from_column(x, y, z, base_surface, surface_cut_depth, &sample)
    }

    fn density_from_column(
        &self,
        x: i32,
        y: i32,
        z: i32,
        base_surface: f32,
        surface_cut_depth: u32,
        sample: &BiomeSample,
    ) -> f32 {
        let base_density = base_surface - y as f32;
        let mut solid_density = base_density;

        for influence in sample.influences() {
            let rule = self.rules.get(influence.biome().as_str()).unwrap_or_else(|| {
                panic!(
                    "surface biome {} has no terrain rule",
                    influence.biome().as_str()
                )
            });
            if let Some(floating) = &rule.floating {
                solid_density = solid_density.max(floating.density(
                    self.entropy,
                    x,
                    y,
                    z,
                    influence.weight(),
                ));
            }
        }

        if surface_cut_depth > 0 {
            let cut_depth = i32::try_from(surface_cut_depth)
                .expect("validated generated surface fluid depth must fit i32");
            let base_surface_y = floor_to_world_y(base_surface);
            let cut_floor = base_surface_y.saturating_sub(cut_depth);
            if y > cut_floor && y <= base_surface_y {
                solid_density = solid_density.min(-SURFACE_FLUID_VOID_DENSITY);
            }
        }

        let cave_void = self
            .caves
            .void_density(self.entropy, x, y, z, base_surface);
        if cave_void > 0.0 {
            solid_density.min(-cave_void)
        } else {
            solid_density
        }
    }

    fn effective_surface_y(
        &self,
        x: i32,
        z: i32,
        base_surface: f32,
        surface_cut_depth: u32,
        sample: &BiomeSample,
    ) -> i32 {
        let cut_depth = i32::try_from(surface_cut_depth)
            .expect("validated generated surface fluid depth must fit i32");
        let base_surface_y = floor_to_world_y(base_surface).saturating_sub(cut_depth);
        let Some((candidate_min, candidate_max)) = self.additive_candidate_bounds(sample) else {
            return base_surface_y;
        };
        if candidate_max <= base_surface_y {
            return base_surface_y;
        }

        let search_min = candidate_min.max(base_surface_y.saturating_add(1));
        if search_min > candidate_max {
            return base_surface_y;
        }
        for y in (search_min..=candidate_max).rev() {
            if self.density_from_column(
                x,
                y,
                z,
                base_surface,
                surface_cut_depth,
                sample,
            ) >= 0.0
            {
                return y;
            }
        }
        base_surface_y
    }

    fn additive_candidate_bounds(&self, sample: &BiomeSample) -> Option<(i32, i32)> {
        let mut bounds: Option<(i32, i32)> = None;
        for influence in sample.influences() {
            if influence.weight() <= 0.0 {
                continue;
            }
            let rule = self.rules.get(influence.biome().as_str()).unwrap_or_else(|| {
                panic!(
                    "surface biome {} has no terrain rule",
                    influence.biome().as_str()
                )
            });
            let Some(floating) = &rule.floating else {
                continue;
            };
            let (min_y, max_y) = floating.vertical_bounds();
            bounds = Some(match bounds {
                Some((current_min, current_max)) => {
                    (current_min.min(min_y), current_max.max(max_y))
                }
                None => (min_y, max_y),
            });
        }
        bounds
    }

    fn sample_surface_grid(
        &self,
        origin_x: i32,
        origin_z: i32,
        width: u32,
        depth: u32,
        step: u32,
    ) -> TerrainAreaSample {
        assert!(
            width > 0 && depth > 0,
            "terrain sample area must be non-empty"
        );
        assert!(step > 0, "terrain sample step must be positive");
        validate_grid_extent(origin_x, width, step, "X");
        validate_grid_extent(origin_z, depth, step, "Z");
        let sample_count = checked_sample_count([width, depth], "terrain sample area");
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
                let surface_cut_depth = self
                    .generated_fluids
                    .surface_cut_depth_at(x, z, biome_sample);
                samples.push(TerrainColumnSample {
                    base_surface,
                    surface_y: self.effective_surface_y(
                        x,
                        z,
                        base_surface,
                        surface_cut_depth,
                        biome_sample,
                    ),
                    surface_cut_depth,
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

    fn sample_density_volume(
        &self,
        origin_x: i32,
        origin_y: i32,
        origin_z: i32,
        width: u32,
        height: u32,
        depth: u32,
    ) -> TerrainVolumeSample {
        assert!(
            width > 0 && height > 0 && depth > 0,
            "terrain density volume must be non-empty"
        );
        validate_grid_extent(origin_x, width, 1, "X");
        validate_grid_extent(origin_y, height, 1, "Y");
        validate_grid_extent(origin_z, depth, 1, "Z");
        let sample_count =
            checked_sample_count([width, height, depth], "terrain density volume");
        let biome_samples = self
            .biomes
            .queries()
            .sample_surface_area(origin_x, origin_z, width, depth);
        let column_count = checked_sample_count([width, depth], "terrain density columns");
        let mut base_surfaces = Vec::with_capacity(column_count);
        let mut surface_cut_depths = Vec::with_capacity(column_count);

        for z_index in 0..depth {
            let z = grid_axis(origin_z, z_index, 1);
            for x_index in 0..width {
                let x = grid_axis(origin_x, x_index, 1);
                let biome_sample = biome_samples
                    .sample_at(x_index, z_index)
                    .expect("matching biome sample grid must contain every density column");
                base_surfaces.push(self.base_surface_from_biome_sample(x, z, biome_sample));
                surface_cut_depths.push(
                    self.generated_fluids
                        .surface_cut_depth_at(x, z, biome_sample),
                );
            }
        }

        let mut densities = Vec::with_capacity(sample_count);
        let width_usize = width as usize;
        for z_index in 0..depth {
            let z = grid_axis(origin_z, z_index, 1);
            for y_index in 0..height {
                let y = grid_axis(origin_y, y_index, 1);
                for x_index in 0..width {
                    let x = grid_axis(origin_x, x_index, 1);
                    let column_index = z_index as usize * width_usize + x_index as usize;
                    let base_surface = base_surfaces[column_index];
                    let surface_cut_depth = surface_cut_depths[column_index];
                    let biome_sample = biome_samples
                        .sample_at(x_index, z_index)
                        .expect("matching biome sample grid must contain every density column");
                    densities.push(self.density_from_column(
                        x,
                        y,
                        z,
                        base_surface,
                        surface_cut_depth,
                        biome_sample,
                    ));
                }
            }
        }

        TerrainVolumeSample {
            origin_x,
            origin_y,
            origin_z,
            width,
            height,
            depth,
            densities,
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

fn value_noise_3d(
    entropy: GenerationEntropy,
    domain: GenerationDomain,
    x: i32,
    y: i32,
    z: i32,
    horizontal_scale: u32,
    vertical_scale: u32,
) -> f32 {
    let horizontal_scale_i64 = i64::from(horizontal_scale);
    let vertical_scale_i64 = i64::from(vertical_scale);
    let x_i64 = i64::from(x);
    let y_i64 = i64::from(y);
    let z_i64 = i64::from(z);

    let cell_x = x_i64.div_euclid(horizontal_scale_i64);
    let cell_y = y_i64.div_euclid(vertical_scale_i64);
    let cell_z = z_i64.div_euclid(horizontal_scale_i64);
    let fraction_x =
        x_i64.rem_euclid(horizontal_scale_i64) as f32 / horizontal_scale as f32;
    let fraction_y = y_i64.rem_euclid(vertical_scale_i64) as f32 / vertical_scale as f32;
    let fraction_z =
        z_i64.rem_euclid(horizontal_scale_i64) as f32 / horizontal_scale as f32;
    let smooth_x = smoothstep(fraction_x);
    let smooth_y = smoothstep(fraction_y);
    let smooth_z = smoothstep(fraction_z);

    let x0 = i32::try_from(cell_x).expect("terrain noise X lattice must fit i32");
    let y0 = i32::try_from(cell_y).expect("terrain noise Y lattice must fit i32");
    let z0 = i32::try_from(cell_z).expect("terrain noise Z lattice must fit i32");
    let x1 = x0
        .checked_add(1)
        .expect("terrain noise X lattice neighbor must fit i32");
    let y1 = y0
        .checked_add(1)
        .expect("terrain noise Y lattice neighbor must fit i32");
    let z1 = z0
        .checked_add(1)
        .expect("terrain noise Z lattice neighbor must fit i32");

    let n000 = entropy_value(entropy.sample_3d(domain, GenerationPoint3::new(x0, y0, z0)));
    let n100 = entropy_value(entropy.sample_3d(domain, GenerationPoint3::new(x1, y0, z0)));
    let n010 = entropy_value(entropy.sample_3d(domain, GenerationPoint3::new(x0, y1, z0)));
    let n110 = entropy_value(entropy.sample_3d(domain, GenerationPoint3::new(x1, y1, z0)));
    let n001 = entropy_value(entropy.sample_3d(domain, GenerationPoint3::new(x0, y0, z1)));
    let n101 = entropy_value(entropy.sample_3d(domain, GenerationPoint3::new(x1, y0, z1)));
    let n011 = entropy_value(entropy.sample_3d(domain, GenerationPoint3::new(x0, y1, z1)));
    let n111 = entropy_value(entropy.sample_3d(domain, GenerationPoint3::new(x1, y1, z1)));

    let nx00 = lerp(n000, n100, smooth_x);
    let nx10 = lerp(n010, n110, smooth_x);
    let nx01 = lerp(n001, n101, smooth_x);
    let nx11 = lerp(n011, n111, smooth_x);
    let nxy0 = lerp(nx00, nx10, smooth_y);
    let nxy1 = lerp(nx01, nx11, smooth_y);
    lerp(nxy0, nxy1, smooth_z)
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

fn checked_sample_count<const N: usize>(extents: [u32; N], label: &str) -> usize {
    extents
        .into_iter()
        .try_fold(1_u64, |count, extent| count.checked_mul(u64::from(extent)))
        .and_then(|count| usize::try_from(count).ok())
        .unwrap_or_else(|| panic!("{label} is too large"))
}

#[cfg(test)]
mod tests {
    use super::super::foundation::{GenerationDimension, GenerationSeed};
    use super::super::generated_fluid::GeneratedFluidField;
    use super::*;
    use crate::content::biome::BiomeDefinition;

    fn field_from_definition(seed: u64, definition: BiomeDefinition) -> TerrainField {
        let mut registry = BiomeRegistry::default();
        registry.insert(definition);
        let snapshot = GenerationSnapshot::new(
            GenerationSeed::new(seed),
            GenerationDimension::new("asteria:test", 64, 1.0),
        );
        let biomes = Arc::new(BiomeLayout::new(&snapshot, &registry));
        let generated_fluids = Arc::new(GeneratedFluidField::new(&snapshot, &registry, None, &[]));
        TerrainField::new(&snapshot, &registry, biomes, generated_fluids)
    }

    fn base_definition() -> BiomeDefinition {
        serde_json::from_value(serde_json::json!({
            "id": "asteria:test/base",
            "name": {
                "english": "Base",
                "portuguese_brazil": "Base",
                "spanish": "Base"
            },
            "surfaceLayout": {},
            "surfaceTerrain": {
                "baseHeightOffset": 8.0,
                "macroAmplitude": 12.0,
                "macroScale": 256,
                "detailAmplitude": 3.0,
                "detailScale": 64
            }
        }))
        .expect("test biome must deserialize")
    }

    fn floating_definition() -> BiomeDefinition {
        serde_json::from_value(serde_json::json!({
            "id": "asteria:test/base",
            "name": {
                "english": "Base",
                "portuguese_brazil": "Base",
                "spanish": "Base"
            },
            "surfaceLayout": {},
            "surfaceTerrain": {
                "baseHeightOffset": 8.0,
                "macroAmplitude": 12.0,
                "macroScale": 256,
                "detailAmplitude": 3.0,
                "detailScale": 64
            },
            "terrain3d": {
                "floatingFormation": {
                    "minY": 120,
                    "maxY": 152,
                    "horizontalScale": 64,
                    "detailScale": 24,
                    "coverage": 1.0,
                    "roughness": 0.0,
                    "densityScale": 32.0
                }
            }
        }))
        .expect("floating test biome must deserialize")
    }

    fn test_field(seed: u64) -> TerrainField {
        field_from_definition(seed, base_definition())
    }

    fn floating_field(seed: u64) -> TerrainField {
        field_from_definition(seed, floating_definition())
    }

    #[test]
    fn density_volume_matches_scalar_queries() {
        let field = floating_field(71);
        let queries = field.queries();
        let volume = queries.sample_density_volume(-9, 110, 13, 11, 50, 7);

        for z_index in 0..volume.depth() {
            for y_index in 0..volume.height() {
                for x_index in 0..volume.width() {
                    let x = volume.origin().0 + x_index as i32;
                    let y = volume.origin().1 + y_index as i32;
                    let z = volume.origin().2 + z_index as i32;
                    assert_eq!(
                        volume.density_at(x_index, y_index, z_index),
                        Some(queries.density_at(x, y, z))
                    );
                }
            }
        }
    }

    #[test]
    fn caves_never_change_the_surface_crossing_without_additive_terrain() {
        let field = test_field(93);
        let queries = field.queries();

        for z in (-96..=96).step_by(24) {
            for x in (-96..=96).step_by(24) {
                let base_surface = queries.base_surface_at(x, z);
                let surface_y = queries.surface_at(x, z);
                assert_eq!(surface_y, floor_to_world_y(base_surface));
                assert!(queries.density_at(x, surface_y, z) >= 0.0);
            }
        }
    }

    #[test]
    fn cave_carving_is_bounded_below_the_base_surface() {
        let field = test_field(104);
        let queries = field.queries();
        let x = 17;
        let z = -29;
        let base_surface = queries.base_surface_at(x, z);

        for depth in [
            0.0_f32,
            4.0,
            CAVE_MIN_DEPTH,
            CAVE_MAX_DEPTH,
            CAVE_MAX_DEPTH + 1.0,
        ] {
            let y = floor_to_world_y(base_surface - depth);
            if depth < CAVE_MIN_DEPTH || depth > CAVE_MAX_DEPTH {
                assert_eq!(
                    queries.density_at(x, y, z),
                    base_surface - y as f32,
                    "out-of-band cave field must leave base density untouched"
                );
            }
        }
    }

    #[test]
    fn floating_formation_extends_effective_surface_within_candidate_bounds() {
        let field = floating_field(211);
        let queries = field.queries();
        let mut found_floating_surface = false;

        for z in (-128..=128).step_by(16) {
            for x in (-128..=128).step_by(16) {
                let base_surface = queries.base_surface_at(x, z);
                let base_y = floor_to_world_y(base_surface);
                let surface_y = queries.surface_at(x, z);
                assert!(surface_y <= 152);
                if surface_y > base_y {
                    found_floating_surface = true;
                    assert!(surface_y >= 120);
                    assert!(queries.density_at(x, surface_y, z) >= 0.0);
                    assert_eq!(queries.base_surface_at(x, z), base_surface);
                }
            }
        }

        assert!(
            found_floating_surface,
            "authored floating terrain must produce at least one additive surface"
        );
    }

    #[test]
    fn effective_surface_grid_matches_scalar_queries() {
        let field = floating_field(307);
        let queries = field.queries();
        let grid = queries.sample_surface_grid(-64, 48, 9, 7, 8);

        for z_index in 0..grid.depth() {
            for x_index in 0..grid.width() {
                let x = grid.origin().0 + x_index as i32 * grid.step() as i32;
                let z = grid.origin().1 + z_index as i32 * grid.step() as i32;
                let sample = grid
                    .sample_at(x_index, z_index)
                    .expect("terrain grid must contain requested sample");
                assert_eq!(sample.surface_y(), queries.surface_at(x, z));
                assert_eq!(sample.base_surface(), queries.base_surface_at(x, z));
            }
        }
    }
}
