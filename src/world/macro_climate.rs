use bevy::prelude::*;

use crate::world::noise::fractal_noise_2d;

const CLIMATE_OCTAVES: usize = 4;
const CONTINENTALNESS_SCALE: f32 = 0.0012;

#[derive(Clone, Copy, Debug)]
pub struct MacroClimateSample {
    pub temperature: f32,
    pub humidity: f32,
    pub continentalness: f32,
    pub erosion: f32,
}

pub struct MacroClimateField {
    seed: u64,
}

impl MacroClimateField {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub fn sample(&self, position: Vec2) -> MacroClimateSample {
        MacroClimateSample {
            temperature: normalized_fractal_noise(
                position * 0.0018,
                self.seed ^ 0x243f_6a88_85a3_08d3,
            ),
            humidity: normalized_fractal_noise(
                position * 0.0024,
                self.seed ^ 0x1319_8a2e_0370_7344,
            ),
            continentalness: normalized_fractal_noise(
                position * CONTINENTALNESS_SCALE,
                self.seed ^ 0xa409_3822_299f_31d0,
            ),
            erosion: normalized_fractal_noise(position * 0.0040, self.seed ^ 0x082e_fa98_ec4e_6c89),
        }
    }
}

fn normalized_fractal_noise(position: Vec2, seed: u64) -> f32 {
    ((fractal_noise_2d(position, seed, CLIMATE_OCTAVES) + 1.0) * 0.5).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn climate_is_deterministic_and_normalized() {
        let field = MacroClimateField::new(42);
        let first = field.sample(Vec2::new(1234.5, -789.25));
        let second = field.sample(Vec2::new(1234.5, -789.25));

        assert_eq!(first.temperature, second.temperature);
        assert_eq!(first.humidity, second.humidity);
        assert_eq!(first.continentalness, second.continentalness);
        assert_eq!(first.erosion, second.erosion);

        for value in [
            first.temperature,
            first.humidity,
            first.continentalness,
            first.erosion,
        ] {
            assert!((0.0..=1.0).contains(&value));
        }
    }
}
