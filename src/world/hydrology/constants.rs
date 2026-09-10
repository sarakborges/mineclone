pub(super) const HYDROLOGY_REGION_SIZE: f32 = 128.0;
pub(super) const MACRO_SAMPLE_GRID: usize = 5;

pub(super) const OCEAN_CONTINENTALNESS_THRESHOLD: f32 = 0.47;
pub(super) const OCEAN_TRANSITION_WIDTH: f32 = 0.14;
pub(super) const OCEAN_MINIMUM_DEPTH: f32 = 8.0;
pub(super) const OCEAN_EXTRA_DEPTH: f32 = 18.0;

pub(super) const RIVER_EDGE_MARGIN_CELLS: i32 = 2;
pub(super) const RIVER_MINIMUM_DROP: f32 = 0.01;
pub(super) const RIVER_MINIMUM_WATER_DROP: f32 = 0.5;
pub(super) const RIVER_BASIN_ESCAPE_RADIUS_CELLS: i32 = 3;
pub(super) const RIVER_ROUTE_VARIATION: f32 = 1.15;
pub(super) const RIVER_FLOW_SEARCH_RADIUS: i32 = 6;
pub(super) const RIVER_FLOW_TRACE_STEPS: usize = 64;
pub(super) const RIVER_MINIMUM_FLOW: u32 = 4;
pub(super) const RIVER_FLOW_FOR_MAX_WIDTH: f32 = 20.0;
pub(super) const RIVER_MINIMUM_RADIUS: f32 = 5.5;
pub(super) const RIVER_MAXIMUM_RADIUS: f32 = 11.0;
pub(super) const RIVER_CARVE_DEPTH: f32 = 7.0;
pub(super) const RIVER_CARVE_STRENGTH: f32 = 16.0;

pub(super) const LAKE_MINIMUM_RADIUS: f32 = 22.0;
pub(super) const LAKE_MAXIMUM_RADIUS: f32 = 46.0;
pub(super) const LAKE_CARVE_DEPTH: f32 = 14.0;
pub(super) const LAKE_MINIMUM_RELIEF: f32 = 0.25;
pub(super) const LAKE_CHANCE: f32 = 0.38;
pub(super) const LAKE_SHORE_INNER_DISTANCE: f32 = 0.72;
pub(super) const LAKE_SHORE_OUTER_DISTANCE: f32 = 1.18;
pub(super) const LAKE_SHORE_SURFACE_OFFSET: f32 = 0.25;

pub(super) const SHORE_STRENGTH: f32 = 0.25;
pub(super) const BED_MATERIAL_DEPTH: f32 = 1.5;
