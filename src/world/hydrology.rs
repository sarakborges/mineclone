mod constants;
mod drainage;
mod field;
mod lake;
mod math;
mod region;
mod river;
mod spatial;
mod types;

#[cfg(test)]
mod tests;

pub(crate) use field::HydrologyField;
pub(crate) use region::HydrologyRegion;
pub(crate) use types::{HydrologyBiomeOverlay, HydrologySurfaceSample};
pub(crate) use types::{
    HydrologyRiverSurfaceSample, HydrologyWaterKind, HydrologyWaterSample,
};

#[cfg(test)]
use types::WaterBody;
