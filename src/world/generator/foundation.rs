use crate::content::dimension::DimensionDefinition;

const DEFAULT_AREA_BATCH_WIDTH: u32 = 32;
const DEFAULT_VOLUME_BATCH_WIDTH: u32 = 16;
pub(super) const GENERATION_CHUNK_EDGE: i32 = 16;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct GenerationSeed(u64);

impl GenerationSeed {
    pub(super) const fn new(value: u64) -> Self {
        Self(value)
    }

    pub(super) const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct GenerationPoint2 {
    x: i32,
    z: i32,
}

impl GenerationPoint2 {
    pub(super) const ZERO: Self = Self::new(0, 0);

    pub(super) const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    pub(super) const fn x(self) -> i32 {
        self.x
    }

    pub(super) const fn z(self) -> i32 {
        self.z
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct GenerationPoint3 {
    x: i32,
    y: i32,
    z: i32,
}

impl GenerationPoint3 {
    pub(super) const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub(super) const fn x(self) -> i32 {
        self.x
    }

    pub(super) const fn y(self) -> i32 {
        self.y
    }

    pub(super) const fn z(self) -> i32 {
        self.z
    }
}

/// Generator-owned identity for the runtime materialization unit.
///
/// The coordinate type is intentionally independent from the voxel runtime's
/// chunk wrappers. Adapters at the materialization boundary translate between
/// the two domains when chunk synthesis is implemented.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct GenerationChunkCoord {
    x: i32,
    y: i32,
    z: i32,
}

impl GenerationChunkCoord {
    pub(super) const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub(super) fn from_world(position: GenerationPoint3) -> Self {
        Self::new(
            position.x().div_euclid(GENERATION_CHUNK_EDGE),
            position.y().div_euclid(GENERATION_CHUNK_EDGE),
            position.z().div_euclid(GENERATION_CHUNK_EDGE),
        )
    }

    pub(super) fn world_bounds(self) -> Option<GenerationChunkBounds> {
        Some(GenerationChunkBounds {
            origin: GenerationPoint3::new(
                self.x.checked_mul(GENERATION_CHUNK_EDGE)?,
                self.y.checked_mul(GENERATION_CHUNK_EDGE)?,
                self.z.checked_mul(GENERATION_CHUNK_EDGE)?,
            ),
        })
    }
}

/// World-space bounds of one future 16x16x16 materialization request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct GenerationChunkBounds {
    origin: GenerationPoint3,
}

impl GenerationChunkBounds {
    pub(super) const fn origin(self) -> GenerationPoint3 {
        self.origin
    }

    pub(super) fn contains(self, position: GenerationPoint3) -> bool {
        let dx = i64::from(position.x()) - i64::from(self.origin.x());
        let dy = i64::from(position.y()) - i64::from(self.origin.y());
        let dz = i64::from(position.z()) - i64::from(self.origin.z());
        let edge = i64::from(GENERATION_CHUNK_EDGE);
        (0..edge).contains(&dx) && (0..edge).contains(&dy) && (0..edge).contains(&dz)
    }
}

/// Immutable dimension inputs owned by the rebuilt generator.
///
/// Authored content is copied at the boundary; the generator keeps no live
/// reference to runtime dimension state or registry identity wrappers.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct GenerationDimension {
    id: String,
    sea_level: i32,
    gravity_strength: f32,
}

impl GenerationDimension {
    pub(super) fn new(id: impl Into<String>, sea_level: i32, gravity_strength: f32) -> Self {
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

    pub(super) fn from_definition(definition: &DimensionDefinition) -> Self {
        Self::new(
            definition.id.clone(),
            definition.sea_level,
            definition.gravity_strength,
        )
    }

    pub(super) fn id(&self) -> &str {
        &self.id
    }

    pub(super) const fn sea_level(&self) -> i32 {
        self.sea_level
    }

    pub(super) const fn gravity_strength(&self) -> f32 {
        self.gravity_strength
    }
}

/// Frozen generated-world inputs. A live generator never observes registry or
/// runtime mutation through shared state.
#[derive(Clone, Debug)]
pub(super) struct GenerationSnapshot {
    seed: GenerationSeed,
    dimension: GenerationDimension,
}

impl GenerationSnapshot {
    pub(super) const fn new(seed: GenerationSeed, dimension: GenerationDimension) -> Self {
        Self { seed, dimension }
    }

    pub(super) const fn seed(&self) -> GenerationSeed {
        self.seed
    }

    pub(super) const fn dimension(&self) -> &GenerationDimension {
        &self.dimension
    }
}

/// Stable semantic salt for one generation algorithm/domain.
///
/// Domain names affect generated-world semantics. Cache sizes, request sizes,
/// task order, and batch widths never participate in this value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct GenerationDomain(u64);

impl GenerationDomain {
    pub(super) fn named(name: &str) -> Self {
        assert!(!name.trim().is_empty(), "generation domain cannot be empty");
        Self(avalanche(hash_text(name)))
    }
}

/// Rectangular X/Z world-space request. `width` grows along X and `depth`
/// grows along Z. Origin is inclusive and the opposite edge is exclusive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SampleArea2d {
    origin: GenerationPoint2,
    width: u32,
    depth: u32,
}

impl SampleArea2d {
    pub(super) fn new(origin: GenerationPoint2, width: u32, depth: u32) -> Self {
        assert!(width > 0 && depth > 0, "sample area must be non-empty");
        validate_axis_extent(origin.x(), width, "X");
        validate_axis_extent(origin.z(), depth, "Z");
        assert!(
            checked_sample_count([width, depth]).is_some(),
            "sample area is too large to address"
        );
        Self {
            origin,
            width,
            depth,
        }
    }

    pub(super) const fn origin(self) -> GenerationPoint2 {
        self.origin
    }

    pub(super) const fn width(self) -> u32 {
        self.width
    }

    pub(super) const fn depth(self) -> u32 {
        self.depth
    }

    fn sample_count(self) -> usize {
        checked_sample_count([self.width, self.depth])
            .expect("validated sample area count must fit usize")
    }

    fn position_at(self, x_offset: u32, z_offset: u32) -> GenerationPoint2 {
        debug_assert!(x_offset < self.width && z_offset < self.depth);
        GenerationPoint2::new(
            offset_axis(self.origin.x(), x_offset),
            offset_axis(self.origin.z(), z_offset),
        )
    }
}

/// Rectangular XYZ request used by future density/volume owners. All extents
/// are non-zero; origin is inclusive and the opposite faces are exclusive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SampleVolume3d {
    origin: GenerationPoint3,
    width: u32,
    height: u32,
    depth: u32,
}

impl SampleVolume3d {
    pub(super) fn new(origin: GenerationPoint3, width: u32, height: u32, depth: u32) -> Self {
        assert!(
            width > 0 && height > 0 && depth > 0,
            "sample volume must be non-empty"
        );
        validate_axis_extent(origin.x(), width, "X");
        validate_axis_extent(origin.y(), height, "Y");
        validate_axis_extent(origin.z(), depth, "Z");
        assert!(
            checked_sample_count([width, height, depth]).is_some(),
            "sample volume is too large to address"
        );
        Self {
            origin,
            width,
            height,
            depth,
        }
    }

    pub(super) const fn origin(self) -> GenerationPoint3 {
        self.origin
    }

    pub(super) const fn width(self) -> u32 {
        self.width
    }

    pub(super) const fn height(self) -> u32 {
        self.height
    }

    pub(super) const fn depth(self) -> u32 {
        self.depth
    }

    fn sample_count(self) -> usize {
        checked_sample_count([self.width, self.height, self.depth])
            .expect("validated sample volume count must fit usize")
    }

    fn position_at(self, x_offset: u32, y_offset: u32, z_offset: u32) -> GenerationPoint3 {
        debug_assert!(
            x_offset < self.width && y_offset < self.height && z_offset < self.depth
        );
        GenerationPoint3::new(
            offset_axis(self.origin.x(), x_offset),
            offset_axis(self.origin.y(), y_offset),
            offset_axis(self.origin.z(), z_offset),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SampleGrid2d {
    area: SampleArea2d,
    values: Vec<u64>,
}

impl SampleGrid2d {
    fn new(area: SampleArea2d, values: Vec<u64>) -> Self {
        debug_assert_eq!(area.sample_count(), values.len());
        Self { area, values }
    }

    pub(super) const fn area(&self) -> SampleArea2d {
        self.area
    }

    pub(super) fn get(&self, position: GenerationPoint2) -> Option<u64> {
        let x = i64::from(position.x()) - i64::from(self.area.origin.x());
        let z = i64::from(position.z()) - i64::from(self.area.origin.z());
        if x < 0 || z < 0 || x >= i64::from(self.area.width) || z >= i64::from(self.area.depth) {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SampleGrid3d {
    volume: SampleVolume3d,
    values: Vec<u64>,
}

impl SampleGrid3d {
    fn new(volume: SampleVolume3d, values: Vec<u64>) -> Self {
        debug_assert_eq!(volume.sample_count(), values.len());
        Self { volume, values }
    }

    pub(super) const fn volume(&self) -> SampleVolume3d {
        self.volume
    }

    pub(super) fn get(&self, position: GenerationPoint3) -> Option<u64> {
        let x = i64::from(position.x()) - i64::from(self.volume.origin.x());
        let y = i64::from(position.y()) - i64::from(self.volume.origin.y());
        let z = i64::from(position.z()) - i64::from(self.volume.origin.z());
        if x < 0
            || y < 0
            || z < 0
            || x >= i64::from(self.volume.width)
            || y >= i64::from(self.volume.height)
            || z >= i64::from(self.volume.depth)
        {
            return None;
        }

        let width = self.volume.width as usize;
        let height = self.volume.height as usize;
        let index = (z as usize * height + y as usize) * width + x as usize;
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
/// structure owners consume it behind their own semantic capabilities.
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

    pub(super) fn sample_volume_3d(
        self,
        domain: GenerationDomain,
        volume: SampleVolume3d,
    ) -> SampleGrid3d {
        self.sample_volume_3d_with_batch_width(domain, volume, DEFAULT_VOLUME_BATCH_WIDTH)
    }

    fn sample_volume_3d_with_batch_width(
        self,
        domain: GenerationDomain,
        volume: SampleVolume3d,
        batch_width: u32,
    ) -> SampleGrid3d {
        assert!(batch_width > 0, "generation batch width must be non-zero");
        let mut values = vec![0; volume.sample_count()];
        let width = volume.width as usize;
        let height = volume.height as usize;

        for z_offset in 0..volume.depth {
            for y_offset in 0..volume.height {
                let mut batch_start = 0;
                while batch_start < volume.width {
                    let batch_end = batch_start.saturating_add(batch_width).min(volume.width);
                    for x_offset in batch_start..batch_end {
                        let index = (z_offset as usize * height + y_offset as usize) * width
                            + x_offset as usize;
                        values[index] = self.sample_3d(
                            domain,
                            volume.position_at(x_offset, y_offset, z_offset),
                        );
                    }
                    batch_start = batch_end;
                }
            }
        }

        SampleGrid3d::new(volume, values)
    }
}

fn validate_axis_extent(origin: i32, extent: u32, axis: &str) {
    let last = i64::from(origin) + i64::from(extent) - 1;
    assert!(
        last <= i64::from(i32::MAX),
        "sample {axis} extent exceeds world coordinate range"
    );
}

fn checked_sample_count<const N: usize>(extents: [u32; N]) -> Option<usize> {
    extents
        .into_iter()
        .try_fold(1_u64, |count, extent| count.checked_mul(u64::from(extent)))
        .and_then(|count| usize::try_from(count).ok())
}

fn offset_axis(origin: i32, offset: u32) -> i32 {
    i32::try_from(i64::from(origin) + i64::from(offset))
        .expect("validated sample coordinate must fit i32")
}

fn hash_text(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn mix_components(mut hash: u64, components: impl IntoIterator<Item = u32>) -> u64 {
    for component in components {
        hash ^= u64::from(component);
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
    fn scalar_and_volume_sampling_are_equivalent() {
        let entropy = entropy(81, "asteria:overworld");
        let domain = GenerationDomain::named("scalar-volume-equivalence");
        let volume = SampleVolume3d::new(GenerationPoint3::new(-9, 48, 21), 7, 5, 6);
        let grid = entropy.sample_volume_3d(domain, volume);

        for z_offset in 0..volume.depth() {
            for y_offset in 0..volume.height() {
                for x_offset in 0..volume.width() {
                    let position = volume.position_at(x_offset, y_offset, z_offset);
                    assert_eq!(grid.get(position), Some(entropy.sample_3d(domain, position)));
                }
            }
        }
    }

    #[test]
    fn internal_batch_width_cannot_change_results() {
        let entropy = entropy(1234, "asteria:umbral");
        let area_domain = GenerationDomain::named("area-batch-equivalence");
        let area = SampleArea2d::new(GenerationPoint2::new(-128, -64), 67, 19);
        let scalar_batches = entropy.sample_area_2d_with_batch_width(area_domain, area, 1);
        let uneven_batches = entropy.sample_area_2d_with_batch_width(area_domain, area, 7);
        let wide_batches = entropy.sample_area_2d_with_batch_width(area_domain, area, 128);
        assert_eq!(scalar_batches.values(), uneven_batches.values());
        assert_eq!(scalar_batches.values(), wide_batches.values());

        let volume_domain = GenerationDomain::named("volume-batch-equivalence");
        let volume = SampleVolume3d::new(GenerationPoint3::new(-24, 0, 31), 19, 7, 11);
        let scalar_batches = entropy.sample_volume_3d_with_batch_width(volume_domain, volume, 1);
        let uneven_batches = entropy.sample_volume_3d_with_batch_width(volume_domain, volume, 5);
        let wide_batches = entropy.sample_volume_3d_with_batch_width(volume_domain, volume, 64);
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
    fn area_and_volume_grids_reject_points_outside_requested_bounds() {
        let entropy = entropy(5, "asteria:overworld");
        let area = SampleArea2d::new(GenerationPoint2::new(10, 20), 2, 3);
        let area_grid = entropy.sample_area_2d(GenerationDomain::named("area-bounds"), area);
        assert_eq!(area_grid.area(), area);
        assert!(area_grid.get(GenerationPoint2::new(9, 20)).is_none());
        assert!(area_grid.get(GenerationPoint2::new(12, 20)).is_none());
        assert!(area_grid.get(GenerationPoint2::new(10, 23)).is_none());

        let volume = SampleVolume3d::new(GenerationPoint3::new(3, 4, 5), 2, 3, 4);
        let volume_grid =
            entropy.sample_volume_3d(GenerationDomain::named("volume-bounds"), volume);
        assert_eq!(volume_grid.volume(), volume);
        assert!(volume_grid.get(GenerationPoint3::new(2, 4, 5)).is_none());
        assert!(volume_grid.get(GenerationPoint3::new(3, 7, 5)).is_none());
        assert!(volume_grid.get(GenerationPoint3::new(3, 4, 9)).is_none());
    }

    #[test]
    fn chunk_coordinates_partition_negative_and_positive_world_space() {
        let cases = [
            (GenerationPoint3::new(0, 0, 0), GenerationChunkCoord::new(0, 0, 0)),
            (GenerationPoint3::new(15, 15, 15), GenerationChunkCoord::new(0, 0, 0)),
            (GenerationPoint3::new(16, 16, 16), GenerationChunkCoord::new(1, 1, 1)),
            (GenerationPoint3::new(-1, -1, -1), GenerationChunkCoord::new(-1, -1, -1)),
            (
                GenerationPoint3::new(-16, -16, -16),
                GenerationChunkCoord::new(-1, -1, -1),
            ),
            (
                GenerationPoint3::new(-17, -17, -17),
                GenerationChunkCoord::new(-2, -2, -2),
            ),
        ];

        for (position, expected) in cases {
            let coord = GenerationChunkCoord::from_world(position);
            assert_eq!(coord, expected);
            assert!(
                coord
                    .world_bounds()
                    .expect("world-derived chunk must have representable bounds")
                    .contains(position)
            );
        }
    }

    #[test]
    fn chunk_bounds_are_exactly_one_materialization_unit() {
        let bounds = GenerationChunkCoord::new(-2, 3, 5)
            .world_bounds()
            .expect("ordinary chunk bounds must be representable");
        assert_eq!(bounds.origin(), GenerationPoint3::new(-32, 48, 80));
        assert!(bounds.contains(GenerationPoint3::new(-32, 48, 80)));
        assert!(bounds.contains(GenerationPoint3::new(-17, 63, 95)));
        assert!(!bounds.contains(GenerationPoint3::new(-16, 63, 95)));
        assert!(!bounds.contains(GenerationPoint3::new(-17, 64, 95)));
        assert!(!bounds.contains(GenerationPoint3::new(-17, 63, 96)));
    }

    #[test]
    fn fresh_entropy_instances_are_cache_and_history_independent() {
        let first = entropy(444, "asteria:overworld");
        let second = entropy(444, "asteria:overworld");
        let domain = GenerationDomain::named("fresh-instance-equivalence");
        let target = GenerationPoint3::new(700_000, 123, -900_000);

        for index in 0..128 {
            let _ = first.sample_3d(
                domain,
                GenerationPoint3::new(index * 17, index - 64, index * -31),
            );
        }

        assert_eq!(first.sample_3d(domain, target), second.sample_3d(domain, target));
    }
}
