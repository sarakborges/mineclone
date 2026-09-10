use bevy::prelude::*;

const CLIMATE_OCTAVES: usize = 4;

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
                position * 0.0008,
                self.seed ^ 0xa409_3822_299f_31d0,
            ),
            erosion: normalized_fractal_noise(position * 0.0040, self.seed ^ 0x082e_fa98_ec4e_6c89),
        }
    }
}

fn normalized_fractal_noise(position: Vec2, seed: u64) -> f32 {
    ((fractal_noise(position, seed) + 1.0) * 0.5).clamp(0.0, 1.0)
}

fn fractal_noise(position: Vec2, seed: u64) -> f32 {
    let mut value = 0.0;
    let mut normalization = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;

    for octave in 0..CLIMATE_OCTAVES {
        let octave_seed = seed.wrapping_add((octave as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        value += value_noise(position * frequency, octave_seed) * amplitude;
        normalization += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value / normalization
}

fn value_noise(position: Vec2, seed: u64) -> f32 {
    let x0 = position.x.floor() as i32;
    let z0 = position.y.floor() as i32;
    let x1 = x0 + 1;
    let z1 = z0 + 1;
    let tx = smoothstep(position.x - x0 as f32);
    let tz = smoothstep(position.y - z0 as f32);
    let top = lerp(lattice_noise(x0, z0, seed), lattice_noise(x1, z0, seed), tx);
    let bottom = lerp(lattice_noise(x0, z1, seed), lattice_noise(x1, z1, seed), tx);

    lerp(top, bottom, tz)
}

fn lattice_noise(x: i32, z: i32, seed: u64) -> f32 {
    let mut hash = seed;
    hash ^= (x as i64 as u64).wrapping_mul(0x9e37_79b1_85eb_ca87);
    hash ^= (z as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    let normalized = (hash & 0xffff) as f32 / u16::MAX as f32;

    normalized * 2.0 - 1.0
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
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
