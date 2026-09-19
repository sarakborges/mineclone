use crate::content::color::Hsi;

const BLOCK_INTENSITY_SHIFT: u16 = 4;
const BLOCK_SATURATION_SHIFT: u16 = 8;
const BLOCK_HUE_SHIFT: u16 = 11;
const HUE_STEPS: u8 = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BlockLight {
    hue: u8,
    saturation: u8,
    intensity: u8,
}

impl BlockLight {
    pub(crate) const MAX_SATURATION: u8 = 7;
    pub(crate) const DARK: Self = Self::new(0, 0, 0);

    pub(crate) const fn new(hue: u8, saturation: u8, intensity: u8) -> Self {
        Self {
            hue: hue % HUE_STEPS,
            saturation: if saturation > Self::MAX_SATURATION {
                Self::MAX_SATURATION
            } else {
                saturation
            },
            intensity: clamp_level(intensity),
        }
    }

    pub(crate) fn from_hsi(color: Hsi, intensity: u8) -> Self {
        let color = color.normalized();
        let hue = ((color.hue / 360.0) * HUE_STEPS as f32).round() as u8 % HUE_STEPS;
        let saturation = (color.saturation * Self::MAX_SATURATION as f32)
            .round()
            .clamp(0.0, Self::MAX_SATURATION as f32) as u8;

        Self::new(hue, saturation, intensity)
    }

    pub(crate) const fn hue(self) -> u8 {
        self.hue
    }

    pub(crate) const fn saturation(self) -> u8 {
        self.saturation
    }

    pub(crate) const fn intensity(self) -> u8 {
        self.intensity
    }

    pub(crate) const fn attenuated(self, attenuation: u8) -> Self {
        Self::new(
            self.hue,
            self.saturation,
            self.intensity.saturating_sub(attenuation),
        )
    }

    pub(crate) fn color(self) -> Hsi {
        Hsi::new(
            self.hue as f32 * 360.0 / HUE_STEPS as f32,
            self.saturation as f32 / Self::MAX_SATURATION as f32,
            self.intensity as f32 / VoxelLight::MAX_LEVEL as f32,
        )
    }

    // Mesh vertex attributes are an RGB boundary. Lighting propagation and
    // mixing stay in HSI until this final conversion for the shader.
    pub(crate) fn to_srgb_levels(self) -> [u8; 3] {
        self.color().to_srgb().map(|channel| {
            (channel * VoxelLight::MAX_LEVEL as f32)
                .round()
                .clamp(0.0, VoxelLight::MAX_LEVEL as f32) as u8
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VoxelLight(u16);

impl VoxelLight {
    pub const MAX_LEVEL: u8 = 15;
    pub const DARK: Self = Self(0);

    #[cfg(test)]
    pub const fn new(sky: u8, block: u8) -> Self {
        Self::new_hsi(sky, BlockLight::new(0, 0, block))
    }

    pub(crate) const fn new_hsi(sky: u8, block: BlockLight) -> Self {
        let sky = clamp_level(sky) as u16;
        let intensity = block.intensity() as u16;
        let saturation = block.saturation() as u16;
        let hue = block.hue() as u16;

        Self(
            sky | (intensity << BLOCK_INTENSITY_SHIFT)
                | (saturation << BLOCK_SATURATION_SHIFT)
                | (hue << BLOCK_HUE_SHIFT),
        )
    }

    pub const fn sky(self) -> u8 {
        (self.0 & 0x0f) as u8
    }

    #[cfg(test)]
    pub const fn sky_levels(self) -> [u8; 3] {
        [self.sky(); 3]
    }

    pub const fn block(self) -> u8 {
        ((self.0 >> BLOCK_INTENSITY_SHIFT) & 0x0f) as u8
    }

    pub(crate) const fn block_hsi(self) -> BlockLight {
        BlockLight::new(
            ((self.0 >> BLOCK_HUE_SHIFT) & 0x1f) as u8,
            ((self.0 >> BLOCK_SATURATION_SHIFT) & 0x07) as u8,
            self.block(),
        )
    }

    pub fn block_srgb_levels(self) -> [u8; 3] {
        self.block_hsi().to_srgb_levels()
    }
}

const fn clamp_level(value: u8) -> u8 {
    if value > VoxelLight::MAX_LEVEL {
        VoxelLight::MAX_LEVEL
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockLight, HUE_STEPS, VoxelLight};
    use crate::content::color::Hsi;

    #[test]
    fn packs_white_block_light_as_zero_saturation() {
        let light = VoxelLight::new(13, 7);

        assert_eq!(light.sky(), 13);
        assert_eq!(light.sky_levels(), [13, 13, 13]);
        assert_eq!(light.block(), 7);
        assert_eq!(light.block_hsi().saturation(), 0);
        assert_eq!(light.block_srgb_levels(), [7, 7, 7]);
    }

    #[test]
    fn colored_block_light_preserves_hsi_channels() {
        let block = BlockLight::from_hsi(Hsi::new(210.0, 0.8, 0.4), 14);
        let light = VoxelLight::new_hsi(15, block);

        assert_eq!(light.sky(), 15);
        assert_eq!(light.block(), 14);
        assert_eq!(light.block_hsi(), block);
    }

    #[test]
    fn block_light_uses_five_hue_three_saturation_and_four_intensity_bits() {
        let block = BlockLight::new(31, 7, 15);
        let light = VoxelLight::new_hsi(15, block);

        assert_eq!(light.block_hsi(), block);
        assert_eq!(light.block(), 15);
    }

    #[test]
    fn hsi_input_is_clamped_before_quantization() {
        let block = BlockLight::from_hsi(Hsi::new(725.0, 3.0, 2.0), 99);

        assert_eq!(block.intensity(), 15);
        assert_eq!(block.saturation(), BlockLight::MAX_SATURATION);
        assert!(block.hue() < HUE_STEPS);
    }
}
