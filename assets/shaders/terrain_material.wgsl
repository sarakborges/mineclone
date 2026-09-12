#import bevy_pbr::{
    pbr_bindings,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    pbr_types::{
        STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND,
        STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS,
        STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    },
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
    _padding: vec2<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> terrain_material_extension: TerrainMaterialExtension;

const AMBIENT_FLOOR: f32 = 0.055;
const LIGHT_GAMMA: f32 = 1.35;
const SUN_AMBIENT_SHARE: f32 = 0.38;
const DYNAMIC_LIGHT_SCALE: f32 = 0.08;
const PACKED_RGB_MAX: f32 = 16777215.0;
const DYED_TRANSPARENT_ALPHA: f32 = 0.20;

fn unpack_rgb(value: f32) -> vec3<f32> {
    let packed = round(clamp(value, 0.0, 1.0) * PACKED_RGB_MAX);
    let red = floor(packed / 65536.0);
    let remainder = packed - red * 65536.0;
    let green = floor(remainder / 256.0);
    let blue = remainder - green * 256.0;

    return vec3<f32>(red, green, blue) / 255.0;
}

fn srgb_channel_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        return value / 12.92;
    }

    return pow((value + 0.055) / 1.055, 2.4);
}

fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        srgb_channel_to_linear(color.r),
        srgb_channel_to_linear(color.g),
        srgb_channel_to_linear(color.b),
    );
}

fn apply_authored_tint(
    texel: vec4<f32>,
    tint_srgb: vec3<f32>,
    reveal_transparent: bool,
) -> vec4<f32> {
    let tint_delta = max(
        abs(1.0 - tint_srgb.r),
        max(abs(1.0 - tint_srgb.g), abs(1.0 - tint_srgb.b)),
    );

    if tint_delta <= 0.001 {
        return texel;
    }

    let tint = srgb_to_linear(tint_srgb);
    let luminance_weights = vec3<f32>(0.2126, 0.7152, 0.0722);
    let luminance = dot(texel.rgb, luminance_weights);
    let texture_max = max(max(texel.r, texel.g), texel.b);
    let texture_min = min(min(texel.r, texel.g), texel.b);
    let texture_chroma = texture_max - texture_min;
    let grayscale_mask = 1.0 - smoothstep(0.006, 0.020, texture_chroma);
    let transparent_mask = 1.0 - smoothstep(0.985, 0.999, texel.a);
    let tint_mask = max(grayscale_mask, transparent_mask);
    let shade = mix(0.55, 1.15, clamp(luminance, 0.0, 1.0));
    let tinted = clamp(tint * shade, vec3<f32>(0.0), vec3<f32>(1.0));
    let rgb = mix(texel.rgb, tinted, tint_mask);

    var alpha = texel.a;
    if reveal_transparent {
        alpha = max(alpha, DYED_TRANSPARENT_ALPHA * transparent_mask);
    }

    return vec4<f32>(rgb, alpha);
}

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

fn animate_fluid_color(color: vec3<f32>, world_position: vec3<f32>) -> vec3<f32> {
    let amount = clamp(
        terrain_material_extension.fluid_animation_factor,
        0.0,
        1.0,
    );

    if amount <= 0.0 {
        return color;
    }

    let time = view_bindings::globals.time;
    let broad_wave = sin(
        world_position.x * 0.30
            + world_position.z * 0.22
            + time * 1.15
    );
    let cross_wave = cos(
        world_position.z * 0.37
            - world_position.x * 0.16
            + time * 0.87
    );
    let fine_wave = sin(
        (world_position.x + world_position.z) * 1.08
            + time * 1.43
    );
    let variation = (
        broad_wave * 0.022
            + cross_wave * 0.016
            + fine_wave * 0.008
    ) * amount;

    return clamp(
        color * (1.0 + variation),
        vec3<f32>(0.0),
        vec3<f32>(1.0),
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
    let tint_srgb = unpack_rgb(in.uv_b.y);
    let material_base_color = pbr_bindings::material.base_color;
    let alpha_mode = pbr_input.material.flags
        & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    let is_alpha_blend = alpha_mode == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND;
    let tinted_texel = apply_authored_tint(texel, tint_srgb, is_alpha_blend);
    var material_rgb = tinted_texel.rgb * material_base_color.rgb;

#ifndef PREPASS_PIPELINE
    material_rgb = animate_fluid_color(material_rgb, in.world_position.xyz);
#endif

    let is_unlit = (
        pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT
    ) != 0u;
    let sky_rgb = unpack_rgb(in.uv_b.x);
    let block_rgb = clamp(
        in.color.rgb,
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
    let ambient_occlusion = clamp(in.color.a, 0.0, 1.0);
    let sky_light = pow(
        clamp(sky_rgb, vec3<f32>(0.0), vec3<f32>(1.0)),
        vec3<f32>(LIGHT_GAMMA),
    ) * clamp(terrain_material_extension.sky_light_factor, 0.0, 1.0);
    let block_light = pow(
        block_rgb,
        vec3<f32>(LIGHT_GAMMA),
    );

    var sun_visibility = 1.0;
    var dynamic_light = vec3<f32>(0.0);

#ifndef PREPASS_PIPELINE
    if !is_unlit {
        let surface_normal = normalize(pbr_input.world_normal);
        sun_visibility = directional_sun_visibility(in);
        dynamic_light = dynamic_point_lighting(
            in,
            surface_normal,
            pbr_input.is_orthographic,
        );
    }
#endif

    let shadowed_sky_light = sky_light * sun_visibility;
    let propagated_light = max(shadowed_sky_light, block_light);
    let local_light = mix(
        vec3<f32>(AMBIENT_FLOOR),
        vec3<f32>(1.0),
        propagated_light,
    ) * ambient_occlusion;

    var lighting_multiplier = vec3<f32>(1.0);
    if !is_unlit {
        lighting_multiplier = local_light + dynamic_light;
    }

    pbr_input.material.base_color = vec4<f32>(
        material_rgb * lighting_multiplier,
        tinted_texel.a * material_base_color.a,
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
