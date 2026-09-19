pub(super) fn hash01(mut value: u32) -> f32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^= value >> 16;
    value as f32 / u32::MAX as f32
}

pub(super) fn hash_signed(value: u32) -> f32 {
    hash01(value) * 2.0 - 1.0
}
