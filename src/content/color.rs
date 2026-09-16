use bevy::prelude::*;
use serde::{Deserialize, Deserializer};

const HUE_TURN_DEGREES: f32 = 360.0;
const HUE_SECTOR_DEGREES: f32 = 120.0;
const HUE_SECTOR_RADIANS: f32 = std::f32::consts::TAU / 3.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsi {
    pub hue: f32,
    pub saturation: f32,
    pub intensity: f32,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SerializedColor {
    Hsi {
        hue: f32,
        saturation: f32,
        intensity: f32,
    },
    LegacyRgb {
        r: f32,
        g: f32,
        b: f32,
    },
}

impl<'de> Deserialize<'de> for Hsi {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match SerializedColor::deserialize(deserializer)? {
            SerializedColor::Hsi {
                hue,
                saturation,
                intensity,
            } => Self::new(hue, saturation, intensity),
            SerializedColor::LegacyRgb { r, g, b } => Self::from_srgb([r, g, b]),
        })
    }
}

impl Hsi {
    pub const BLACK: Self = Self {
        hue: 0.0,
        saturation: 0.0,
        intensity: 0.0,
    };
    pub const WHITE: Self = Self {
        hue: 0.0,
        saturation: 0.0,
        intensity: 1.0,
    };

    pub const fn new(hue: f32, saturation: f32, intensity: f32) -> Self {
        Self {
            hue,
            saturation,
            intensity,
        }
    }

    pub fn normalized(self) -> Self {
        Self {
            hue: self.hue.rem_euclid(HUE_TURN_DEGREES),
            saturation: self.saturation.clamp(0.0, 1.0),
            intensity: self.intensity.clamp(0.0, 1.0),
        }
    }

    pub fn from_srgb(rgb: [f32; 3]) -> Self {
        let [red, green, blue] = rgb.map(|channel| channel.clamp(0.0, 1.0));
        let intensity = (red + green + blue) / 3.0;

        if intensity <= f32::EPSILON {
            return Self::BLACK;
        }

        let minimum = red.min(green).min(blue);
        let saturation = (1.0 - minimum / intensity).clamp(0.0, 1.0);
        if saturation <= f32::EPSILON {
            return Self::new(0.0, 0.0, intensity);
        }

        let numerator = 0.5 * ((red - green) + (red - blue));
        let denominator = ((red - green) * (red - green) + (red - blue) * (green - blue))
            .max(0.0)
            .sqrt();
        let theta = if denominator <= f32::EPSILON {
            0.0
        } else {
            (numerator / denominator).clamp(-1.0, 1.0).acos()
        };
        let hue_radians = if blue > green {
            std::f32::consts::TAU - theta
        } else {
            theta
        };

        Self::new(
            hue_radians.to_degrees(),
            saturation,
            intensity.clamp(0.0, 1.0),
        )
        .normalized()
    }

    pub fn to_srgb(self) -> [f32; 3] {
        let color = self.normalized();
        if color.intensity <= f32::EPSILON {
            return [0.0; 3];
        }
        if color.saturation <= f32::EPSILON {
            return [color.intensity; 3];
        }

        let hue = color.hue.to_radians();
        let (red, green, blue) = if color.hue < HUE_SECTOR_DEGREES {
            let blue = color.intensity * (1.0 - color.saturation);
            let red = color.intensity
                * (1.0
                    + color.saturation * hue.cos()
                        / (std::f32::consts::FRAC_PI_3 - hue).cos().max(f32::EPSILON));
            let green = 3.0 * color.intensity - (red + blue);
            (red, green, blue)
        } else if color.hue < HUE_SECTOR_DEGREES * 2.0 {
            let shifted = hue - HUE_SECTOR_RADIANS;
            let red = color.intensity * (1.0 - color.saturation);
            let green = color.intensity
                * (1.0
                    + color.saturation * shifted.cos()
                        / (std::f32::consts::FRAC_PI_3 - shifted)
                            .cos()
                            .max(f32::EPSILON));
            let blue = 3.0 * color.intensity - (red + green);
            (red, green, blue)
        } else {
            let shifted = hue - HUE_SECTOR_RADIANS * 2.0;
            let green = color.intensity * (1.0 - color.saturation);
            let blue = color.intensity
                * (1.0
                    + color.saturation * shifted.cos()
                        / (std::f32::consts::FRAC_PI_3 - shifted)
                            .cos()
                            .max(f32::EPSILON));
            let red = 3.0 * color.intensity - (green + blue);
            (red, green, blue)
        };

        [red, green, blue].map(|channel| channel.clamp(0.0, 1.0))
    }

    pub fn to_color(self) -> Color {
        let [red, green, blue] = self.to_srgb();
        Color::srgb(red, green, blue)
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let start = self.normalized();
        let end = other.normalized();
        let start_hue = if start.saturation <= f32::EPSILON {
            end.hue
        } else {
            start.hue
        };
        let end_hue = if end.saturation <= f32::EPSILON {
            start_hue
        } else {
            end.hue
        };
        let hue_delta = (end_hue - start_hue + 180.0).rem_euclid(360.0) - 180.0;

        Self::new(
            start_hue + hue_delta * t,
            start.saturation + (end.saturation - start.saturation) * t,
            start.intensity + (end.intensity - start.intensity) * t,
        )
        .normalized()
    }

    pub fn blend_weighted(colors: impl IntoIterator<Item = (Self, f32)>) -> Self {
        let mut total_weight = 0.0;
        let mut saturation = 0.0;
        let mut intensity = 0.0;
        let mut hue_x = 0.0;
        let mut hue_y = 0.0;
        let mut fallback_hue = 0.0;
        let mut fallback_weight = -1.0_f32;

        for (color, weight) in colors {
            let weight = weight.max(0.0);
            if weight <= f32::EPSILON {
                continue;
            }
            let color = color.normalized();
            total_weight += weight;
            saturation += color.saturation * weight;
            intensity += color.intensity * weight;

            let chroma_weight = weight * color.saturation;
            if chroma_weight > f32::EPSILON {
                let hue = color.hue.to_radians();
                hue_x += hue.cos() * chroma_weight;
                hue_y += hue.sin() * chroma_weight;
                if chroma_weight > fallback_weight {
                    fallback_weight = chroma_weight;
                    fallback_hue = color.hue;
                }
            }
        }

        if total_weight <= f32::EPSILON {
            return Self::BLACK;
        }

        let hue = if hue_x.abs() <= f32::EPSILON && hue_y.abs() <= f32::EPSILON {
            fallback_hue
        } else {
            hue_y.atan2(hue_x).to_degrees().rem_euclid(HUE_TURN_DEGREES)
        };

        Self::new(hue, saturation / total_weight, intensity / total_weight).normalized()
    }

    pub fn is_valid(self) -> bool {
        self.hue.is_finite()
            && self.saturation.is_finite()
            && self.intensity.is_finite()
            && (0.0..HUE_TURN_DEGREES).contains(&self.hue.rem_euclid(HUE_TURN_DEGREES))
            && (0.0..=1.0).contains(&self.saturation)
            && (0.0..=1.0).contains(&self.intensity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_round_trip_preserves_color() {
        for rgb in [[1.0, 0.0, 0.0], [0.2, 0.8, 0.4], [0.7, 0.3, 0.9], [0.5; 3]] {
            let round_trip = Hsi::from_srgb(rgb).to_srgb();
            for channel in 0..3 {
                assert!((round_trip[channel] - rgb[channel]).abs() < 0.001);
            }
        }
    }

    #[test]
    fn hue_interpolation_wraps_across_zero() {
        let middle = Hsi::new(350.0, 1.0, 0.3).lerp(Hsi::new(10.0, 1.0, 0.3), 0.5);
        assert!(middle.hue < 1.0 || middle.hue > 359.0);
    }

    #[test]
    fn weighted_blend_keeps_saturation_for_opposing_hues() {
        let mixed = Hsi::blend_weighted([
            (Hsi::new(0.0, 1.0, 0.33), 0.5),
            (Hsi::new(180.0, 1.0, 0.33), 0.5),
        ]);

        assert_eq!(mixed.saturation, 1.0);
    }
}
