pub(crate) mod connectors;
pub(crate) mod geometry;
pub(crate) mod hash;
pub(crate) mod placement;
mod resolver;
pub(crate) mod set;

pub(crate) use connectors::connected_horizontal_bounds_for_reference;
pub(crate) use placement::candidate_anchor as structure_candidate_anchor;
pub(crate) use resolver::{StructureCandidate, resolve_structure_placements};
