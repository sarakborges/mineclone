pub(super) const HYDROLOGY_REGION_SIZE: f32 = 128.0;
pub(super) const RIVER_EDGE_MARGIN_CELLS: i32 = 4;
pub(super) const RIVER_MINIMUM_DROP: f32 = 0.01;
pub(super) const RIVER_BASIN_ESCAPE_RADIUS_CELLS: i32 = 4;
pub(super) const RIVER_OCEAN_OUTLET_RADIUS_CELLS: i32 = 4;
pub(super) const RIVER_ROUTE_VARIATION: f32 = 2.4;
pub(super) const RIVER_FLOW_SEARCH_RADIUS: i32 = 6;
pub(super) const RIVER_FLOW_TRACE_STEPS: usize = 64;
pub(super) const RIVER_MINIMUM_FLOW: u32 = 2;
pub(super) const RIVER_FLOW_FOR_MAX_WIDTH: f32 = 20.0;
pub(super) const RIVER_MINIMUM_RADIUS: f32 = 5.5;
pub(super) const RIVER_MAXIMUM_RADIUS: f32 = 11.0;
// Graph distance after which river shore grading has fully faded.
pub(super) const RIVER_BANK_OUTER_NORMALIZED_DISTANCE: f32 = 1.30;
pub(super) const RIVER_CARVE_DEPTH: f32 = 7.0;
pub(super) const RIVER_CARVE_STRENGTH: f32 = 16.0;
pub(super) const RIVER_WATER_BODY_APPROACH_MARGIN: f32 = 20.0;

pub(super) const LAKE_MINIMUM_RADIUS: f32 = 48.0;
pub(super) const LAKE_MAXIMUM_RADIUS: f32 = 112.0;
pub(super) const LAKE_CARVE_DEPTH: f32 = 18.0;
pub(super) const LAKE_MINIMUM_RELIEF: f32 = 0.25;
pub(super) const LAKE_CHANCE: f32 = 0.62;
pub(super) const LAKE_SHORE_OUTER_DISTANCE: f32 = 1.18;
pub(super) const LAKE_SHORE_SURFACE_OFFSET: f32 = 0.9;

pub(super) const SHORE_STRENGTH: f32 = 0.25;
pub(super) const BED_MATERIAL_DEPTH: f32 = 1.5;
