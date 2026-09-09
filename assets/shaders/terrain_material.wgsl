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
    mesh_view_bindings as view_bindings,
    mesh_view_types,
    pbr_functions::main_pass_post_lighting_processing,
    shadows,
    view_transformations,
}
#endif

struct TerrainMaterialExtension {
    sky_light_factor: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> terrain_material_extension: TerrainMaterialExtension;

const AMBIENT_FLOOR: f32 = 0.055;
const LIGHT_GAMMA: f32 = 1.35;
const SUN_AMBIENT_SHARE: f32 = 0.62;

#ifndef PREPASS_PIPELINE
fn directional_sun_visibility(in: VertexOutput) -> f32 {
    let view_z = view_transformations::position_world_to_view(in.world_position.xyz).z;
    let surface_normal = normalize(in.world_normal);
    let directional_light_count = view_bindings::lights.n_directional_lights;

    for (var light_id: u32 = 0u; light_id < directional_light_count; light_id = light_id + 1u) {
        let light = &view_bindings::lights.directional_lights[light_id];
        let casts_shadows = ((*light).flags
            & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u;

        if !casts_shadows {
            continue;
        }

        let shadow = shadows::fetch_directional_shadow(
            light_id,
            in.world_position,
            surface_normal,
            view_z,
            in.position.xy,
        );
        let incidence = max(
            dot(surface_normal, normalize((*light).direction_to_light)),
            0.0,
        );

        return mix(SUN_AMBIENT_SHARE, 1.0, shadow * incidence);
    }

    return 1.0;
}
#endif

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

#ifdef PREPASS_PIPELINE
    let sun_visibility = 1.0;
#else
    let sun_visibility = directional_sun_visibility(in);
#endif

    let shadowed_sky_light = sky_light * sun_visibility;
    let propagated_light = max(shadowed_sky_light, block_light);
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
