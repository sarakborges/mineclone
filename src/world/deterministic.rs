pub(crate) fn mix_u32_components(mut hash: u64, components: impl IntoIterator<Item = u32>) -> u64 {
    for component in components {
        hash ^= component as u64;
        hash = hash.wrapping_mul(0x9e37_79b1_85eb_ca87);
        hash ^= hash >> 31;
    }

    hash
}

pub(crate) fn hash_string(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn hash_unit(hash: u64) -> f32 {
    (hash & 0xffff) as f32 / u16::MAX as f32
}

pub(crate) fn hash_signed(hash: u64) -> f32 {
    hash_unit(hash) * 2.0 - 1.0
}
