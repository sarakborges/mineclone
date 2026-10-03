use serde::Deserialize;

use super::color::Hsi;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmbientParticleRange {
    pub min: f32,
    pub max: f32,
}

impl AmbientParticleRange {
    fn validate(self, owner: &str, field: &str, strictly_positive: bool) {
        assert!(
            self.min.is_finite() && self.max.is_finite(),
            "{owner} ambient particle {field} must be finite"
        );
        if strictly_positive {
            assert!(
                self.min > 0.0,
                "{owner} ambient particle {field}.min must be positive"
            );
        } else {
            assert!(
                self.min >= 0.0,
                "{owner} ambient particle {field}.min must be non-negative"
            );
        }
        assert!(
            self.max >= self.min,
            "{owner} ambient particle {field}.max must be greater than or equal to min"
        );
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmbientParticleDefinition {
    pub color: Hsi,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default = "default_size")]
    pub size: AmbientParticleRange,
    #[serde(default = "default_lifetime")]
    pub lifetime: AmbientParticleRange,
    pub spawn_rate: f32,
    #[serde(default)]
    pub velocity: [f32; 3],
    #[serde(default)]
    pub velocity_jitter: [f32; 3],
    #[serde(default)]
    pub acceleration: [f32; 3],
    #[serde(default)]
    pub wander_strength: f32,
    #[serde(default)]
    pub wind_influence: f32,
    #[serde(default)]
    pub pop_at_end: bool,
    #[serde(default = "default_spawn_radius")]
    pub spawn_radius: f32,
    #[serde(default = "default_vertical_range")]
    pub vertical_range: f32,
}

impl AmbientParticleDefinition {
    pub(crate) fn validate(&self, owner: &str) {
        assert!(
            self.color.is_valid(),
            "{owner} ambient particle color is invalid"
        );
        assert!(
            self.opacity.is_finite() && (0.0..=1.0).contains(&self.opacity),
            "{owner} ambient particle opacity must be between 0 and 1"
        );
        self.size.validate(owner, "size", true);
        self.lifetime.validate(owner, "lifetime", true);
        assert!(
            self.spawn_rate.is_finite() && self.spawn_rate >= 0.0,
            "{owner} ambient particle spawnRate must be finite and non-negative"
        );
        validate_vec3(owner, "velocity", self.velocity, false);
        validate_vec3(owner, "velocityJitter", self.velocity_jitter, true);
        validate_vec3(owner, "acceleration", self.acceleration, false);
        assert!(
            self.wander_strength.is_finite() && self.wander_strength >= 0.0,
            "{owner} ambient particle wanderStrength must be finite and non-negative"
        );
        assert!(
            self.wind_influence.is_finite() && self.wind_influence >= 0.0,
            "{owner} ambient particle windInfluence must be finite and non-negative"
        );
        assert!(
            self.spawn_radius.is_finite() && self.spawn_radius > 0.0,
            "{owner} ambient particle spawnRadius must be finite and positive"
        );
        assert!(
            self.vertical_range.is_finite() && self.vertical_range >= 0.0,
            "{owner} ambient particle verticalRange must be finite and non-negative"
        );
    }
}

fn validate_vec3(owner: &str, field: &str, value: [f32; 3], non_negative: bool) {
    assert!(
        value.into_iter().all(f32::is_finite),
        "{owner} ambient particle {field} must contain only finite values"
    );
    if non_negative {
        assert!(
            value.into_iter().all(|component| component >= 0.0),
            "{owner} ambient particle {field} must contain only non-negative values"
        );
    }
}

fn default_opacity() -> f32 {
    1.0
}

fn default_size() -> AmbientParticleRange {
    AmbientParticleRange {
        min: 0.05,
        max: 0.1,
    }
}

fn default_lifetime() -> AmbientParticleRange {
    AmbientParticleRange { min: 2.0, max: 4.0 }
}

fn default_spawn_radius() -> f32 {
    16.0
}

fn default_vertical_range() -> f32 {
    8.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_definition() -> AmbientParticleDefinition {
        AmbientParticleDefinition {
            color: Hsi::WHITE,
            opacity: 1.0,
            size: AmbientParticleRange { min: 0.1, max: 0.2 },
            lifetime: AmbientParticleRange { min: 1.0, max: 2.0 },
            spawn_rate: 3.0,
            velocity: [0.0, 0.1, 0.0],
            velocity_jitter: [0.1, 0.1, 0.1],
            acceleration: [0.0, -0.2, 0.0],
            wander_strength: 0.1,
            wind_influence: 0.0,
            pop_at_end: false,
            spawn_radius: 12.0,
            vertical_range: 6.0,
        }
    }

    #[test]
    fn valid_definition_passes_validation() {
        valid_definition().validate("test");
    }

    #[test]
    #[should_panic(expected = "spawnRadius")]
    fn zero_spawn_radius_is_rejected() {
        let mut definition = valid_definition();
        definition.spawn_radius = 0.0;
        definition.validate("test");
    }
}
