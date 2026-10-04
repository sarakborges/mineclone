use crate::content::dimension::DimensionDefinition;

const DEFAULT_AREA_BATCH_WIDTH: u32 = 32;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct GenerationSeed(u64);

impl GenerationSeed {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(crate) const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct GenerationPoint2 {
    x: i32,
    z: i32,
}

impl GenerationPoint2 {
    pub(crate) const ZERO: Self = Self::new(0, 0);

    pub(crate) const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    pub(crate) const fn x(self) -> i32 {
        self.x
    }

    pub(crate) const fn z(self) -> i32 {
        self.z
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct GenerationPoint3 {
    x: i32,
    y: i32,
    z: i32,
}

impl GenerationPoint3 {
    pub(crate) const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub(crate) const fn x(self) -> i32 {
        self.x
    }

    pub(crate) const fn y(self) -> i32 {
        self.y
    }

    pub(crate) const fn z(self) -> i32 {
        self.z
    }
}

/// Immutable dimension inputs owned by the rebuilt generator.
///
/// Authored content is copied at the boundary; the generator keeps no live
/// reference to the legacy/runtime dimension registry or its identity types.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GenerationDimension {
    id: String,
    sea_level: i32,
    gravity_strength: f32,
}

impl GenerationDimension {
    pub(crate) fn new(id: impl Into<String>, sea_level: i32, gravity_strength: f32) -> Self {
        let id = id.into();
        assert!(!id.trim().is_empty(), "generation dimension id cannot be empty");
        assert!(
            gravity_strength.is_finite() && gravity_strength >= 0.0,
            "generation dimension gravity must be finite and nonnegative"
        );
        Self {
            id,
            sea_level,
            gravity_strength,
        }
    }

    pub(crate) fn from_definition(definition: &DimensionDefinition) -> Self {
        Self::new(
            definition.id.clone(),
            definition.sea_level,
            definition.gravity_strength,
        )
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) const fn sea_level(&self) -> i32 {
        self.sea_level
    }

    pub(crate) const fn gravity_strength(&self) -> f32 {
        self.gravity_strength
    }
}

/// Frozen generated-world inputs. A live generator never observes registry or
/// runtime mutation through shared state.
#[derive(Clone, Debug)]
pub(crate) struct GenerationSnapshot {
    seed: GenerationSeed,
    dimension: GenerationDimension,
}

impl GenerationSnapshot {
    pub(crate) const fn new(seed: GenerationSeed, dimension: GenerationDimension) -> Self {
        Self { seed, dimension }
    }

    pub(crate) const fn seed(&self) -> GenerationSeed {
        self.seed
    }

    pub(crate) const fn dimension(&self) -> &GenerationDimension {
        &self.dimension
    }
}

/// Stable semantic salt for one generation algorithm/domain.
///
/// Domain names affect generated-world semantics. Cache sizes, request sizes,
/// task order, and batch widths never participate in this value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct GenerationDomain(u64);

impl GenerationDomain {
    pub(crate) fn named(name: &str) -> Self {
        assert!(!name.trim().is_empty(), "generation domain cannot be empty");
        Self(avalanche(hash_text(name)))
    }
}

/// Rectangular X/Z world-space request. `width` grows along X and `depth`
/// grows along Z. Origin is inclusive and the opposite edge is exclusive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SampleArea2d {
    origin: GenerationPoint2,
    width: u32,
    depth: u32,
}

impl SampleArea2d {
    pub(crate) fn new(origin: GenerationPoint2, width: u32, depth: u32) -> Self {
        assert!(width > 0 && depth > 0, "sample area must be non-empty");

        let last_x = origin.x as i64 + width as i64 - 1;
        let last_z = origin.z as i64 + depth as i64 - 1;
        assert!(
            last_x <= i32::MAX as i64 && last_z <= i32::MAX as i64,
            "sample area exceeds world coordinate range"
        );

        let sample_count = (width as u64)
            .checked_mul(depth as u64)
            .and_then(|count| usize::try_from(count).ok());
        assert!(sample_count.is_some(), "sample area is too large to address");

        Self {
            origin,
            width,
            depth,
        }
    }

    pub(crate) const fn origin(self) -> GenerationPoint2 {
        self.origin
    }

    pub(crate) const fn width(self) -> u32 {
        self.width
    }

    pub(crate) const fn depth(self) -> u32 {
        self.depth
    }

    fn sample_count(self) -> usize {
        usize::try_from(self.width as u64 * self.depth as u64)
            .expect("validated sample area count must fit usize")
    }

    fn position_at(self, x_offset: u32, z_offset: u32) -> GenerationPoint2 {
        debug_assert!(x_offset < self.width && z_offset < self.depth);
        GenerationPoint2::new(
            i32::try_from(self.origin.x as i64 + x_offset as i64)
                .expect("validated X sample must fit i32"),
            i32::try_from(self.origin.z as i64 + z_offset as i64)
                .expect("validated Z sample must fit i32"),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SampleGrid2d {
    area: SampleArea2d,
    values: Vec<u64>,
}

impl SampleGrid2d {
    fn new(area: SampleArea2d, values: Vec<u64>) -> Self {
        debug_assert_eq!(area.sample_count(), values.len());
        Self { area, values }
    }

    pub(crate) const fn area(&self) -> SampleArea2d {
        self.area
    }

    pub(crate) fn get(&self, position: GenerationPoint2) -> Option<u64> {
        let x = position.x as i64 - self.area.origin.x as i64;
        let z = position.z as i64 - self.area.origin.z as i64;
        if x < 0 || z < 0 || x >= self.area.width as i64 || z >= self.area.depth as i64 {
            return None;
        }

        let index = z as usize * self.area.width as usize + x as usize;
        self.values.get(index).copied()
    }

    #[cfg(test)]
    fn values(&self) -> &[u64] {
        &self.values
    }
}

/// Stateless entropy owned entirely by the rebuilt generator.
///
/// This is intentionally not a gameplay-facing query API. Biome, terrain, and
/// structure owners will consume it behind their own capabilities.
#[derive(Clone, Copy, Debug)]
pub(super) struct GenerationEntropy {
    root: u64,
}

impl GenerationEntropy {
    pub(super) fn new(snapshot: &GenerationSnapshot) -> Self {
        let dimension = avalanche(hash_text(snapshot.dimension.id()));
        Self {
            root: avalanche(snapshot.seed.value() ^ dimension.rotate_left(17)),
        }
    }

    pub(super) fn sample_2d(self, domain: GenerationDomain, position: GenerationPoint2) -> u64 {
        let domain_seed = avalanche(self.root ^ domain.0);
        avalanche(mix_components(
            domain_seed,
            [position.x() as u32, position.z() as u32],
        ))
    }

    pub(super) fn sample_3d(self, domain: GenerationDomain, position: GenerationPoint3) -> u64 {
        let domain_seed = avalanche(self.root ^ domain.0);
        avalanche(mix_components(
            domain_seed,
            [position.x() as u32, position.y() as u32, position.z() as u32],
        ))
    }

    pub(super) fn sample_area_2d(
        self,
        domain: GenerationDomain,
        area: SampleArea2d,
    ) -> SampleGrid2d {
        self.sample_area_2d_with_batch_width(domain, area, DEFAULT_AREA_BATCH_WIDTH)
    }

    fn sample_area_2d_with_batch_width(
        self,
        domain: GenerationDomain,
        area: SampleArea2d,
        batch_width: u32,
    ) -> SampleGrid2d {
        assert!(batch_width > 0, "generation batch width must be non-zero");
        let mut values = vec![0; area.sample_count()];

        for z_offset in 0..area.depth {
            let mut batch_start = 0;
            while batch_start < area.width {
                let batch_end = batch_start.saturating_add(batch_width).min(area.width);
                for x_offset in batch_start..batch_end {
                    let index = z_offset as usize * area.width as usize + x_offset as usize;
                    values[index] = self.sample_2d(domain, area.position_at(x_offset, z_offset));
                }
                batch_start = batch_end;
            }
        }

        SampleGrid2d::new(area, values)
    }
}

fn hash_text(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn mix_components(mut hash: u64, components: impl IntoIterator<Item = u32>) -> u64 {
    for component in components {
        hash ^= component as u64;
        hash = hash.wrapping_mul(0x9e37_79b1_85eb_ca87);
        hash ^= hash >> 31;
    }
    hash
}

fn avalanche(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entropy(seed: u64, dimension: &str) -> GenerationEntropy {
        GenerationEntropy::new(&GenerationSnapshot::new(
            GenerationSeed::new(seed),
            GenerationDimension::new(dimension, 64, 1.0),
        ))
    }

    #[test]
    fn direct_far_coordinate_query_is_order_independent() {
        let entropy = entropy(42, "asteria:overworld");
        let domain = GenerationDomain::named("direct-coordinate");
        let far = GenerationPoint2::new(1_500_000_000, -1_500_000_000);
        let direct = entropy.sample_2d(domain, far);

        for point in [
            GenerationPoint2::ZERO,
            GenerationPoint2::new(16, 16),
            GenerationPoint2::new(-4096, 8192),
            GenerationPoint2::new(999_999, -777_777),
        ] {
            let _ = entropy.sample_2d(domain, point);
        }

        assert_eq!(entropy.sample_2d(domain, far), direct);
    }

    #[test]
    fn scalar_and_area_sampling_are_equivalent() {
        let entropy = entropy(77, "asteria:overworld");
        let domain = GenerationDomain::named("scalar-area-equivalence");
        let area = SampleArea2d::new(GenerationPoint2::new(-37, 91), 13, 9);
        let grid = entropy.sample_area_2d(domain, area);

        for z_offset in 0..area.depth() {
            for x_offset in 0..area.width() {
                let position = area.position_at(x_offset, z_offset);
                assert_eq!(grid.get(position), Some(entropy.sample_2d(domain, position)));
            }
        }
    }

    #[test]
    fn internal_batch_width_cannot_change_results() {
        let entropy = entropy(1234, "asteria:umbral");
        let domain = GenerationDomain::named("batch-equivalence");
        let area = SampleArea2d::new(GenerationPoint2::new(-128, -64), 67, 19);

        let scalar_batches = entropy.sample_area_2d_with_batch_width(domain, area, 1);
        let uneven_batches = entropy.sample_area_2d_with_batch_width(domain, area, 7);
        let wide_batches = entropy.sample_area_2d_with_batch_width(domain, area, 128);

        assert_eq!(scalar_batches.values(), uneven_batches.values());
        assert_eq!(scalar_batches.values(), wide_batches.values());
    }

    #[test]
    fn dimensions_have_independent_entropy_domains() {
        let domain = GenerationDomain::named("dimension-separation");
        let point = GenerationPoint3::new(200, 70, -400);

        assert_ne!(
            entropy(99, "asteria:overworld").sample_3d(domain, point),
            entropy(99, "asteria:umbral").sample_3d(domain, point)
        );
    }

    #[test]
    fn area_grid_rejects_points_outside_requested_bounds() {
        let entropy = entropy(5, "asteria:overworld");
        let area = SampleArea2d::new(GenerationPoint2::new(10, 20), 2, 3);
        let grid = entropy.sample_area_2d(GenerationDomain::named("bounds"), area);

        assert_eq!(grid.area(), area);
        assert!(grid.get(GenerationPoint2::new(9, 20)).is_none());
        assert!(grid.get(GenerationPoint2::new(12, 20)).is_none());
        assert!(grid.get(GenerationPoint2::new(10, 23)).is_none());
    }
}
