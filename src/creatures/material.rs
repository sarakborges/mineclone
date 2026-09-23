use bevy::prelude::*;

use crate::content::color::Hsi;

pub(crate) fn apply_creature_material_overrides(
    material: &mut StandardMaterial,
    tint: Option<&Hsi>,
    texture: Option<&Handle<Image>>,
    unlit: bool,
) {
    if let Some(tint) = tint {
        let rgb = tint.to_srgb();
        let alpha = material.base_color.to_srgba().alpha;
        material.base_color = Color::srgba(rgb[0], rgb[1], rgb[2], alpha);
    }
    if let Some(texture) = texture {
        material.base_color_texture = Some(texture.clone());
    }

    // Creature override materials are intentionally fully matte. Keep normal
    // light/shadow response unless the definition explicitly requests unlit.
    material.metallic = 0.0;
    material.perceptual_roughness = 1.0;
    material.reflectance = 0.0;
    material.specular_tint = Color::BLACK;
    material.clearcoat = 0.0;
    material.unlit = unlit;
    material.diffuse_transmission = 0.0;
    material.specular_transmission = 0.0;
    material.thickness = 0.0;
    material.emissive = LinearRgba::BLACK;
    material.emissive_texture = None;

    // Tinted body materials are deliberately opaque. Texture-only materials
    // keep the GLB-authored alpha mode so decals/cutouts can stay transparent.
    if tint.is_some() {
        material.base_color = material.base_color.with_alpha(1.0);
        material.alpha_mode = AlphaMode::Opaque;
    }
}
