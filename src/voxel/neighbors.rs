use bevy::prelude::IVec3;

pub(crate) const CARDINAL_NEIGHBORS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

pub(crate) const HORIZONTAL_NEIGHBORS: [IVec3; 4] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Z,
    IVec3::NEG_Z,
];
