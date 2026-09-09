#import bevy_pbr::{
    pbr_bindings,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::main_pass_post_lighting_processing,
}
#endif

struct TerrainMaterialExtension {
    sky_light_factor: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> terrain_material_extension: TerrainMaterialExtension;

const AMBIENT_FLOOR: f32 = 0.055;
const LIGHT_GAMMA: f32 = 1.35;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let texel = textureSample(
        pbr_bindings::base_color_texture,
        pbr_bindings::base_color_sampler,
        in.uv,
    );
    let tint = clamp(in.color.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let ambient_occlusion = clamp(in.color.a, 0.0, 1.0);
    let sky_level = clamp(in.uv_b.x, 0.0, 1.0);
    let block_level = clamp(in.uv_b.y, 0.0, 1.0);
    let sky_light = pow(sky_level, LIGHT_GAMMA) * terrain_material_extension.sky_light_factor;
    let block_light = pow(block_level, LIGHT_GAMMA);
    let propagated_light = max(sky_light, block_light);
    let local_light = mix(AMBIENT_FLOOR, 1.0, propagated_light) * ambient_occlusion;
    let maximum_channel = max(texel.r, max(texel.g, texel.b));
    let minimum_channel = min(texel.r, min(texel.g, texel.b));
    let chroma = maximum_channel - minimum_channel;

    var base_rgb = texel.rgb;
    if chroma <= 0.02 {
        let tint_peak = max(max(tint.r, tint.g), max(tint.b, 0.001));
        let hue = tint / tint_peak;
        let softened_hue = mix(vec3<f32>(1.0), hue, 0.72);
        let luminance_weights = vec3<f32>(0.2126, 0.7152, 0.0722);
        let softened_luma = max(dot(softened_hue, luminance_weights), 0.001);
        let luminance_compensation = min(1.35, 1.0 / softened_luma);

        base_rgb = clamp(
            vec3<f32>(texel.r)
                * softened_hue
                * luminance_compensation
                * 1.08,
            vec3<f32>(0.0),
            vec3<f32>(1.0)
        );
    }

    pbr_input.material.base_color = vec4<f32>(
        base_rgb * local_light * pbr_bindings::material.base_color.rgb,
        texel.a * pbr_bindings::material.base_color.a,
    );
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material,
        pbr_input.material.base_color,
    );

#ifdef PREPASS_PIPELINE
    return deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = pbr_input.material.base_color;
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
#endif
}
