use bevy::prelude::*;
use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    pub fn to_color(self) -> Color {
        Color::srgb(self.r, self.g, self.b)
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
        }
    }
}
