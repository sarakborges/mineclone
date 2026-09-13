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
        let red = clamp_channel(block[0]) as u16;
        let green = clamp_channel(block[1]) as u16;
        let blue = clamp_channel(block[2]) as u16;

        Self(sky | (red << 4) | (green << 8) | (blue << 12))
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
    fn preserves_colored_block_light_channels() {
        let light = VoxelLight::new_colored([15, 6, 1], [2, 9, 14]);

        assert_eq!(light.sky(), 15);
        assert_eq!(light.sky_rgb(), [15, 15, 15]);
        assert_eq!(light.block_rgb(), [2, 9, 14]);
        assert_eq!(light.block(), 14);
    }

    #[test]
    fn clamps_channels_to_four_bits() {
        let light = VoxelLight::new_colored([42, 1, 31], [2, 99, 7]);

        assert_eq!(light.sky(), 15);
        assert_eq!(light.block_rgb(), [2, 15, 7]);
    }
}
