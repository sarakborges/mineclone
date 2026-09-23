pub(super) fn next_u32(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

pub(super) fn next_unit_f32(state: &mut u32) -> f32 {
    next_u32(state) as f32 / u32::MAX as f32
}

pub(super) fn next_signed_f32(state: &mut u32) -> f32 {
    next_unit_f32(state) * 2.0 - 1.0
}
