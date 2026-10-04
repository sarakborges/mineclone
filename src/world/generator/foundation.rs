use bevy::prelude::{IVec2, IVec3, UVec2};

use crate::{
    content::dimension::DimensionDefinition,
    world::{
        WorldSeed,
        deterministic::{hash_string, mix_seed, mix_u32_components},
        dimension::DimensionId,
    },
};

const DEFAULT_AREA_BATCH_WIDTH: u32 = 32;

/// Immutable dimension-level inputs that are already part of the new generation
/// contract. Legacy biome-placement fields intentionally do not cross this
/// boundary; later phases add new authored generation definitions explicitly.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GenerationDimension {
    id: DimensionId,
    sea_level: i32,
    gravity_strength: f32,
}

impl GenerationDimension {
    pub(crate) fn new(id: DimensionId, sea_level: i32, gravity_strength: f32) -> Self {
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
            DimensionId::from(definition.id.as_str()),
            definition.sea_level,
            definition.gravity_strength,
        )
    }

    pub(crate) fn id(&self) -> &DimensionId {
        &self.id
    }

    pub(crate) fn sea_level(&self) -> i32 {
        self.sea_level
    }

    pub(crate) fn gravity_strength(&self) -> f32 {
        self.gravity_strength
    }
}

/// Frozen generated-world inputs. Runtime systems may replace a generator with a
/// new snapshot, but a live generator never observes registry mutation through
/// shared mutable state.
#[derive(Clone, Debug)]
pub(crate) struct GenerationSnapshot {
    seed: WorldSeed,
    dimension: GenerationDimension,
}

impl GenerationSnapshot {
    pub(crate) fn new(seed: WorldSeed, dimension: GenerationDimension) -> Self {
        Self { seed, dimension }
    }

    pub(crate) fn seed(&self) -> WorldSeed {
        self.seed
    }

    pub(crate) fn dimension(&self) -> &GenerationDimension {
        &self.dimension
    }
}

/// Stable semantic salt for one generation algorithm/domain.
///
/// Domain names are part of generated-world semantics. Internal cache/batch
/// dimensions are deliberately excluded so changing performance strategy cannot
/// change generated output.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct GenerationDomain(u64);

impl GenerationDomain {
    pub(crate) fn named(name: &str) -> Self {
        assert!(!name.trim().is_empty(), "generation domain cannot be empty");
        Self(mix_seed(hash_string(name)))
    }
}

/// Rectangular X/Z world-space request. `size.x` is width in X and `size.y` is
/// depth in Z. The rectangle is inclusive at origin and exclusive at
/// `origin + size`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SampleArea2d {
    origin: IVec2,
    size: UVec2,
}

impl SampleArea2d {
    pub(crate) fn new(origin: IVec2, size: UVec2) -> Self {
        assert!(size.x > 0 && size.y > 0, "sample area must be non-empty");
        let last_x = origin.x as i64 + size.x as i64 - 1;
        let last_z = origin.y as i64 + size.y as i64 - 1;
        assert!(
            last_x <= i32::MAX as i64 && last_z <= i32::MAX as i64,
            "sample area exceeds world coordinate range"
        );
        let sample_count = (size.x as u64)
            .checked_mul(size.y as u64)
            .and_then(|count| usize::try_from(count).ok());
        assert!(sample_count.is_some(), "sample area is too large to address");
        Self { origin, size }
    }

    pub(crate) fn origin(self) -> IVec2 {
        self.origin
    }

    pub(crate) fn size(self) -> UVec2 {
        self.size
    }

    fn sample_count(self) -> usize {
        usize::try_from(self.size.x as u64 * self.size.y as u64)
            .expect("validated sample area count must fit usize")
    }

    fn position_at(self, x_offset: u32, z_offset: u32) -> IVec2 {
        debug_assert!(x_offset < self.size.x && z_offset < self.size.y);
        IVec2::new(
            i32::try_from(self.origin.x as i64 + x_offset as i64)
                .expect("validated X sample must fit i32"),
            i32::try_from(self.origin.y as i64 + z_offset as i64)
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

    pub(crate) fn area(&self) -> SampleArea2d {
        self.area
    }

    pub(crate) fn get(&self, position: IVec2) -> Option<u64> {
        let x = position.x as i64 - self.area.origin.x as i64;
        let z = position.y as i64 - self.area.origin.y as i64;
        if x < 0 || z < 0 || x >= self.area.size.x as i64 || z >= self.area.size.y as i64 {
            return None;
        }
        let index = z as usize * self.area.size.x as usize + x as usize;
        self.values.get(index).copied()
    }

    #[cfg(test)]
    fn values(&self) -> &[u64] {
        &self.values
    }
}

/// Stateless deterministic entropy rooted in world seed + dimension identity.
/// It is intentionally below biome/terrain/structure ownership and is not a
/// gameplay-facing query API.
#[derive(Clone, Copy, Debug)]
pub(super) struct GenerationEntropy {
    root: u64,
}

impl GenerationEntropy {
    pub(super) fn new(snapshot: &GenerationSnapshot) -> Self {
        let dimension = mix_seed(hash_string(snapshot.dimension.id().as_str()));
        Self {
            root: mix_seed(snapshot.seed.0 ^ dimension),
        }
    }

    pub(super) fn sample_2d(self, domain: GenerationDomain, position: IVec2) -> u64 {
        let domain_seed = mix_seed(self.root ^ domain.0);
        mix_seed(mix_u32_components(
            domain_seed,
            [position.x as u32, position.y as u32],
        ))
    }

    pub(super) fn sample_3d(self, domain: GenerationDomain, position: IVec3) -> u64 {
        let domain_seed = mix_seed(self.root ^ domain.0);
        mix_seed(mix_u32_components(
            domain_seed,
            [position.x as u32, position.y as u32, position.z as u32],
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
        let width = area.size.x;

        for z_offset in 0..area.size.y {
            let mut batch_start = 0;
            while batch_start < width {
                let batch_end = batch_start.saturating_add(batch_width).min(width);
                for x_offset in batch_start..batch_end {
                    let index = z_offset as usize * width as usize + x_offset as usize;
                    values[index] = self.sample_2d(domain, area.position_at(x_offset, z_offset));
                }
                batch_start = batch_end;
            }
        }

        SampleGrid2d::new(area, values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entropy(seed: u64, dimension: &str) -> GenerationEntropy {
        GenerationEntropy::new(&GenerationSnapshot::new(
            WorldSeed(seed),
            GenerationDimension::new(DimensionId::from(dimension), 64, 1.0),
        ))
    }

    #[test]
    fn direct_far_coordinate_query_is_order_independent() {
        let entropy = entropy(42, "asteria:overworld");
        let domain = GenerationDomain::named("direct-coordinate");
        let far = IVec2::new(1_500_000_000, -1_500_000_000);
        let direct = entropy.sample_2d(domain, far);

        for point in [
            IVec2::ZERO,
            IVec2::new(16, 16),
            IVec2::new(-4096, 8192),
            IVec2::new(999_999, -777_777),
        ] {
            let _ = entropy.sample_2d(domain, point);
        }

        assert_eq!(entropy.sample_2d(domain, far), direct);
    }

    #[test]
    fn scalar_and_area_sampling_are_equivalent() {
        let entropy = entropy(77, "asteria:overworld");
        let domain = GenerationDomain::named("scalar-area-equivalence");
        let area = SampleArea2d::new(IVec2::new(-37, 91), UVec2::new(13, 9));
        let grid = entropy.sample_area_2d(domain, area);

        for z_offset in 0..area.size().y {
            for x_offset in 0..area.size().x {
                let position = area.position_at(x_offset, z_offset);
                assert_eq!(grid.get(position), Some(entropy.sample_2d(domain, position)));
            }
        }
    }

    #[test]
    fn internal_batch_width_cannot_change_results() {
        let entropy = entropy(1234, "asteria:umbral");
        let domain = GenerationDomain::named("batch-equivalence");
        let area = SampleArea2d::new(IVec2::new(-128, -64), UVec2::new(67, 19));

        let scalar_batches = entropy.sample_area_2d_with_batch_width(domain, area, 1);
        let uneven_batches = entropy.sample_area_2d_with_batch_width(domain, area, 7);
        let wide_batches = entropy.sample_area_2d_with_batch_width(domain, area, 128);

        assert_eq!(scalar_batches.values(), uneven_batches.values());
        assert_eq!(scalar_batches.values(), wide_batches.values());
    }

    #[test]
    fn dimensions_have_independent_entropy_domains() {
        let domain = GenerationDomain::named("dimension-separation");
        let point = IVec3::new(200, 70, -400);

        assert_ne!(
            entropy(99, "asteria:overworld").sample_3d(domain, point),
            entropy(99, "asteria:umbral").sample_3d(domain, point)
        );
    }

    #[test]
    fn area_grid_rejects_points_outside_requested_bounds() {
        let entropy = entropy(5, "asteria:overworld");
        let area = SampleArea2d::new(IVec2::new(10, 20), UVec2::new(2, 3));
        let grid = entropy.sample_area_2d(GenerationDomain::named("bounds"), area);

        assert_eq!(grid.area(), area);
        assert!(grid.get(IVec2::new(9, 20)).is_none());
        assert!(grid.get(IVec2::new(12, 20)).is_none());
        assert!(grid.get(IVec2::new(10, 23)).is_none());
    }
}
