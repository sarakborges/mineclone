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
            intensity: clamp_channel(intensity),
        }
    }

    #[cfg(test)]
    pub(crate) const fn from_rgb_levels(rgb: [u8; 3]) -> Self {
        let red = clamp_channel(rgb[0]);
        let green = clamp_channel(rgb[1]);
        let blue = clamp_channel(rgb[2]);
        let maximum = max_channel([red, green, blue]);

        if maximum == 0 {
            return Self::DARK;
        }

        let minimum = min_channel([red, green, blue]);
        let delta = maximum - minimum;
        if delta == 0 {
            return Self::new(0, 0, maximum);
        }

        let saturation = ((delta as u16 * Self::MAX_SATURATION as u16
            + maximum as u16 / 2)
            / maximum as u16) as u8;
        let hue = quantized_hue(red, green, blue, maximum, delta);
        Self::new(hue, saturation, maximum)
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

    pub(crate) fn to_rgb_levels(self) -> [u8; 3] {
        if self.intensity == 0 {
            return [0; 3];
        }
        if self.saturation == 0 {
            return [self.intensity; 3];
        }

        let hue_sector = self.hue as f32 * 6.0 / HUE_STEPS as f32;
        let sector = hue_sector as u32;
        let fraction = hue_sector - sector as f32;
        let saturation = self.saturation as f32 / Self::MAX_SATURATION as f32;
        let value = self.intensity as f32;
        let low = value * (1.0 - saturation);
        let falling = value * (1.0 - saturation * fraction);
        let rising = value * (1.0 - saturation * (1.0 - fraction));

        let rgb = match sector % 6 {
            0 => [value, rising, low],
            1 => [falling, value, low],
            2 => [low, value, rising],
            3 => [low, falling, value],
            4 => [rising, low, value],
            _ => [value, low, falling],
        };

        rgb.map(|channel| {
            channel
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

    #[cfg(test)]
    pub const fn new_colored(sky: [u8; 3], block: [u8; 3]) -> Self {
        Self::new_hsi(max_channel(sky), BlockLight::from_rgb_levels(block))
    }

    pub(crate) const fn new_hsi(sky: u8, block: BlockLight) -> Self {
        let sky = clamp_channel(sky) as u16;
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
    pub const fn sky_rgb(self) -> [u8; 3] {
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

    pub fn block_rgb(self) -> [u8; 3] {
        self.block_hsi().to_rgb_levels()
    }
}

const fn clamp_channel(value: u8) -> u8 {
    if value > VoxelLight::MAX_LEVEL {
        VoxelLight::MAX_LEVEL
    } else {
        value
    }
}

#[cfg(test)]
const fn max_channel(value: [u8; 3]) -> u8 {
    let first = if value[0] > value[1] {
        value[0]
    } else {
        value[1]
    };
    if first > value[2] { first } else { value[2] }
}

#[cfg(test)]
const fn min_channel(value: [u8; 3]) -> u8 {
    let first = if value[0] < value[1] {
        value[0]
    } else {
        value[1]
    };
    if first < value[2] { first } else { value[2] }
}

#[cfg(test)]
const fn quantized_hue(red: u8, green: u8, blue: u8, maximum: u8, delta: u8) -> u8 {
    let delta = delta as i32;
    let mut sector_numerator = if maximum == red {
        green as i32 - blue as i32
    } else if maximum == green {
        blue as i32 - red as i32 + 2 * delta
    } else {
        red as i32 - green as i32 + 4 * delta
    };
    let full_turn = 6 * delta;

    if sector_numerator < 0 {
        sector_numerator += full_turn;
    }

    let scaled = sector_numerator * HUE_STEPS as i32;
    let quantized = (scaled + full_turn / 2) / full_turn;
    (quantized as u8) % HUE_STEPS
}

#[cfg(test)]
mod tests {
    use super::{BlockLight, VoxelLight};

    #[test]
    fn packs_white_block_light_as_zero_saturation() {
        let light = VoxelLight::new(13, 7);

        assert_eq!(light.sky(), 13);
        assert_eq!(light.sky_rgb(), [13, 13, 13]);
        assert_eq!(light.block(), 7);
        assert_eq!(light.block_hsi().saturation(), 0);
        assert_eq!(light.block_rgb(), [7, 7, 7]);
    }

    #[test]
    fn colored_block_light_round_trips_through_hsi_quantization() {
        let light = VoxelLight::new_colored([15, 6, 1], [2, 9, 14]);

        assert_eq!(light.sky(), 15);
        assert_eq!(light.block(), 14);
        assert_eq!(light.block_rgb(), [2, 10, 14]);
    }

    #[test]
    fn block_light_uses_five_hue_three_saturation_and_four_intensity_bits() {
        let block = BlockLight::new(31, 7, 15);
        let light = VoxelLight::new_hsi(15, block);

        assert_eq!(light.block_hsi(), block);
        assert_eq!(light.block(), 15);
    }

    #[test]
    fn clamps_rgb_inputs_before_hsi_encoding() {
        let light = VoxelLight::new_colored([42, 1, 31], [2, 99, 7]);

        assert_eq!(light.sky(), 15);
        assert_eq!(light.block(), 15);
        assert_eq!(light.block_rgb(), [2, 15, 8]);
    }
}
