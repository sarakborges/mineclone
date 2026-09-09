#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VoxelLight(u8);

impl VoxelLight {
    pub const MAX_LEVEL: u8 = 15;
    pub const DARK: Self = Self(0);

    pub const fn new(sky: u8, block: u8) -> Self {
        let sky = if sky > Self::MAX_LEVEL {
            Self::MAX_LEVEL
        } else {
            sky
        };
        let block = if block > Self::MAX_LEVEL {
            Self::MAX_LEVEL
        } else {
            block
        };

        Self((sky << 4) | block)
    }

    pub const fn sky(self) -> u8 {
        self.0 >> 4
    }

    pub const fn block(self) -> u8 {
        self.0 & 0x0f
    }
}

#[cfg(test)]
mod tests {
    use super::VoxelLight;

    #[test]
    fn packs_sky_and_block_channels() {
        let light = VoxelLight::new(13, 7);

        assert_eq!(light.sky(), 13);
        assert_eq!(light.block(), 7);
    }

    #[test]
    fn clamps_channels_to_four_bits() {
        let light = VoxelLight::new(42, 31);

        assert_eq!(light.sky(), VoxelLight::MAX_LEVEL);
        assert_eq!(light.block(), VoxelLight::MAX_LEVEL);
    }
}
