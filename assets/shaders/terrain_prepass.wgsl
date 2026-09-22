#import bevy_pbr::{
    pbr_prepass_functions,
    prepass_io,
}

struct TerrainMaterialExtension {
    fluid_animation_factor: f32,
    base_tint_enabled: f32,
    overlay_enabled: f32,
    overlay_tint_enabled: f32,
    texture_array_enabled: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(101)
var<uniform> terrain_material_extension: TerrainMaterialExtension;

@group(#{MATERIAL_BIND_GROUP}) @binding(104)
var terrain_texture_array: texture_2d_array<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(105)
var terrain_texture_array_sampler: sampler;

@fragment
fn fragment(in: prepass_io::VertexOutput) -> prepass_io::FragmentOutput {
    var out: prepass_io::FragmentOutput;

    if terrain_material_extension.texture_array_enabled > 0.5 {
        let base_and_flags = u32(floor(in.uv.x / 16.0));
        let base_index = base_and_flags & 1023u;
        let texel = textureSample(
            terrain_texture_array,
            terrain_texture_array_sampler,
            fract(in.uv),
            i32(base_index),
        );

        // Shared opaque/cutout terrain uses the standard 0.5 mask. Opaque
        // textures remain fully covered; foliage holes stay absent from depth
        // and therefore cannot become false occluders.
        if texel.a < 0.5 {
            discard;
        }
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
        out.frag_depth = in.unclipped_depth;
#endif
        return out;
    }

    // Legacy block materials and attached layers still use StandardMaterial
    // base textures, so retain Bevy's native alpha-discard behavior for them.
    pbr_prepass_functions::prepass_alpha_discard(in);
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.frag_depth = in.unclipped_depth;
#endif
    return out;
}
