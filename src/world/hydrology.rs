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

pub use field::HydrologyField;
pub use region::HydrologyRegion;
pub use types::{HydrologyBiomeOverlay, HydrologySurfaceSample};
pub(crate) use types::{
    HydrologyRiverSurfaceSample, HydrologyWaterKind, HydrologyWaterSample,
};

#[cfg(test)]
use types::WaterBody;
