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
    clustered_forward as clustering,
    forward_io::{VertexOutput, FragmentOutput},
    lighting,
    mesh_view_bindings as view_bindings,
    mesh_view_types,
    pbr_functions::main_pass_post_lighting_processing,
    shadows,
    view_transformations,
}
#endif

struct TerrainMaterialExtension {
    sky_light_factor: f32,
    fluid_animation_factor: f32,
    fog_color: vec4<f32>,
    fog_distances: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> terrain_material_extension: TerrainMaterialExtension;

const AMBIENT_FLOOR: f32 = 0.055;
const LIGHT_GAMMA: f32 = 1.35;
const SUN_AMBIENT_SHARE: f32 = 0.38;
const DYNAMIC_LIGHT_SCALE: f32 = 0.08;
const TINTED_TRANSPARENCY_ALPHA_FLOOR: f32 = 0.18;

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

fn dynamic_point_lighting(
    in: VertexOutput,
    surface_normal: vec3<f32>,
    is_orthographic: bool,
) -> vec3<f32> {
    let view_z = dot(
        vec4<f32>(
            view_bindings::view.view_from_world[0].z,
            view_bindings::view.view_from_world[1].z,
            view_bindings::view.view_from_world[2].z,
            view_bindings::view.view_from_world[3].z,
        ),
        in.world_position,
    );
    let cluster_index = clustering::view_fragment_cluster_index(
        in.position.xy,
        view_z,
        is_orthographic,
    );
    let ranges = clustering::unpack_clusterable_object_index_ranges(cluster_index);
    var result = vec3<f32>(0.0);

    for (
        var index = ranges.first_point_light_index_offset;
        index < ranges.first_spot_light_index_offset;
        index = index + 1u
    ) {
        let light_id = clustering::get_clusterable_object_id(index);
        let light = &view_bindings::clustered_lights.data[light_id];
        let to_light = (*light).position_radius.xyz - in.world_position.xyz;
        let distance_squared = max(dot(to_light, to_light), 0.0001);
        let light_direction = to_light * inverseSqrt(distance_squared);
        let incidence = max(dot(surface_normal, light_direction), 0.0);

        if incidence <= 0.0 {
            continue;
        }

        let attenuation = lighting::getDistanceAttenuation(
            distance_squared,
            (*light).color_inverse_square_range.w,
        );
        var visibility = 1.0;
        let casts_shadows = ((*light).flags
            & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u;

        if casts_shadows {
            visibility = shadows::fetch_point_shadow(
                light_id,
                in.world_position,
                surface_normal,
                in.position.xy,
            );
        }

        result += (*light).color_inverse_square_range.rgb
            * attenuation
            * incidence
            * visibility
            * DYNAMIC_LIGHT_SCALE;
    }

    return result;
}

fn apply_asteria_distance_fog(
    color: vec4<f32>,
    world_position: vec3<f32>,
) -> vec4<f32> {
    let start = terrain_material_extension.fog_distances.x;
    let end = max(terrain_material_extension.fog_distances.y, start + 0.001);
    let view_distance = distance(world_position, view_bindings::view.world_position.xyz);
    let fog_amount = smoothstep(start, end, view_distance)
        * clamp(terrain_material_extension.fog_color.a, 0.0, 1.0);

    return vec4<f32>(
        mix(color.rgb, terrain_material_extension.fog_color.rgb, fog_amount),
        color.a,
    );
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
    let dynamic_light = vec3<f32>(0.0);
#else
    let surface_normal = normalize(pbr_input.world_normal);
    let sun_visibility = directional_sun_visibility(in);
    let dynamic_light = dynamic_point_lighting(
        in,
        surface_normal,
        pbr_input.is_orthographic,
    );
#endif

    let shadowed_sky_light = sky_light * sun_visibility;
    let propagated_light = max(shadowed_sky_light, block_light);
    let local_light = mix(AMBIENT_FLOOR, 1.0, propagated_light) * ambient_occlusion;

    var base_rgb = texel.rgb;
    let tint_delta = max(
        abs(1.0 - tint.r),
        max(abs(1.0 - tint.g), abs(1.0 - tint.b)),
    );
    if tint_delta > 0.001 {
        let tint_peak = max(max(tint.r, tint.g), max(tint.b, 0.001));
        let hue = tint / tint_peak;
        let softened_hue = mix(vec3<f32>(1.0), hue, 0.82);
        let luminance_weights = vec3<f32>(0.2126, 0.7152, 0.0722);
        let softened_luma = max(dot(softened_hue, luminance_weights), 0.001);
        let luminance_compensation = min(1.25, 1.0 / softened_luma);

        base_rgb = clamp(
            texel.rgb * softened_hue * luminance_compensation,
            vec3<f32>(0.0),
            vec3<f32>(1.0)
        );
    }

    var material_rgb = base_rgb * pbr_bindings::material.base_color.rgb;

#ifndef PREPASS_PIPELINE
    let fluid_animation = clamp(
        terrain_material_extension.fluid_animation_factor,
        0.0,
        1.0,
    );
    let time = view_bindings::globals.time;
    let wave_a = sin(
        in.world_position.x * 0.34
            + in.world_position.z * 0.22
            + time * 1.35
    );
    let wave_b = cos(
        in.world_position.z * 0.41
            - in.world_position.x * 0.17
            + time * 0.92
    );
    let moving_wave = (wave_a * 0.65 + wave_b * 0.35) * fluid_animation;
    let ripple_a = sin(
        (in.world_position.x + in.world_position.z) * 0.86
            + time * 1.72
    );
    let ripple_b = cos(
        (in.world_position.x - in.world_position.z) * 1.18
            - time * 1.06
    );
    let fine_wave = sin(
        in.world_position.x * 2.08
            + in.world_position.z * 1.71
            + time * 0.58
    );
    let interference = abs(ripple_a * 0.58 + ripple_b * 0.42);
    let ripple_ridge = smoothstep(0.48, 0.92, interference) * fluid_animation;
    let fluid_variation =
        moving_wave * 0.050
        + (ripple_a * 0.018 + ripple_b * 0.014 + fine_wave * 0.010) * fluid_animation;

    material_rgb = clamp(
        material_rgb * (1.0 + fluid_variation)
            + vec3<f32>(0.018, 0.028, 0.042) * ripple_ridge,
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
#endif

    let lighting_multiplier = vec3<f32>(local_light) + dynamic_light;
    var surface_alpha = texel.a * pbr_bindings::material.base_color.a;
    if tint_delta > 0.001 {
        surface_alpha = max(
            surface_alpha,
            TINTED_TRANSPARENCY_ALPHA_FLOOR * clamp(tint_delta, 0.0, 1.0),
        );
    }
    pbr_input.material.base_color = vec4<f32>(
        material_rgb * lighting_multiplier,
        surface_alpha,
    );
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material,
        pbr_input.material.base_color,
    );

#ifdef PREPASS_PIPELINE
    return deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_asteria_distance_fog(
        pbr_input.material.base_color,
        in.world_position.xyz,
    );
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
#endif
}
