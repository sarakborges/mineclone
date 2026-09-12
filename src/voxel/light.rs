#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VoxelLight(u16);

impl VoxelLight {
    pub const MAX_LEVEL: u8 = 15;
    pub const DARK: Self = Self(0);

    pub const fn new(sky: u8, block: u8) -> Self {
        Self::new_colored([sky, sky, sky], [block, block, block])
    }

    pub const fn new_colored(sky: [u8; 3], block: [u8; 3]) -> Self {
        let sky = clamp_channel(max_channel(sky)) as u16;
        let block = clamp_channel(max_channel(block)) as u16;

        // Keep the wider storage/API for compatibility with newer systems, but
        // preserve the September 9 lighting contract: both propagated channels
        // are scalar. Block light is mirrored into RGB rather than tinted.
        Self(sky | (block << 4) | (block << 8) | (block << 12))
    }

    pub const fn sky(self) -> u8 {
        (self.0 & 0x0f) as u8
    }

    pub const fn sky_rgb(self) -> [u8; 3] {
        [self.sky(); 3]
    }

    pub const fn block(self) -> u8 {
        max_channel(self.block_rgb())
    }

    pub const fn block_rgb(self) -> [u8; 3] {
        [
            ((self.0 >> 4) & 0x0f) as u8,
            ((self.0 >> 8) & 0x0f) as u8,
            ((self.0 >> 12) & 0x0f) as u8,
        ]
    }
}

const fn clamp_channel(value: u8) -> u8 {
    if value > VoxelLight::MAX_LEVEL {
        VoxelLight::MAX_LEVEL
    } else {
        value
    }
}

const fn max_channel(value: [u8; 3]) -> u8 {
    let first = if value[0] > value[1] {
        value[0]
    } else {
        value[1]
    };
    if first > value[2] { first } else { value[2] }
}

#[cfg(test)]
mod tests {
    use super::VoxelLight;

    #[test]
    fn packs_sky_and_block_channels() {
        let light = VoxelLight::new(13, 7);

        assert_eq!(light.sky(), 13);
        assert_eq!(light.sky_rgb(), [13, 13, 13]);
        assert_eq!(light.block_rgb(), [7, 7, 7]);
    }

    #[test]
    fn collapses_colored_inputs_to_scalar_light() {
        let light = VoxelLight::new_colored([15, 6, 1], [2, 9, 14]);

        assert_eq!(light.sky(), 15);
        assert_eq!(light.sky_rgb(), [15, 15, 15]);
        assert_eq!(light.block_rgb(), [14, 14, 14]);
        assert_eq!(light.block(), 14);
    }

    #[test]
    fn clamps_channels_to_four_bits() {
        let light = VoxelLight::new_colored([42, 1, 31], [2, 99, 7]);

        assert_eq!(light.sky(), 15);
        assert_eq!(light.block_rgb(), [15, 15, 15]);
    }
}
